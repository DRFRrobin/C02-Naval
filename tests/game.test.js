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
    const rel = req.url.split('?')[0] === '/' ? 'index.html' : decodeURIComponent(req.url.split('?')[0].slice(1));
    const full = path.resolve(ROOT, rel);
    if (!full.startsWith(ROOT + path.sep) || rel.startsWith('node_modules') || !fs.existsSync(full) || !fs.statSync(full).isFile()) { res.writeHead(404); return res.end(); }
    const types = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript', '.png': 'image/png', '.svg': 'image/svg+xml', '.webmanifest': 'application/manifest+json' };
    res.writeHead(200, { 'content-type': types[path.extname(full)] || 'application/octet-stream' });
    res.end(fs.readFileSync(full));
  });
  await new Promise(r => server.listen(0, '127.0.0.1', r));
  base = `http://127.0.0.1:${server.address().port}/`;
  browser = await chromium.launch();
});
after(async () => { await browser.close(); server.close(); });

let pseudoN = 0;
async function newPage(ctx, viewport, pseudo) {
  const pg = await ctx.newPage();
  if (viewport) await pg.setViewportSize(viewport);
  const errors = [];
  pg.errors = errors;
  pg.on('pageerror', e => errors.push(e.message));
  pg.on('dialog', d => d.accept());
  await pg.goto(base);
  if (!(await pg.$('#auth.active'))) await pg.click('#btnLogout');   // le profil du premier onglet est repris : on change de pseudo
  await pg.fill('#suName', pseudo || 'Joueur' + (++pseudoN));
  await pg.click('#btnPlay');
  return pg;
}
async function newContext() {
  const ctx = await browser.newContext({ serviceWorkers: 'block' });   // le service worker a ses propres tests
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

/* ---------------- Profil dans le navigateur (pseudo seul) ---------------- */
test('connexion : un pseudo suffit, sans mot de passe', async () => {
  const ctx = await newContext(); const pg = await ctx.newPage();
  const errs = []; pg.on('pageerror', e => errs.push(e.message));
  await pg.goto(base);
  assert.equal(await pg.locator('#auth input[type=password]').count(), 0, 'aucun champ mot de passe sur l\'écran de connexion');
  await pg.fill('#suName', 'A');
  await pg.click('#btnPlay');
  assert.ok(await pg.$('#auth.active'), 'pseudo trop court refusé');
  assert.match(await pg.textContent('#authMsg'), /\S/);
  await pg.fill('#suName', 'Capitaine');
  await pg.press('#suName', 'Enter');
  await pg.waitForSelector('#dash.active');
  assert.equal(await pg.textContent('#tbName'), 'Capitaine');
  // le profil survit au rechargement (stocké dans le navigateur)
  await pg.reload();
  await pg.waitForSelector('#dash.active');
  assert.equal(await pg.textContent('#tbName'), 'Capitaine');
  assert.deepEqual(errs, []);
  await ctx.close();
});

test('connexion : retrouver un pseudo existant reprend son XP et ses statistiques', async () => {
  const ctx = await newContext(); const pg = await newPage(ctx, undefined, 'Amiral');
  await pg.evaluate(() => { const a = me(); a.xp = 250; a.stats = { ...emptyStats(), games: 3, wins: 2 }; saveAccount(a); });
  await pg.click('#btnLogout');
  assert.ok(await pg.$('#auth.active'));
  assert.equal(await pg.locator('#accPick .acc').count(), 1, 'le pseudo de cet appareil est proposé');
  await pg.fill('#suName', 'amiral');                                  // casse différente : même profil
  await pg.click('#btnPlay');
  await pg.waitForSelector('#dash.active');
  const r = await pg.evaluate(() => ({ xp: me().xp, games: me().stats.games, n: Object.keys(accounts).length }));
  assert.deepEqual(r, { xp: 250, games: 3, n: 1 });
  await ctx.close();
});

test('une partie contre le bot enregistre l\'XP dans le navigateur', async () => {
  const ctx = await newContext(); const pg = await newPage(ctx, undefined, 'Moussaillon');
  const before = await pg.evaluate(() => me().xp);
  const gain = await pg.evaluate(() => { S.match = { shots: 10, hits: 6, sunk: 2, upgrades: 0, start: Date.now() - 60000 }; S.vsBot = true; return recordGame(true).gain; });
  assert.ok(gain > 0);
  await pg.reload(); await pg.waitForSelector('#dash.active');
  assert.equal(await pg.evaluate(() => me().xp), before + gain);
  await ctx.close();
});

/* ---------------- Application installable (écran d'accueil) ---------------- */
test('PWA : manifeste, icônes et balises iPhone présents', async () => {
  const ctx = await newContext(); const pg = await ctx.newPage();
  await pg.goto(base);
  const head = await pg.evaluate(() => ({
    manifest: document.querySelector('link[rel=manifest]')?.getAttribute('href'),
    touch: document.querySelector('link[rel=apple-touch-icon]')?.getAttribute('href'),
    capable: document.querySelector('meta[name=apple-mobile-web-app-capable]')?.content,
    title: document.querySelector('meta[name=apple-mobile-web-app-title]')?.content,
    bar: document.querySelector('meta[name=apple-mobile-web-app-status-bar-style]')?.content,
    viewport: document.querySelector('meta[name=viewport]')?.content,
  }));
  assert.equal(head.manifest, 'manifest.webmanifest'); assert.equal(head.capable, 'yes');
  assert.equal(head.title, 'C02 Naval'); assert.equal(head.bar, 'black-translucent');
  assert.match(head.viewport, /viewport-fit=cover/);
  const man = await (await ctx.request.get(base + head.manifest)).json();
  assert.equal(man.display, 'standalone'); assert.ok(man.name && man.start_url && man.background_color);
  assert.ok(man.icons.some(i => i.sizes === '192x192') && man.icons.some(i => i.sizes === '512x512'));
  assert.ok(man.icons.some(i => i.purpose === 'maskable'), 'icône adaptative Android');
  for (const src of [...man.icons.map(i => i.src), head.touch]) {
    const r = await ctx.request.get(base + src);
    assert.equal(r.status(), 200, src); assert.equal(r.headers()['content-type'], 'image/png', src);
  }
  const png = await (await ctx.request.get(base + head.touch)).body();
  assert.equal(png.readUInt32BE(16), 180); assert.equal(png.readUInt32BE(20), 180);   // l'icône iPhone fait 180×180
  await ctx.close();
});

test('iPhone : le bandeau et le mode d\'emploi « Sur l\'écran d\'accueil » s\'affichent', async () => {
  const ctx = await browser.newContext({ locale: 'fr-FR', serviceWorkers: 'block', userAgent: 'Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1', viewport: { width: 390, height: 844 }, hasTouch: true });
  await ctx.route(/unpkg|googleapis|gstatic|cdnjs/, r => r.abort());
  const pg = await newPage(ctx);
  assert.ok(await pg.isVisible('#installTip'), 'bandeau visible sur iPhone');
  await pg.click('#btnInstTip');                                       // « Comment ? » ouvre les réglages
  assert.equal(await pg.locator('#instBody li').count(), 4);
  assert.match(await pg.textContent('#instBody'), /écran d'accueil/);
  await pg.click('.tabsnav [data-pane=play]');
  await pg.click('#btnInstTipX');                                      // fermé : ne revient pas
  await pg.reload(); await pg.waitForSelector('#dash.active');
  assert.ok(!(await pg.isVisible('#installTip')));
  assert.equal(await pg.evaluate(() => document.documentElement.scrollWidth - innerWidth) <= 1, true);
  await ctx.close();
});

test('application déjà installée : pas de bandeau, message de confirmation', async () => {
  const ctx = await newContext();
  await ctx.addInitScript(() => { try { localStorage.setItem('c02naval.v1.settings', JSON.stringify({ lang: 'fr' })); } catch (e) {} });
  await ctx.addInitScript(() => { Object.defineProperty(navigator, 'standalone', { value: true }); });
  const pg = await newPage(ctx);
  assert.ok(!(await pg.isVisible('#installTip')));
  await pg.click('.tabsnav [data-pane=settings]');
  assert.match(await pg.textContent('#instBody'), /installée/);
  await ctx.close();
});

test('service worker : le jeu s\'ouvre hors ligne après une première visite', async () => {
  const ctx = await browser.newContext();                              // service worker autorisé
  await ctx.route(/unpkg|googleapis|gstatic|cdnjs/, r => r.abort());
  const pg = await ctx.newPage();
  await pg.goto(base);
  await pg.evaluate(() => navigator.serviceWorker.ready);
  await pg.waitForFunction(() => navigator.serviceWorker.controller || true);
  await pg.reload();                                                   // la page est maintenant contrôlée
  await pg.waitForFunction(() => !!navigator.serviceWorker.controller, null, { timeout: 5000 });
  await ctx.setOffline(true);
  await pg.reload();
  assert.equal(await pg.title(), 'C02 Naval');
  await pg.fill('#suName', 'HorsLigne'); await pg.click('#btnPlay');
  await pg.waitForSelector('#dash.active');
  await ctx.setOffline(false);
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
