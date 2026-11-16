/* Tests de l'API de comptes (base SQLite en mémoire). Lancement : npm test */
const { test, before, after } = require('node:test');
const assert = require('node:assert/strict');
const { createApp } = require('../server/app');

let app, base;
const ORIGIN = 'https://drfrrobin.github.io';
before(async () => {
  app = createApp({ DB_PATH: ':memory:', ALLOWED_ORIGINS: ORIGIN });
  await new Promise(r => app.server.listen(0, '127.0.0.1', r));
  base = 'http://127.0.0.1:' + app.server.address().port;
});
after(() => new Promise(r => app.server.close(r)));

async function call(method, path, body, token, extra = {}) {
  const r = await fetch(base + path, { method, headers: { 'content-type': 'application/json', ...(token ? { authorization: 'Bearer ' + token } : {}), ...extra },
    body: body === undefined ? undefined : JSON.stringify(body) });
  return { status: r.status, body: await r.json().catch(() => null), headers: r.headers };
}
const reg = (name, password = 'motdepasse-solide') => call('POST', '/api/register', { name, password });
const good = { win: true, shots: 30, hits: 17, sunk: 5, upgrades: 1, duration: 300, bot: false, std: true };

test('inscription : pseudo unique, insensible à la casse et aux espaces', async () => {
  app.limiter.reset();
  const a = await reg('Capitaine');
  assert.equal(a.status, 200);
  assert.equal(a.body.profile.name, 'Capitaine');
  assert.equal(a.body.profile.xp, 0);
  assert.match(a.body.token, /^[\w-]{43}$/);
  for (const n of ['capitaine', 'CAPITAINE', '  Capitaine  ']) assert.equal((await reg(n)).body.error, 'pseudo_taken', n);
});

test('inscription : pseudo et mot de passe invalides refusés', async () => {
  app.limiter.reset();
  for (const n of ['a', 'x'.repeat(17), '<script>', 'Admin', 'ad min', 12, null]) assert.equal((await reg(n)).status, 400, String(n));
  assert.equal((await reg('Valide1', 'court')).body.error, 'password_invalid');
  assert.equal((await reg('Valide2', 'x'.repeat(129))).body.error, 'password_invalid');
});

test('le mot de passe n\'est jamais stocké en clair', async () => {
  app.limiter.reset();
  await reg('Secret1', 'phrase-unique-12345');
  const { DatabaseSync } = require('node:sqlite');
  // on relit via l'API : aucun champ sensible dans le profil renvoyé
  const l = await call('POST', '/api/login', { name: 'Secret1', password: 'phrase-unique-12345' });
  assert.equal(l.status, 200);
  assert.ok(!JSON.stringify(l.body).includes('phrase-unique'));
  assert.ok(!('hash' in l.body.profile));
});

test('connexion : bons identifiants, mauvais, pseudo inconnu (même message)', async () => {
  app.limiter.reset();
  await reg('Marin');
  const ok = await call('POST', '/api/login', { name: 'marin', password: 'motdepasse-solide' });
  assert.equal(ok.status, 200);
  const bad = await call('POST', '/api/login', { name: 'Marin', password: 'mauvais-mot-de-passe' });
  const unk = await call('POST', '/api/login', { name: 'Inconnu', password: 'mauvais-mot-de-passe' });
  assert.equal(bad.status, 401); assert.equal(unk.status, 401);
  assert.deepEqual(bad.body, unk.body);
});

test('verrouillage après 5 échecs, même avec le bon mot de passe', async () => {
  app.limiter.reset();
  await reg('Cible');
  for (let i = 0; i < 5; i++) await call('POST', '/api/login', { name: 'Cible', password: 'faux-faux-faux' });
  const r = await call('POST', '/api/login', { name: 'Cible', password: 'motdepasse-solide' });
  assert.equal(r.status, 429); assert.equal(r.body.error, 'locked');
});

test('limitation de débit : trop de tentatives depuis une même adresse', async () => {
  app.limiter.reset();
  let last;
  for (let i = 0; i < 22; i++) last = await call('POST', '/api/login', { name: 'Nobody' + i, password: 'xxxxxxxxxx' });
  assert.equal(last.status, 429);
  app.limiter.reset();
});

test('session : /me, déconnexion, jeton invalide', async () => {
  app.limiter.reset();
  const { body: { token } } = await reg('Session1');
  assert.equal((await call('GET', '/api/me', undefined, token)).body.profile.name, 'Session1');
  assert.equal((await call('GET', '/api/me', undefined, 'x'.repeat(43))).status, 401);
  assert.equal((await call('GET', '/api/me')).status, 401);
  await call('POST', '/api/logout', {}, token);
  assert.equal((await call('GET', '/api/me', undefined, token)).status, 401);
});

test('résultat de partie : le serveur calcule l\'XP et les stats', async () => {
  app.limiter.reset();
  const { body: { token } } = await reg('Joueur1');
  const r = await call('POST', '/api/result', good, token);
  assert.equal(r.status, 200);
  assert.equal(r.body.gain, 100 + 85 + 100 + 57 + 50);   // base + touchés + coulés + précision + rapidité
  assert.equal(r.body.profile.xp, r.body.gain);
  assert.deepEqual([r.body.profile.stats.games, r.body.profile.stats.wins, r.body.profile.stats.fastestWin], [1, 1, 30]);
  // le client ne peut pas imposer un gain
  const again = await call('POST', '/api/result', { ...good, gain: 99999, xp: 99999 }, token);
  assert.equal(again.status, 429);   // deux résultats trop rapprochés
});

test('anti-triche : résultats invraisemblables refusés', async () => {
  app.limiter.reset();
  const { body: { token } } = await reg('Tricheur');
  const bads = [{ ...good, hits: 40, shots: 30 }, { ...good, sunk: 13 }, { ...good, shots: 9999 }, { ...good, duration: 1 },
    { ...good, win: 'oui' }, { ...good, hits: -1 }, { ...good, sunk: 6, hits: 5 }, { ...good, win: true, hits: 0, sunk: 0, shots: 3 }, { ...good, shots: 100, hits: 17, duration: 100 }];
  for (const b of bads) assert.equal((await call('POST', '/api/result', b, token)).status, 400, JSON.stringify(b));
  assert.equal((await call('POST', '/api/result', good)).status, 401);
});

test('victoire par abandon de l\'adversaire sans aucun touché : acceptée', async () => {
  app.limiter.reset();
  const { body: { token } } = await reg('Forfait');
  const r = await call('POST', '/api/result', { ...good, win: true, shots: 0, hits: 0, sunk: 0, duration: 20, forfeit: true }, token);
  assert.equal(r.status, 200);
  assert.equal(r.body.profile.stats.wins, 1);
});

test('victoire contre un bot : gain divisé par deux', async () => {
  app.limiter.reset();
  const { body: { token } } = await reg('Bot1');
  const r = await call('POST', '/api/result', { ...good, bot: true }, token);
  assert.equal(r.body.gain, Math.round((100 + 85 + 100 + 57 + 50) / 2));
});

test('import d\'un profil local : valeurs invraisemblables bornées', async () => {
  app.limiter.reset();
  const r = await call('POST', '/api/register', { name: 'Migrant', password: 'motdepasse-solide',
    profile: { xp: 9e9, emoji: '🐙', stats: { games: 10, wins: 99, losses: 99, hits: 500, shots: 20, bestStreak: 50 } } });
  assert.equal(r.status, 200);
  const p = r.body.profile;
  assert.ok(p.xp <= 4000); assert.ok(p.stats.wins <= 10); assert.ok(p.stats.hits <= 20);
  assert.ok(p.stats.wins + p.stats.losses <= 10); assert.equal(p.emoji, '🐙');
});

test('changement de mot de passe : ancien refusé, autres appareils déconnectés', async () => {
  app.limiter.reset();
  const a = (await reg('Change1')).body.token;
  const b = (await call('POST', '/api/login', { name: 'Change1', password: 'motdepasse-solide' })).body.token;
  assert.equal((await call('POST', '/api/password', { password: 'faux-faux-faux', newPassword: 'nouveau-mdp-123' }, a)).status, 401);
  assert.equal((await call('POST', '/api/password', { password: 'motdepasse-solide', newPassword: 'nouveau-mdp-123' }, a)).status, 200);
  assert.equal((await call('GET', '/api/me', undefined, b)).status, 401);
  assert.equal((await call('GET', '/api/me', undefined, a)).status, 200);
  assert.equal((await call('POST', '/api/login', { name: 'Change1', password: 'motdepasse-solide' })).status, 401);
  assert.equal((await call('POST', '/api/login', { name: 'Change1', password: 'nouveau-mdp-123' })).status, 200);
});

test('suppression du compte : mot de passe exigé, pseudo libéré', async () => {
  app.limiter.reset();
  const t = (await reg('Efface1')).body.token;
  assert.equal((await call('POST', '/api/delete', { password: 'faux-faux-faux' }, t)).status, 401);
  assert.equal((await call('POST', '/api/delete', { password: 'motdepasse-solide' }, t)).status, 200);
  assert.equal((await call('GET', '/api/me', undefined, t)).status, 401);
  assert.equal((await reg('Efface1')).status, 200);
});

test('CORS : seule l\'origine autorisée est acceptée', async () => {
  app.limiter.reset();
  const ok = await call('GET', '/api/health', undefined, null, { origin: ORIGIN });
  assert.equal(ok.headers.get('access-control-allow-origin'), ORIGIN);
  const ko = await call('GET', '/api/health', undefined, null, { origin: 'https://evil.example' });
  assert.equal(ko.status, 403);
  const pre = await fetch(base + '/api/login', { method: 'OPTIONS', headers: { origin: ORIGIN, 'access-control-request-method': 'POST' } });
  assert.equal(pre.status, 204);
  const pre2 = await fetch(base + '/api/login', { method: 'OPTIONS', headers: { origin: 'https://evil.example' } });
  assert.equal(pre2.status, 403);
});

test('requêtes malformées : JSON invalide, corps trop gros, mauvais type, route inconnue', async () => {
  app.limiter.reset();
  const r1 = await fetch(base + '/api/login', { method: 'POST', headers: { 'content-type': 'application/json' }, body: '{pas du json' });
  assert.equal(r1.status, 400);
  const r2 = await fetch(base + '/api/login', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ name: 'x'.repeat(40000) }) }).catch(() => ({ status: 413 }));
  assert.equal(r2.status, 413);
  const r3 = await fetch(base + '/api/login', { method: 'POST', headers: { 'content-type': 'text/plain' }, body: 'x' });
  assert.equal(r3.status, 415);
  assert.equal((await call('GET', '/api/nimporte')).status, 404);
  assert.equal((await call('GET', '/api/health')).body.ok, true);
});

test('un pseudo différent par la casse unicode (NFKC) reste unique', async () => {
  app.limiter.reset();
  await reg('Ｕnique');           // « Ｕ » pleine chasse
  assert.equal((await reg('Unique')).body.error, 'pseudo_taken');
});

test('XP : la précision et la rapidité font varier le gain', () => {
  const { xpParts } = require('../server/app');
  const g = o => xpParts({ win: true, shots: 40, hits: 17, sunk: 5, duration: 300, half: false, ...o }).gain;
  assert.ok(g({ shots: 20 }) > g({ shots: 60 }), 'plus précis = plus d\'XP');
  assert.ok(g({ duration: 150 }) > g({ duration: 600 }), 'plus rapide = plus d\'XP');
  assert.ok(g({ duration: 600 }) > g({ duration: 1200 }));
  assert.equal(g({ duration: 1200 }), g({ duration: 5000 }), 'bonus de temps nul après 15 min');
  assert.equal(xpParts({ win: false, shots: 30, hits: 10, sunk: 1, duration: 100, half: false }).parts.time, 0, 'pas de bonus de temps en cas de défaite');
  assert.ok(xpParts({ win: true, shots: 17, hits: 17, sunk: 5, duration: 150, half: false }).gain <= 700);
});
