/* Tests de bout en bout : logique du jeu, partie contre le bot, multijoueur (PeerJS simulé),
   reprise après coupure, abandon, chronomètre, règles et responsive.
   Lancement : npm test */
const { test, before, after } = require('node:test');
const assert = require('node:assert/strict');
const http = require('node:http');
const fs = require('node:fs');
const path = require('node:path');
const { chromium } = require('playwright');

const ROOT = path.join(__dirname, '..');
let server, base, browser;

before(async () => {
  server = http.createServer((req, res) => {
    const file = req.url.split('?')[0] === '/' ? 'index.html' : null;
    if (!file) { res.writeHead(404); return res.end(); }
    res.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
    res.end(fs.readFileSync(path.join(ROOT, file)));
  });
  await new Promise(r => server.listen(0, '127.0.0.1', r));
  base = `http://127.0.0.1:${server.address().port}/`;
  browser = await chromium.launch();
});
after(async () => { await browser.close(); server.close(); });

async function newPage(ctx, viewport) {
  const pg = await ctx.newPage();
  if (viewport) await pg.setViewportSize(viewport);
  const errors = [];
  pg.errors = errors;
  pg.on('pageerror', e => errors.push(e.message));
  pg.on('dialog', d => d.accept());
  await pg.goto(base);
  await pg.click('#btnGuest');
  return pg;
}
async function newContext() {
  const ctx = await browser.newContext();
  await ctx.route(/unpkg|googleapis|gstatic|cdnjs/, r => r.abort());   // aucun accès réseau : PeerJS est simulé
  await ctx.addInitScript({ path: path.join(__dirname, 'fake-peer.js') });
  return ctx;
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
// Compte les coupures détectées (la reprise pouvant être plus rapide que l'interrogation de la page).
const watchRecon = pg => pg.evaluate(() => { window.__recon = 0; const orig = connDown; connDown = function () { if (!S.reconnecting) window.__recon++; return orig(); }; });
const ready = async pg => { await pg.click('#btnRandom'); await pg.click('#btnReady'); };
const phase = (pg, ph, timeout = 15000) => pg.waitForFunction(p => S.phase === p, ph, { timeout });

/* ---------------- Logique pure ---------------- */
test('règles : validation et normalisation', async () => {
  const ctx = await newContext(); const pg = await newPage(ctx);
  const r = await pg.evaluate(() => ({
    bad: [sanitizeRules(null), sanitizeRules({ size: 99, fleet: [2] }), sanitizeRules({ size: 10, fleet: [] }),
      sanitizeRules({ size: 6, fleet: [6, 6, 6, 6] }), sanitizeRules({ size: 10, fleet: [7] }), sanitizeRules({ size: 10, fleet: Array(11).fill(1) })],
    ok: sanitizeRules({ size: 12, fleet: [2, 5, 3], timer: 30 }),
    timerBad: sanitizeRules({ size: 10, fleet: [3], timer: 7 }).timer,
    cheat1: normalizeAct({ kind: 'cross', at: [3, 3], cells: [[3, 3], [0, 0], [9, 9], [3, 4], [3, 4]] }, false),
    cheat2: normalizeAct({ kind: 'radar', at: [2, 2], cells: [[2, 2], [8, 8]] }, true),
  }));
  assert.ok(r.bad.every(x => x === null));
  assert.deepEqual(r.ok.fleet, [5, 3, 2]); assert.equal(r.ok.timer, 30); assert.equal(r.timerBad, 0);
  assert.deepEqual(r.cheat1, { kind: 'shot', at: [3, 3], cells: [[3, 3]] });
  assert.deepEqual(r.cheat2.cells, [[2, 2]]);
  await ctx.close();
});

test('placement aléatoire réussi pour toutes les tailles de grille', async () => {
  const ctx = await newContext(); const pg = await newPage(ctx);
  const bad = await pg.evaluate(() => {
    let bad = 0;
    for (let sz = MIN_SIZE; sz <= MAX_SIZE; sz++) {
      const fleet = []; for (const l of [6, 5, 5, 4, 4, 3, 3, 2, 2, 1]) if (sumOf(fleet) + l <= maxCells(sz)) fleet.push(l);
      for (let i = 0; i < 100; i++) { const out = randomCells(fleet); if (out.some(c => !c)) bad++; }
    }
    return bad;
  });
  assert.equal(bad, 0);
  await ctx.close();
});

test('avatar : seuls les emojis de la liste sont acceptés', async () => {
  const ctx = await newContext(); const pg = await newPage(ctx);
  const r = await pg.evaluate(() => [validEmoji('🐙'), validEmoji('💣'), validEmoji('<img>'), validEmoji(null), EMOJIS.every(validEmoji)]);
  assert.deepEqual(r, [true, false, false, false, true]);
  await ctx.close();
});

/* ---------------- Bot ---------------- */
for (const skill of [0, 1, 2]) {
  test(`partie complète contre le bot (niveau ${skill})`, async () => {
    const ctx = await newContext(); const pg = await newPage(ctx);
    await pg.evaluate(s => { settings.skill = s; }, skill);
    await pg.evaluate(() => { BOT_SPEED = 0.03; });
    await pg.click('#mBot'); await ready(pg);
    await phase(pg, 'game', 8000); await startAuto(pg);
    await phase(pg, 'over', 60000);
    assert.match(await pg.textContent('#endTitle'), /Victoire|Défaite|Victory|Defeat/);
    assert.deepEqual(pg.errors, []);
    await ctx.close();
  });
}

test('règles personnalisées contre le bot : grille 7×7, sans améliorations', async () => {
  const ctx = await newContext(); const pg = await newPage(ctx);
  await pg.evaluate(() => { BOT_SPEED = 0.03; RULES = sanitizeRules({ size: 7, fleet: [3, 2, 2], radar: false, cross: false, shield: false }); });
  await pg.click('#mBot'); await ready(pg); await phase(pg, 'game', 8000);
  assert.equal(await pg.evaluate(() => [N, LENS.length, $('enemyGrid').cells.length, anyUpgrade()].join()), '7,3,49,false');
  await startAuto(pg); await phase(pg, 'over', 60000);
  assert.equal(await pg.evaluate(() => S.choices), 0);
  assert.deepEqual(pg.errors, []);
  await ctx.close();
});

/* ---------------- Chronomètre ---------------- */
test('chronomètre : tir automatique quand le temps est écoulé', async () => {
  const ctx = await newContext(); const pg = await newPage(ctx);
  await pg.evaluate(() => { RULES = sanitizeRules({ ...DEFAULT_RULES, timer: 15 }); });
  await pg.click('#mBot'); await pg.waitForFunction(() => S.phase === 'place');
  // placement : temps écoulé => flotte complétée et validée automatiquement
  await pg.evaluate(() => { S.placeDeadline = Date.now() - 1; });
  await phase(pg, 'game', 10000);
  const before = await pg.evaluate(() => S.match.shots);
  await pg.evaluate(() => { S.turnStart = Date.now() - 16000; });
  await pg.waitForFunction(b => S.match.shots > b || S.waiting, before, { timeout: 5000 });
  assert.deepEqual(pg.errors, []);
  await ctx.close();
});

/* ---------------- Multijoueur (PeerJS simulé) ---------------- */
async function hostAndJoin(ctx, rules) {
  const A = await newPage(ctx), B = await newPage(ctx);
  if (rules) await A.evaluate(r => { RULES = sanitizeRules(r); }, rules);
  await A.click('#mHost');
  await A.waitForFunction(() => /waiting|attente|Waiting/i.test($('lobbyMsg').textContent) || S.peer?.open);
  const code = await A.textContent('#codeBox');
  await B.click('#mJoin'); await B.fill('#codeInput', code); await B.click('#btnJoin');
  await phase(A, 'place'); await phase(B, 'place');
  return [A, B];
}
const bothOver = async (A, B, t = 120000) => { await phase(A, 'over', t); await phase(B, 'over', t); };
async function assertConsistentEnd(A, B) {
  const [a, b] = await Promise.all([A, B].map(p => p.evaluate(() => ({ lost: S.lost, title: $('endTitle').textContent }))));
  assert.equal(a.lost, false); assert.equal(b.lost, false);
  const wins = [a, b].filter(x => /Victoire|Victory/.test(x.title)).length;
  assert.equal(wins, 1, `un seul vainqueur attendu, reçu : ${a.title} / ${b.title}`);
}

test('salon privé : l\'invité reçoit les règles de l\'hôte et la partie va au bout', async () => {
  const ctx = await newContext();
  const [A, B] = await hostAndJoin(ctx, { size: 8, fleet: [4, 3, 2], radar: false, cross: true, shield: true });
  assert.equal(await B.evaluate(() => [N, LENS.join('-'), CFG.radar].join()), '8,4-3-2,false');
  await ready(A); await ready(B); await phase(A, 'game'); await phase(B, 'game');
  await startAuto(A); await startAuto(B);
  await bothOver(A, B); await assertConsistentEnd(A, B);
  assert.deepEqual([...A.errors, ...B.errors], []);
  await ctx.close();
});

test('reprise après coupure : la partie continue sans perdre de message', async () => {
  const ctx = await newContext();
  const [A, B] = await hostAndJoin(ctx);
  await ready(A); await ready(B); await phase(A, 'game'); await phase(B, 'game');
  await startAuto(A); await startAuto(B);
  await A.waitForFunction(() => S.match.shots >= 4);
  // 1) messages perdus pendant que le réseau est « trou noir », puis coupure
  await watchRecon(A); await watchRecon(B);
  await Promise.all([A, B].map(p => p.evaluate(() => { window.__blackhole = true; })));
  await A.waitForTimeout(400);
  await A.evaluate(() => window.__dropAll());
  await A.waitForFunction(() => window.__recon > 0, null, { timeout: 3000 });
  await Promise.all([A, B].map(p => p.evaluate(() => { window.__blackhole = false; })));
  // 2) le réseau revient : reprise automatique, messages renvoyés
  await A.waitForFunction(() => !S.reconnecting && !S.lost, null, { timeout: 15000 });
  assert.ok(await A.evaluate(() => window.__recon > 0));
  await B.waitForFunction(() => !S.reconnecting && !S.lost, null, { timeout: 15000 });
  await bothOver(A, B); await assertConsistentEnd(A, B);
  assert.deepEqual([...A.errors, ...B.errors], []);
  await ctx.close();
});

test('adversaire aléatoire : appariement puis reprise après coupure', async () => {
  const ctx = await newContext();
  const A = await newPage(ctx), B = await newPage(ctx);
  await A.click('#mRandom'); await A.waitForFunction(() => S.isHost && S.peer && S.peer.open);
  await B.click('#mRandom');
  await phase(A, 'place'); await phase(B, 'place');
  await ready(A); await ready(B); await phase(A, 'game'); await phase(B, 'game');
  await startAuto(A); await startAuto(B);
  await B.waitForFunction(() => S.match.shots >= 3);
  await watchRecon(A); await watchRecon(B);
  await B.evaluate(() => window.__dropAll());
  await B.waitForFunction(() => window.__recon > 0, null, { timeout: 3000 });
  await B.waitForFunction(() => !S.reconnecting && !S.lost, null, { timeout: 20000 });
  await A.waitForFunction(() => !S.reconnecting && !S.lost, null, { timeout: 20000 });
  await bothOver(A, B); await assertConsistentEnd(A, B);
  assert.deepEqual([...A.errors, ...B.errors], []);
  await ctx.close();
});

test('coupure définitive : « connexion perdue » après le délai de reprise', async () => {
  const ctx = await newContext();
  const [A, B] = await hostAndJoin(ctx);
  await ready(A); await ready(B); await phase(A, 'game'); await phase(B, 'game');
  await Promise.all([A, B].map(p => p.evaluate(() => { window.__blackhole = true; })));   // le réseau ne revient jamais
  await A.evaluate(() => window.__dropAll());
  await A.waitForFunction(() => S.reconnecting || S.lost, null, { timeout: 3000 });
  await A.evaluate(() => { S.reconDeadline = Date.now(); });                               // on accélère le délai de 45 s
  await A.waitForFunction(() => S.lost, null, { timeout: 8000 });
  assert.match(await A.textContent('#endTitle'), /perdue|lost/i);
  await ctx.close();
});

test('abandon : l\'adversaire gagne par forfait', async () => {
  const ctx = await newContext();
  const [A, B] = await hostAndJoin(ctx);
  await ready(A); await ready(B); await phase(A, 'game'); await phase(B, 'game');
  await A.evaluate(() => { window.confirm = () => true; document.querySelector('#game .btnQuit').click(); });
  await B.waitForFunction(() => S.phase === 'over', null, { timeout: 5000 });
  assert.match(await B.textContent('#endTitle'), /Victoire|Victory/);
  assert.match(await B.textContent('#endText'), /abandonn|left/);
  await ctx.close();
});

test('un adversaire ne peut pas tricher avec une amélioration non gagnée', async () => {
  const ctx = await newContext();
  const [A, B] = await hostAndJoin(ctx);
  await ready(A); await ready(B); await phase(A, 'game'); await phase(B, 'game');
  // A (hôte) joue en premier : il envoie une « frappe en croix » sans en avoir gagné
  await A.evaluate(() => { S.inv.cross = 1; S.mode = 'cross'; onEnemyTap(5, 5); });
  await B.waitForFunction(() => S.myTurn, null, { timeout: 5000 });
  const hits = await B.evaluate(() => S.incoming.filter(Boolean).length);
  assert.equal(hits, 1, 'seule 1 case doit être touchée');
  await ctx.close();
});

/* ---------------- Interface ---------------- */
test('responsive : aucun débordement horizontal à 320 px (tous thèmes)', async () => {
  const ctx = await newContext(); const pg = await newPage(ctx, { width: 320, height: 640 });
  const issues = [];
  for (const th of ['ocean', 'night', 'forest', 'sunset', 'light']) {
    await pg.evaluate(t => { settings.theme = t; applyTheme(); }, th);
    for (const pane of ['play', 'stats', 'rules', 'settings']) {
      await pg.evaluate(p => { pane = p; renderDash(); }, pane);
      const over = await pg.evaluate(() => document.documentElement.scrollWidth - innerWidth);
      if (over > 1) issues.push(`${th}/${pane}: ${over}px`);
    }
  }
  assert.deepEqual(issues, []);
  await ctx.close();
});
