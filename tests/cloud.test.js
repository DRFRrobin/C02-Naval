/* Tests du mode en ligne (comptes Google / Discord + relais Realtime), avec un faux Supabase.
   La logique SQL réelle est vérifiée séparément sur PostgreSQL (voir supabase/schema.sql). */
const { test, before, after } = require('node:test');
const assert = require('node:assert/strict');
const http = require('node:http');
const fs = require('node:fs');
const path = require('node:path');
const { chromium } = require('playwright');
const { FakeBackend } = require('./fake-backend');

const ROOT = path.join(__dirname, '..');
let server, base, browser;

before(async () => {
  server = http.createServer((req, res) => {
    if (req.url.split('?')[0] !== '/') { res.writeHead(404); return res.end(); }
    res.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
    res.end(fs.readFileSync(path.join(ROOT, 'index.html')));
  });
  await new Promise(r => server.listen(0, '127.0.0.1', r));
  base = `http://127.0.0.1:${server.address().port}/`;
  browser = await chromium.launch();
});
after(async () => { await browser.close(); server.close(); });

async function ctxFor(backend, init) {
  const ctx = await browser.newContext();
  await ctx.route(/unpkg|googleapis|gstatic|cdnjs|jsdelivr/, r => r.abort());
  await ctx.addInitScript({ path: path.join(__dirname, 'fake-supabase.js') });
  if (init) await ctx.addInitScript(init);
  await backend.attach(ctx);
  return ctx;
}
async function open(ctx, query = '') {
  const pg = await ctx.newPage();
  pg.errors = [];
  pg.on('pageerror', e => pg.errors.push(e.message));
  pg.on('dialog', d => d.accept());
  await pg.goto(base + query);
  return pg;
}
async function guest(ctx) { const pg = await open(ctx); await pg.click('#btnGuest'); return pg; }
async function login(pg, uid, provider = 'google') {
  await pg.evaluate(u => { window.__oauthUser = u; }, uid);
  await pg.click(provider === 'google' ? '#btnGoogle' : '#btnDiscord');
}
async function createProfile(pg, name) {
  await pg.waitForSelector('#onboard.active');
  await pg.fill('#obName', name);
  await pg.click('#obEmojis button:nth-child(5)');
  await pg.click('#btnCreateProfile');
}
const startAuto = pg => pg.evaluate(() => {
  window.__auto = setInterval(() => {
    if (S.choices > 0) { S.inv.cross++; S.choices--; $('choice').classList.remove('show'); }
    if (S.phase === 'game' && S.myTurn && !S.waiting) {
      let k; do { k = Math.floor(Math.random() * N * N); } while (!targetable(k));
      onEnemyTap(Math.floor(k / N), k % N);
    }
  }, 30);
});
const ready = async pg => { await pg.click('#btnRandom'); await pg.click('#btnReady'); };
const phase = (pg, ph, timeout = 15000) => pg.waitForFunction(p => S.phase === p, ph, { timeout });
const watchRecon = pg => pg.evaluate(() => { window.__recon = 0; const orig = connDown; connDown = function () { if (!S.reconnecting) window.__recon++; return orig(); }; });
const bothOver = async (A, B, t = 90000) => { await phase(A, 'over', t); await phase(B, 'over', t); };
async function assertConsistentEnd(A, B) {
  const r = await Promise.all([A, B].map(p => p.evaluate(() => ({ lost: S.lost, title: $('endTitle').textContent }))));
  assert.equal(r[0].lost, false); assert.equal(r[1].lost, false);
  assert.equal(r.filter(x => /Victoire|Victory/.test(x.title)).length, 1, `un seul vainqueur : ${r[0].title} / ${r[1].title}`);
}
async function room(backend) {
  const ctxA = await ctxFor(backend), ctxB = await ctxFor(backend);
  const A = await guest(ctxA), B = await guest(ctxB);
  return { ctxA, ctxB, A, B };
}
async function hostAndJoin(A, B, rules) {
  if (rules) await A.evaluate(r => { RULES = sanitizeRules(r); }, rules);
  await A.click('#mHost');
  await A.waitForFunction(() => /waiting|attente/i.test($('lobbyMsg').textContent));
  const code = await A.textContent('#codeBox');
  await B.click('#mJoin'); await B.fill('#codeInput', code); await B.click('#btnJoin');
  await phase(A, 'place'); await phase(B, 'place');
  return code;
}

/* ---------------- Comptes ---------------- */
test('en ligne : connexion Google / Discord, aucun mot de passe', async () => {
  const be = new FakeBackend(); const ctx = await ctxFor(be); const pg = await open(ctx);
  assert.equal(await pg.isVisible('#btnGoogle'), true);
  assert.equal(await pg.isVisible('#btnDiscord'), true);
  assert.equal(await pg.isVisible('#loginPass'), false);
  assert.equal(await pg.isVisible('#suPass'), false);
  assert.deepEqual(pg.errors, []);
  await ctx.close();
});

test('en ligne : première connexion → création du profil → session conservée', async () => {
  const be = new FakeBackend(); const ctx = await ctxFor(be); const pg = await open(ctx);
  await login(pg, 'u-A');
  await createProfile(pg, 'Capitaine');
  await pg.waitForSelector('#dash.active');
  assert.equal(await pg.textContent('#tbName'), 'Capitaine');
  assert.equal(be.profiles.get('u-A').name, 'Capitaine');
  assert.ok(be.profiles.get('u-A').emoji);
  await pg.reload(); await pg.waitForSelector('#dash.active');          // session conservée : pas de nouvelle création
  assert.equal(await pg.textContent('#tbName'), 'Capitaine');
  assert.deepEqual(pg.errors, []);
  await ctx.close();
});

test('en ligne : pseudo déjà pris, puis déconnexion', async () => {
  const be = new FakeBackend();
  const c1 = await ctxFor(be), c2 = await ctxFor(be);
  const p1 = await open(c1); await login(p1, 'u-A'); await createProfile(p1, 'Capitaine'); await p1.waitForSelector('#dash.active');
  const p2 = await open(c2); await login(p2, 'u-B', 'discord'); await createProfile(p2, 'capitaine');
  await p2.waitForFunction(() => /pris|taken/i.test($('obMsg').textContent));
  await p2.fill('#obName', 'Matelot'); await p2.click('#btnCreateProfile'); await p2.waitForSelector('#dash.active');
  assert.equal(await p2.textContent('#tbName'), 'Matelot');
  await p2.click('#btnLogout'); await p2.waitForSelector('#auth.active');
  await p2.reload(); await p2.waitForSelector('#auth.active');           // bien déconnecté
  await c1.close(); await c2.close();
});

test('en ligne : XP et statistiques enregistrées côté serveur après une partie', async () => {
  const be = new FakeBackend(); const ctx = await ctxFor(be); const pg = await open(ctx);
  await login(pg, 'u-A'); await createProfile(pg, 'Capitaine'); await pg.waitForSelector('#dash.active');
  await pg.evaluate(() => { BOT_SPEED = 0.03; });
  await pg.click('#mBot'); await ready(pg); await phase(pg, 'game', 8000); await startAuto(pg); await phase(pg, 'over', 60000);
  await pg.waitForFunction(() => /\+\d+ XP/.test($('endXp').textContent), null, { timeout: 5000 });
  const p = be.profiles.get('u-A');
  assert.equal(p.games, 1); assert.ok(p.xp > 0); assert.equal(p.wins + p.losses, 1);
  assert.ok(p.shots > 0);
  assert.equal(await pg.evaluate(() => me().xp), p.xp);                  // l'affichage reprend la valeur du serveur
  assert.deepEqual(pg.errors, []);
  await ctx.close();
});

test('en ligne : import des statistiques d\'un ancien compte local', async () => {
  const be = new FakeBackend();
  const ctx = await ctxFor(be, () => {
    if (!localStorage.getItem('c02naval.v1.acc.old1')) localStorage.setItem('c02naval.v1.acc.old1', JSON.stringify({ id: 'old1', name: 'Ancien', xp: 700,
      stats: { games: 12, wins: 7, losses: 5, shots: 200, hits: 90, sunk: 20, upgrades: 3, bestStreak: 4, fastestWin: 31, timePlayed: 900, interrupted: 1, streak: 0 } }));
  });
  const pg = await open(ctx); await login(pg, 'u-A');
  await pg.waitForSelector('#onboard.active');
  assert.equal(await pg.isVisible('#obImport'), true);
  await pg.selectOption('#obImport', 'old1');
  await pg.fill('#obName', 'Veteran'); await pg.click('#btnCreateProfile'); await pg.waitForSelector('#dash.active');
  const p = be.profiles.get('u-A');
  assert.deepEqual([p.xp, p.games, p.wins, p.best_streak, p.fastest_win], [700, 12, 7, 4, 31]);
  await ctx.close();
});

test('en ligne : suppression du compte', async () => {
  const be = new FakeBackend(); const ctx = await ctxFor(be); const pg = await open(ctx);
  await login(pg, 'u-A'); await createProfile(pg, 'Capitaine'); await pg.waitForSelector('#dash.active');
  await pg.evaluate(() => { pane = 'settings'; renderDash(); });
  assert.equal(await pg.isVisible('#editPass'), false);                  // plus de mot de passe
  await pg.click('#btnDelete'); await pg.waitForSelector('#auth.active');
  assert.equal(be.profiles.size, 0);
  await ctx.close();
});

test('en ligne : le mode invité reste possible (sans XP enregistrée)', async () => {
  const be = new FakeBackend(); const ctx = await ctxFor(be); const pg = await guest(ctx);
  await pg.evaluate(() => { BOT_SPEED = 0.03; });
  await pg.click('#mBot'); await ready(pg); await phase(pg, 'game', 8000); await startAuto(pg); await phase(pg, 'over', 60000);
  assert.ok(!be.calls.includes('record_game'));
  await ctx.close();
});

/* ---------------- Relais Realtime ---------------- */
test('relais : salon privé, règles de l\'hôte, partie complète', async () => {
  const be = new FakeBackend(); const { ctxA, ctxB, A, B } = await room(be);
  await hostAndJoin(A, B, { size: 8, fleet: [4, 3, 2], radar: false, cross: true, shield: true });
  assert.equal(await B.evaluate(() => [N, LENS.join('-'), CFG.radar].join()), '8,4-3-2,false');
  await ready(A); await ready(B); await phase(A, 'game'); await phase(B, 'game');
  await startAuto(A); await startAuto(B); await bothOver(A, B); await assertConsistentEnd(A, B);
  assert.deepEqual([...A.errors, ...B.errors], []);
  await ctxA.close(); await ctxB.close();
});

test('relais : code inconnu et salon complet', async () => {
  const be = new FakeBackend(); const { ctxA, ctxB, A, B } = await room(be);
  await B.click('#mJoin'); await B.fill('#codeInput', 'ZZZZZZ'); await B.click('#btnJoin');
  await B.waitForFunction(() => /No game|Aucune partie/.test($('joinMsg').textContent), null, { timeout: 8000 });
  await B.click('#btnJoinCancel');
  const code = await hostAndJoin(A, B);
  const ctxC = await ctxFor(be), C = await guest(ctxC);
  await C.click('#mJoin'); await C.fill('#codeInput', code); await C.click('#btnJoin');
  await C.waitForFunction(() => /already full|déjà complète/.test($('joinMsg').textContent), null, { timeout: 8000 });
  assert.equal(await A.evaluate(() => S.phase), 'place');                 // la partie en cours n'est pas perturbée
  await ctxA.close(); await ctxB.close(); await ctxC.close();
});

test('relais : adversaire aléatoire (en ignorant un salon fantôme dans la file)', async () => {
  const be = new FakeBackend();
  be.queue.push({ ticket: 'ghost-ticket', room: 'GHOST1', at: Date.now() });   // un joueur parti sans quitter la file
  const { ctxA, ctxB, A, B } = await room(be);
  await A.click('#mRandom'); await A.waitForFunction(() => S.isHost && S.conn && S.conn.joined, null, { timeout: 15000 });
  await B.click('#mRandom');
  await phase(A, 'place', 20000); await phase(B, 'place', 20000);
  assert.notEqual(await A.evaluate(() => S.isHost), await B.evaluate(() => S.isHost));
  await ready(A); await ready(B); await phase(A, 'game'); await phase(B, 'game');
  await startAuto(A); await startAuto(B); await bothOver(A, B); await assertConsistentEnd(A, B);
  assert.equal(be.queue.length, 0);
  assert.deepEqual([...A.errors, ...B.errors], []);
  await ctxA.close(); await ctxB.close();
});

test('relais : reprise après coupure, messages perdus renvoyés', async () => {
  const be = new FakeBackend(); const { ctxA, ctxB, A, B } = await room(be);
  await hostAndJoin(A, B);
  await ready(A); await ready(B); await phase(A, 'game'); await phase(B, 'game');
  await startAuto(A); await startAuto(B);
  await A.waitForFunction(() => S.match.shots >= 4);
  await watchRecon(A); await watchRecon(B);
  await Promise.all([A, B].map(p => p.evaluate(() => { window.__sbBlackhole = true; })));   // messages perdus
  await A.waitForTimeout(500);
  await A.evaluate(() => window.__sbDrop());                                                // A perd le réseau
  await A.waitForFunction(() => window.__recon > 0, null, { timeout: 8000 });
  await B.waitForFunction(() => window.__recon > 0, null, { timeout: 8000 });
  await Promise.all([A, B].map(p => p.evaluate(() => { window.__sbBlackhole = false; })));
  await A.evaluate(() => window.__sbRestore());                                             // le réseau revient
  await A.waitForFunction(() => !S.reconnecting && !S.lost, null, { timeout: 15000 });
  await B.waitForFunction(() => !S.reconnecting && !S.lost, null, { timeout: 15000 });
  await bothOver(A, B); await assertConsistentEnd(A, B);
  assert.deepEqual([...A.errors, ...B.errors], []);
  await ctxA.close(); await ctxB.close();
});

test('relais : abandon = victoire par forfait', async () => {
  const be = new FakeBackend(); const { ctxA, ctxB, A, B } = await room(be);
  await hostAndJoin(A, B);
  await ready(A); await ready(B); await phase(A, 'game'); await phase(B, 'game');
  await A.evaluate(() => { document.querySelector('#game .btnQuit').click(); });
  await B.waitForFunction(() => S.phase === 'over', null, { timeout: 5000 });
  assert.match(await B.textContent('#endTitle'), /Victoire|Victory/);
  assert.match(await B.textContent('#endText'), /abandonn|left/);
  await ctxA.close(); await ctxB.close();
});

test('relais : un joueur qui disparaît définitivement → « connexion perdue »', async () => {
  const be = new FakeBackend(); const { ctxA, ctxB, A, B } = await room(be);
  await hostAndJoin(A, B);
  await ready(A); await ready(B); await phase(A, 'game'); await phase(B, 'game');
  await B.evaluate(() => window.__sbDrop());
  await A.waitForFunction(() => S.reconnecting, null, { timeout: 8000 });
  await A.evaluate(() => { S.reconDeadline = Date.now(); });
  await A.waitForFunction(() => S.lost, null, { timeout: 8000 });
  await ctxA.close(); await ctxB.close();
});
