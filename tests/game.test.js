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
// Code du jeu (index.html + fichiers js/), lu tel quel par quelques tests
const SOURCE = () => ['index.html', ...fs.readdirSync(path.join(ROOT, 'js')).map(f => 'js/' + f)].map(f => fs.readFileSync(path.join(ROOT, f), 'utf8')).join('\n');
let server, base, browser;

before(async () => {
  server = http.createServer((req, res) => {
    const rel = req.url.split('?')[0] === '/' ? 'index.html' : decodeURIComponent(req.url.split('?')[0].slice(1));
    const full = path.resolve(ROOT, rel);
    if (!full.startsWith(ROOT + path.sep) || rel.startsWith('node_modules') || !fs.existsSync(full) || !fs.statSync(full).isFile()) { res.writeHead(404); return res.end(); }
    const types = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript; charset=utf-8', '.css': 'text/css; charset=utf-8', '.png': 'image/png', '.svg': 'image/svg+xml', '.webmanifest': 'application/manifest+json' };
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
  await ctx.addInitScript(() => { window.C02_API = ''; });   // pas de serveur de comptes par défaut : connexion par pseudo local
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
  await A.evaluate(() => { S.inv.cross = 1; S.mode = 'cross'; onEnemyTap(5, 5); onEnemyTap(5, 5); });   // viser puis confirmer
  await B.waitForFunction(() => S.myTurn, null, { timeout: 5000 });
  const hits = await B.evaluate(() => S.incoming.filter(Boolean).length);
  assert.equal(hits, 1, 'seule 1 case doit être touchée');
  await ctx.close();
});

test('anti-triche : partie honnête, les deux grilles sont vérifiées', async () => {
  const ctx = await newContext();
  const [A, B] = await hostAndJoin(ctx);
  await ready(A); await ready(B); await phase(A, 'game'); await phase(B, 'game');
  assert.ok(await A.evaluate(() => /^[0-9a-f]{64}$/.test(S.oppCommit)), 'empreinte de la grille adverse reçue');
  await startAuto(A); await startAuto(B);
  await bothOver(A, B);
  for (const p of [A, B]) await p.waitForFunction(() => S.verify === 'ok', null, { timeout: 8000 });
  const loser = (await A.evaluate(() => S.win)) ? B : A;
  assert.equal(await loser.evaluate(() => me().stats.losses), 1, 'la défaite est comptée');
  assert.match(await loser.textContent('#verifyNote'), /vérifiée|checked/);
  assert.deepEqual([...A.errors, ...B.errors], []);
  await ctx.close();
});

test('anti-triche : un adversaire qui ment sur ses résultats est démasqué, la défaite n\'est pas comptée', async () => {
  const ctx = await newContext();
  const [A, B] = await hostAndJoin(ctx);
  // B est modifié : il annonce « raté » à chaque tir reçu
  await B.evaluate(() => { resolveOn = (ships, incoming, kind, cells) => ({ results: cells.map(([r, c]) => ({ r, c, v: kind === 'radar' ? 'empty' : 'miss' })), over: false }); });
  await ready(A); await ready(B); await phase(A, 'game'); await phase(B, 'game');
  await startAuto(A); await startAuto(B);
  await bothOver(A, B);
  await A.waitForFunction(() => S.verify === 'cheat', null, { timeout: 8000 });
  assert.match(await A.textContent('#endTitle'), /Défaite|Defeat/);
  assert.match(await A.textContent('#verifyNote'), /Triche|Cheating/);
  const st = await A.evaluate(() => me().stats);
  assert.equal(st.losses, 0); assert.equal(st.interrupted, 1);
  await ctx.close();
});

test('Radar et Frappe en croix : un premier appui vise, un second confirme', async () => {
  const ctx = await newContext(); const pg = await newPage(ctx);
  await pg.evaluate(() => { BOT_SPEED = 0.03; });
  await pg.click('#mBot'); await ready(pg); await phase(pg, 'game', 8000);
  await pg.waitForFunction(() => S.myTurn && !S.waiting);
  await pg.evaluate(() => { S.inv.radar = 1; setMode('radar'); onEnemyTap(3, 3); });
  let st = await pg.evaluate(() => ({ waiting: S.waiting, inv: S.inv.radar, aim: $('enemyGrid').querySelectorAll('.cell.aim').length, hint: $('hint').textContent }));
  assert.deepEqual([st.waiting, st.inv, st.aim], [false, 1, 4]);
  assert.match(st.hint, /D4/);
  await pg.evaluate(() => onEnemyTap(6, 6));   // un appui ailleurs déplace la visée
  assert.equal(await pg.evaluate(() => S.aim.join()), '6,6');
  await pg.evaluate(() => onEnemyTap(6, 6));
  st = await pg.evaluate(() => ({ waiting: S.waiting || !S.myTurn, inv: S.inv.radar, aim: S.aim }));
  assert.deepEqual(st, { waiting: true, inv: 0, aim: null });
  assert.deepEqual(pg.errors, []);
  await ctx.close();
});

test('Bouclier : la liste des bateaux permet de le poser sans chercher sa flotte', async () => {
  const ctx = await newContext(); const pg = await newPage(ctx);
  await pg.evaluate(() => { BOT_SPEED = 0.03; });
  await pg.click('#mBot'); await ready(pg); await phase(pg, 'game', 8000);
  await pg.waitForFunction(() => S.myTurn && !S.waiting);
  await pg.evaluate(() => { S.inv.shield = 1; setMode('shield'); });
  assert.equal(await pg.locator('#shieldPick button').count(), 5);
  await pg.locator('#shieldPick button').nth(1).click();
  const r = await pg.evaluate(() => ({ sh: S.ships[1].shield, inv: S.inv.shield, my: S.myTurn, shown: $('shieldPick').classList.contains('show') }));
  assert.deepEqual(r, { sh: 'active', inv: 0, my: false, shown: false });
  assert.deepEqual(pg.errors, []);
  await ctx.close();
});

test('téléphone : les boutons d\'action restent fixés en bas pendant la partie', async () => {
  const ctx = await newContext(); const pg = await newPage(ctx, { width: 390, height: 844 });
  await pg.click('#mBot'); await ready(pg); await phase(pg, 'game', 8000);
  const r = await pg.evaluate(() => { const b = $('actBar').getBoundingClientRect(); return { pos: getComputedStyle($('actBar')).position, bottom: b.bottom, h: innerHeight }; });
  assert.equal(r.pos, 'fixed'); assert.ok(Math.abs(r.bottom - r.h) <= 1, 'collée en bas');
  // tout en bas de la page, la flotte n'est pas cachée par la barre
  await pg.evaluate(() => scrollTo(0, document.body.scrollHeight));
  const clear = await pg.evaluate(() => $('myGrid').getBoundingClientRect().bottom <= $('actBar').getBoundingClientRect().top);
  assert.ok(clear, 'la flotte reste visible au-dessus de la barre');
  const over = await pg.evaluate(() => document.documentElement.scrollWidth - innerWidth);
  assert.ok(over <= 1, 'pas de débordement horizontal');
  await pg.setViewportSize({ width: 1200, height: 800 });
  assert.equal(await pg.evaluate(() => getComputedStyle($('actBar')).position), 'static');
  await ctx.close();
});

/* ---------------- Profil dans le navigateur (pseudo seul) ---------------- */
test('connexion : un pseudo suffit, sans mot de passe', async () => {
  const ctx = await newContext(); const pg = await ctx.newPage();
  const errs = []; pg.on('pageerror', e => errs.push(e.message));
  await pg.goto(base);
  assert.equal(await pg.locator('#auth input[type=password]:visible').count(), 0, 'aucun champ mot de passe visible sans serveur de comptes'.slice(0,0) + 'aucun champ mot de passe sur l\'écran de connexion');
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
  await ctx.addInitScript(() => { window.C02_API = ''; });
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
  await ctx.addInitScript(() => { window.C02_API = ''; });
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

test('iPhone en mode application : l\'interface ne passe pas sous la Dynamic Island', async () => {
  const UA = 'Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1';
  const top = async (viewport, standalone, ua) => {
    const ctx = await browser.newContext({ serviceWorkers: 'block', userAgent: ua, viewport, hasTouch: true });
    await ctx.addInitScript(() => { window.C02_API = ''; });
    await ctx.route(/unpkg|googleapis|gstatic|cdnjs/, r => r.abort());
    if (standalone) await ctx.addInitScript(() => { Object.defineProperty(navigator, 'standalone', { value: true }); });
    const pg = await newPage(ctx);
    const v = await pg.evaluate(() => parseFloat(getComputedStyle(document.getElementById('app')).paddingTop));
    await ctx.close(); return v;
  };
  assert.ok(await top({ width: 393, height: 852 }, true, UA) >= 59, 'iPhone récent (Dynamic Island) : au moins 59 px en haut');
  const small = await top({ width: 375, height: 667 }, true, UA);
  assert.ok(small >= 20 && small < 40, 'iPhone SE : barre d\'état seule, ' + small);
  assert.ok(await top({ width: 393, height: 852 }, false, UA) <= 16, 'dans Safari : espacement normal');
  assert.ok(await top({ width: 393, height: 852 }, true, 'Mozilla/5.0 (Linux; Android 14) Chrome/120 Mobile Safari/537.36') <= 16, 'Android : inchangé');
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

test('téléphone : onglets en bas de l\'écran, profil/XP en haut', async () => {
  const ctx = await newContext(); const pg = await newPage(ctx, { width: 390, height: 844 });
  const r = await pg.evaluate(() => {
    const n = document.querySelector('.tabsnav').getBoundingClientRect();
    const t = document.querySelector('.topbar').getBoundingClientRect();
    const last = document.querySelector('#pane-play').lastElementChild.getBoundingClientRect();
    return { navBottom: n.bottom, navTop: n.top, h: innerHeight, topBottom: t.bottom, lastBottom: last.bottom };
  });
  assert.ok(Math.abs(r.navBottom - r.h) <= 1, 'barre collée en bas');
  assert.ok(r.topBottom < 200, 'profil en haut et compact');
  await pg.evaluate(() => scrollTo(0, document.body.scrollHeight));
  const clear = await pg.evaluate(() => document.querySelector('#pane-play').lastElementChild.getBoundingClientRect().bottom <= document.querySelector('.tabsnav').getBoundingClientRect().top);
  assert.ok(clear, 'le contenu n\'est pas masqué par la barre');
  // Ordinateur : la barre reste dans l'en-tête
  await pg.setViewportSize({ width: 1200, height: 800 });
  const pos = await pg.evaluate(() => getComputedStyle(document.querySelector('.tabsnav')).position);
  assert.equal(pos, 'static');
  await ctx.close();
});

test('sauvegarde chiffrée : export puis import sur un autre appareil', async () => {
  const c1 = await newContext(); const p1 = await newPage(c1, { width: 1000, height: 800 }, 'Capitaine');
  await p1.evaluate(() => { loadAccounts(); const a = me(); a.xp = 450; a.stats.games = 12; a.stats.wins = 7; saveAccount(a); });
  await p1.evaluate(() => { pane = 'settings'; renderDash(); });
  await p1.fill('#bkPass', 'court');
  await p1.click('#btnBkCopy');
  assert.match(await p1.textContent('#bkMsg'), /8 (caract|characters)/);
  await p1.fill('#bkPass', 'phrase secrète solide');
  await p1.click('#btnBkCopy');
  await p1.waitForFunction(() => $('bkCode').value.startsWith('C02S1.'), null, { timeout: 15000 });
  const code = await p1.inputValue('#bkCode');
  assert.ok(!code.includes('Capitaine') && !code.includes('450'), 'rien en clair dans le code');
  // autre appareil = autre navigateur
  const c2 = await newContext(); const p2 = await newPage(c2, { width: 1000, height: 800 }, 'Autre');
  await p2.evaluate(() => { pane = 'settings'; renderDash(); });
  await p2.fill('#bkCode', code);
  await p2.fill('#bkPass', 'mauvaise phrase');
  await p2.click('#btnBkImport');
  await p2.waitForFunction(() => /incorrecte|Wrong/.test($('bkMsg').textContent), null, { timeout: 15000 });
  await p2.fill('#bkPass', 'phrase secrète solide');
  await p2.click('#btnBkImport');
  await p2.waitForFunction(() => /import|Import/.test($('bkMsg').textContent) && me().name === 'Capitaine', null, { timeout: 15000 });
  const r = await p2.evaluate(() => ({ xp: me().xp, g: me().stats.games, w: me().stats.wins }));
  assert.deepEqual(r, { xp: 450, g: 12, w: 7 });
  // code altéré : refusé (intégrité)
  const bad = code.slice(0, -4) + (code.endsWith('AAAA') ? 'BBBB' : 'AAAA');
  await p2.fill('#bkCode', bad); await p2.fill('#bkPass', 'phrase secrète solide');
  await p2.click('#btnBkImport');
  await p2.waitForFunction(() => /incorrecte|valide|Wrong|valid/.test($('bkMsg').textContent), null, { timeout: 15000 });
  await c1.close(); await c2.close();
});

/* ---------------- Comptes en ligne (API simulée par le vrai serveur, base en mémoire) ---------------- */
test('compte en ligne : inscription, XP calculée par le serveur, connexion sur un autre appareil', async () => {
  const { createApp } = require('../server/app');
  const api = createApp({ DB_PATH: ':memory:', ALLOWED_ORIGINS: new URL(base).origin });
  await new Promise(r => api.server.listen(0, '127.0.0.1', r));
  const url = 'http://127.0.0.1:' + api.server.address().port;
  const mk = async () => {
    const ctx = await newContext();
    await ctx.addInitScript(u => { window.C02_API = u; }, url);
    const pg = await ctx.newPage(); pg.on('dialog', d => d.accept());
    const errors = []; pg.on('pageerror', e => errors.push(e.message));
    await pg.goto(base); return { ctx, pg, errors };
  };
  const A = await mk();
  assert.ok(await A.pg.isVisible('#cloudPane'), 'écran de compte affiché');
  // mauvais mot de passe, puis création
  await A.pg.click('#tabReg');
  await A.pg.fill('#cName', 'Amiral'); await A.pg.fill('#cPass', 'court');
  await A.pg.click('#btnCloud');
  assert.match(await A.pg.textContent('#authMsg'), /8 (à|to) 128/);
  await A.pg.fill('#cPass', 'un-mot-de-passe-solide');
  await A.pg.click('#btnCloud');
  await A.pg.waitForSelector('#dash.active', { timeout: 15000 }).catch(async e => { throw new Error('inscription bloquée : ' + await A.pg.textContent('#authMsg') + ' / ' + A.errors.join('|')); });
  assert.equal(await A.pg.evaluate(() => me().cloud), true);
  // une partie : le serveur recalcule le gain
  const gain = await A.pg.evaluate(() => { S.match = { shots: 10, hits: 6, sunk: 2, upgrades: 0, start: Date.now() - 60000 }; S.vsBot = false; return recordGame(true).gain; });
  await A.pg.waitForFunction(() => !me().pending.length, null, { timeout: 15000 });
  const xpA = await A.pg.evaluate(() => me().xp);
  assert.equal(xpA, gain);
  // pseudo déjà pris (sans tenir compte de la casse) sur un second appareil
  const B = await mk();
  await B.pg.click('#tabReg');
  await B.pg.fill('#cName', 'AMIRAL'); await B.pg.fill('#cPass', 'un-mot-de-passe-solide');
  await B.pg.click('#btnCloud');
  await B.pg.waitForFunction(() => /pris|taken/.test($('authMsg').textContent), null, { timeout: 15000 });
  // mauvais mot de passe puis bon mot de passe : même profil
  await B.pg.click('#tabLogin');
  await B.pg.fill('#cName', 'amiral'); await B.pg.fill('#cPass', 'mauvais-mot-de-passe');
  await B.pg.click('#btnCloud');
  await B.pg.waitForFunction(() => /incorrect|Wrong/.test($('authMsg').textContent), null, { timeout: 15000 });
  await B.pg.fill('#cPass', 'un-mot-de-passe-solide');
  await B.pg.click('#btnCloud');
  await B.pg.waitForSelector('#dash.active');
  assert.equal(await B.pg.evaluate(() => me().xp), xpA);
  assert.equal(await B.pg.evaluate(() => me().stats.wins), 1);
  // déconnexion : le jeton est révoqué
  const tok = await A.pg.evaluate(() => me().tok);
  await A.pg.click('#btnLogout');
  await A.pg.waitForSelector('#auth.active');
  assert.equal((await fetch(url + '/api/me', { headers: { authorization: 'Bearer ' + tok } })).status, 401);
  assert.deepEqual(A.errors.concat(B.errors), []);
  await A.ctx.close(); await B.ctx.close();
  await new Promise(r => api.server.close(r));
});

test('compte en ligne : serveur injoignable => repli automatique sur le pseudo local', async () => {
  const ctx = await newContext();
  await ctx.addInitScript(() => { window.C02_API = 'http://127.0.0.1:1'; });
  const pg = await ctx.newPage(); pg.on('dialog', d => d.accept());
  await pg.goto(base);
  await pg.waitForSelector('#loginPane', { state: 'visible', timeout: 10000 });
  assert.ok(!(await pg.isVisible('#cloudPane')));
  assert.match(await pg.textContent('#authMsg'), /indisponibles|unavailable/);
  await pg.fill('#suName', 'Hors-ligne'); await pg.click('#btnPlay');
  await pg.waitForSelector('#dash.active');
  await ctx.close();
});

/* ---------------- Revanche, animations et XP selon précision / rapidité ---------------- */
test('formule d\'XP : le jeu et le serveur donnent exactement le même résultat', async () => {
  const { xpParts } = require('../server/app');
  const ctx = await newContext(); const pg = await newPage(ctx);
  const cases = [];
  for (const win of [true, false]) for (const half of [true, false]) for (const [shots, hits, sunk] of [[0, 0, 0], [17, 17, 5], [40, 17, 5], [100, 30, 3]])
    for (const duration of [5, 150, 400, 900, 3000]) cases.push({ win, half, shots, hits, sunk, duration });
  const got = await pg.evaluate(cs => cs.map(c => xpParts(c.win, c.shots, c.hits, c.sunk, c.duration, c.half).gain), cases);
  assert.deepEqual(got, cases.map(c => xpParts(c).gain));
  await ctx.close();
});

test('revanche contre le bot : nouvelle manche immédiate, le premier tir alterne', async () => {
  const ctx = await newContext(); const pg = await newPage(ctx);
  await pg.evaluate(() => { BOT_SPEED = 0.03; settings.skill = 0; });
  await pg.click('#mBot'); await ready(pg); await phase(pg, 'game', 8000); await startAuto(pg);
  await phase(pg, 'over', 60000);
  // détail de l'XP affiché : 5 lignes, puis compteur animé jusqu'au gain réel
  await pg.waitForSelector('#end.show');
  assert.equal(await pg.locator('.xplist li').count(), 5);
  await pg.waitForFunction(() => /^\+[1-9]\d* XP$/.test($('xpGainN').textContent) && $('xpGainN').textContent !== '+0 XP', null, { timeout: 8000 });
  await pg.click('#btnRematch');
  await phase(pg, 'place', 5000);
  assert.deepEqual(await pg.evaluate(() => [S.match.shots, S.enemy.every(v => v === null), S.ships.every(s => !s.cells), $('end').classList.contains('show'), S.rounds]), [0, true, true, false, 1]);
  await ready(pg); await phase(pg, 'game', 8000);
  assert.equal(await pg.evaluate(() => S.myTurn), false, 'le bot commence la 2e manche');
  await startAuto(pg); await phase(pg, 'over', 60000);
  assert.equal(await pg.evaluate(() => me().stats.games), 2);
  assert.deepEqual(pg.errors, []);
  await ctx.close();
});

test('revanche entre deux joueurs : les deux doivent accepter', async () => {
  const ctx = await newContext();
  const [A, B] = await hostAndJoin(ctx);
  await ready(A); await ready(B); await phase(A, 'game'); await phase(B, 'game');
  await startAuto(A); await startAuto(B);
  await bothOver(A, B);
  await Promise.all([A, B].map(p => p.evaluate(() => clearInterval(window.__auto))));   // sinon l'ancien tireur automatique jouerait à la place du test
  await A.click('#btnRematch');
  await A.waitForFunction(() => $('rematchNote').textContent.length > 0);
  await B.waitForFunction(() => /revanche|rematch/i.test($('rematchNote').textContent), null, { timeout: 5000 });
  assert.equal(await A.evaluate(() => S.phase), 'over', 'pas de départ tant que l\'autre n\'a pas accepté');
  await B.click('#btnRematch');
  await phase(A, 'place', 5000); await phase(B, 'place', 5000);
  await ready(A); await ready(B); await phase(A, 'game'); await phase(B, 'game');
  assert.deepEqual([await A.evaluate(() => [S.myTurn, S.isHost, S.rounds]), await B.evaluate(() => [S.myTurn, S.isHost, S.rounds])], [[false, true, 1], [true, false, 1]], 'celui qui jouait en second commence');
  await startAuto(A); await startAuto(B);
  await bothOver(A, B); await assertConsistentEnd(A, B);
  assert.deepEqual([...A.errors, ...B.errors], []);
  await ctx.close();
});

test('revanche : impossible si l\'adversaire est parti', async () => {
  const ctx = await newContext();
  const [A, B] = await hostAndJoin(ctx);
  await ready(A); await ready(B); await phase(A, 'game'); await phase(B, 'game');
  await startAuto(A); await startAuto(B);
  await bothOver(A, B);
  await B.click('#btnNewGame');
  await A.waitForFunction(() => $('btnRematch').disabled && /quitt|left/i.test($('rematchNote').textContent), null, { timeout: 5000 });
  await ctx.close();
});

test('téléphone : portrait imposé (manifeste) et message si on le tourne en paysage', async () => {
  const man = JSON.parse(fs.readFileSync(path.join(ROOT, 'manifest.webmanifest'), 'utf8'));
  assert.equal(man.orientation, 'portrait');
  const ctx = await browser.newContext({ serviceWorkers: 'block', locale: 'fr-FR', viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true });
  await ctx.addInitScript(() => { window.C02_API = ''; });
  const pg = await ctx.newPage(); await pg.goto(base);
  const shown = () => pg.evaluate(() => getComputedStyle($('rotateLock')).display !== 'none');
  assert.equal(await shown(), false, 'portrait : pas de message');
  await pg.setViewportSize({ width: 844, height: 390 });
  assert.equal(await shown(), true, 'paysage sur téléphone : message affiché');
  assert.match(await pg.textContent('#rotateLock'), /portrait/);
  await pg.setViewportSize({ width: 390, height: 844 });
  assert.equal(await shown(), false, 'retour en portrait : le jeu réapparaît');
  await ctx.close();
  // PC / souris : jamais bloqué, même dans une fenêtre basse
  const ctx2 = await browser.newContext({ serviceWorkers: 'block', viewport: { width: 900, height: 400 } });
  await ctx2.addInitScript(() => { window.C02_API = ''; });
  const p2 = await ctx2.newPage(); await p2.goto(base);
  assert.equal(await p2.evaluate(() => getComputedStyle($('rotateLock')).display), 'none');
  await ctx2.close();
});

test('le texte du jeu n\'est pas sélectionnable, sauf les champs de saisie et le code de salon', async () => {
  const ctx = await newContext(); const pg = await newPage(ctx, { width: 390, height: 844 }, 'Selection');
  const us = sel => pg.evaluate(s => getComputedStyle(document.querySelector(s)).userSelect, sel);
  assert.equal(await us('body'), 'none');
  assert.equal(await us('.mode-card h3'), 'none');
  assert.equal(await us('#tbName'), 'none');
  // un glissé sur le texte ne sélectionne rien
  await pg.mouse.move(20, 150); await pg.mouse.down(); await pg.mouse.move(370, 400, { steps: 8 }); await pg.mouse.up();
  assert.equal(await pg.evaluate(() => String(getSelection())), '');
  await pg.keyboard.press('Control+A');
  assert.equal(await pg.evaluate(() => String(getSelection())), '');
  // les champs restent utilisables
  await pg.evaluate(() => { pane = 'settings'; renderDash(); });
  assert.equal(await pg.evaluate(() => getComputedStyle($('editName')).userSelect), 'text');
  await pg.fill('#editName', 'Nouveau'); assert.equal(await pg.inputValue('#editName'), 'Nouveau');
  assert.equal(await pg.evaluate(() => getComputedStyle($('codeBox')).userSelect), 'all');
  await ctx.close();
});

/* ---------------- Version, mises à jour, sons ---------------- */
test('numéro de version identique partout (package.json, js/main.js, version.json, service worker)', () => {
  const v = JSON.parse(fs.readFileSync(path.join(ROOT, 'package.json'), 'utf8')).version;
  assert.match(v, /^\d+\.\d+\.\d+$/);
  assert.equal(JSON.parse(fs.readFileSync(path.join(ROOT, 'version.json'), 'utf8')).version, v);
  assert.equal(/const APP_VERSION = '([^']+)'/.exec(fs.readFileSync(path.join(ROOT, 'js/main.js'), 'utf8'))[1], v);
  assert.ok(fs.readFileSync(path.join(ROOT, 'sw.js'), 'utf8').includes(`'c02-naval-v${v}'`));
});

test('paramètres : le numéro de version s\'affiche et la recherche de mise à jour répond', async () => {
  const ctx = await newContext(); const pg = await newPage(ctx, { width: 1000, height: 800 });
  const v = await pg.evaluate(() => APP_VERSION);
  await pg.evaluate(() => { pane = 'settings'; renderDash(); });
  assert.equal(await pg.textContent('#verNum'), v);
  await pg.click('#btnVer');
  await pg.waitForFunction(() => /dernière version|latest/.test($('verMsg').textContent), null, { timeout: 5000 });
  assert.equal(await pg.isVisible('#updBar'), false, 'rien à signaler : pas de bandeau');
  await ctx.close();
});

test('mise à jour : une version plus récente est détectée à l\'ouverture, un seul rechargement automatique', async () => {
  const ctx = await newContext();
  let hits = 0;
  await ctx.route(/version\.json/, r => { hits++; r.fulfill({ contentType: 'application/json', body: '{"version":"99.0.0"}' }); });
  const pg = await ctx.newPage();
  let loads = 0; pg.on('load', () => loads++);
  await pg.goto(base);
  // 1er chargement : rechargement automatique ; 2e : garde-fou, le bandeau propose la mise à jour
  await pg.waitForSelector('#updBar', { state: 'visible', timeout: 15000 });
  assert.equal(loads, 2, 'un seul rechargement automatique (pas de boucle)');
  assert.match(await pg.textContent('#updTxt'), /99\.0\.0/);
  assert.ok(hits >= 2);
  await ctx.close();
});

test('mise à jour : hors ligne ou fichier illisible, le jeu fonctionne sans rien afficher', async () => {
  const ctx = await newContext();
  await ctx.route(/version\.json/, r => r.abort());
  const pg = await newPage(ctx);
  await pg.waitForTimeout(800);
  assert.equal(await pg.isVisible('#updBar'), false);
  assert.equal(await pg.evaluate(() => S.phase), 'menu');
  await ctx.close();
});

test('sons : chaque son est défini, joue sans erreur, et le volume est réglable', async () => {
  const names = new Set([...SOURCE().matchAll(/Snd\.play\('([a-z]+)'\)/g)].map(m => m[1]));
  ['win', 'lose', 'levelup', 'xp'].forEach(n => names.add(n));
  const ctx = await newContext(); const pg = await newPage(ctx);
  const r = await pg.evaluate(async list => {
    settings.sound = true; settings.volume = 0.5;
    const missing = list.filter(n => typeof Snd.sounds[n] !== 'function');
    const errs = [];
    for (const n of list) { try { Snd.sounds[n] && Snd.sounds[n](Snd.get(), Snd); } catch (e) { errs.push(n + ': ' + e.message); } }
    return { missing, errs, vol: Snd.bus.gain.value, hasRev: !!Snd.rev };
  }, [...names]);
  assert.deepEqual(r.missing, []);
  assert.deepEqual(r.errs, []);
  assert.equal(r.vol, 0.5); assert.ok(r.hasRev);
  await pg.evaluate(() => { pane = 'settings'; renderDash(); });
  await pg.fill('#volRange', '30');
  assert.equal(await pg.evaluate(() => settings.volume), 0.3);
  assert.deepEqual(pg.errors, []);
  await ctx.close();
});
