'use strict';
/* C02 Naval — API de comptes (pseudo unique + mot de passe).
   Aucune dépendance : http, node:sqlite et crypto.scrypt sont intégrés à Node (>= 22.5). */
const http = require('node:http');
const crypto = require('node:crypto');
const { DatabaseSync } = require('node:sqlite');

const EMOJIS_MAX = 8;
const STAT_KEYS = ['games', 'wins', 'losses', 'interrupted', 'streak', 'bestStreak', 'shots', 'hits', 'sunk', 'upgrades', 'timePlayed'];
const emptyStats = () => ({ games: 0, wins: 0, losses: 0, interrupted: 0, streak: 0, bestStreak: 0,
  shots: 0, hits: 0, sunk: 0, upgrades: 0, fastestWin: null, timePlayed: 0 });
const RESERVED = new Set(['admin', 'administrateur', 'administrator', 'root', 'system', 'moderator', 'moderateur', 'support', 'c02', 'c02naval', 'bot', 'invite', 'guest', 'null', 'undefined']);
/* Formule d'XP (identique dans js/profile.js) : participation/victoire + touchés + bateaux coulés
   + précision (tirs au but / tirs) + rapidité en cas de victoire (60 XP avant 3 min, dégressif, 0 après 15 min). */
const clamp01 = x => Math.min(1, Math.max(0, x));
function xpParts({ win, shots, hits, sunk, duration, half }) {
  const acc = shots ? hits / shots : 0;
  const parts = { base: win ? 100 : 30, hits: hits * 5, sunk: sunk * 20, acc: Math.round(acc * (win ? 100 : 30)), time: win ? Math.round(60 * clamp01((900 - duration) / 720)) : 0 };
  const sum = parts.base + parts.hits + parts.sunk + parts.acc + parts.time;
  return { parts, gain: half ? Math.round(sum / 2) : sum };
}
const SESSION_MS = 60 * 24 * 3600 * 1000;
const MIN_RESULT_GAP_MS = 15000;

class HttpError extends Error { constructor(status, code, extra) { super(code); this.status = status; this.code = code; this.extra = extra; } }

function normName(raw) {
  if (typeof raw !== 'string') throw new HttpError(400, 'pseudo_invalid');
  const name = raw.normalize('NFC').trim().replace(/\s+/g, ' ');
  if (name.length < 2 || name.length > 16 || !/^[\p{L}\p{N} _.\-]+$/u.test(name)) throw new HttpError(400, 'pseudo_invalid');
  const key = name.normalize('NFKC').toLowerCase();
  if (RESERVED.has(key.replace(/[ _.\-]/g, ''))) throw new HttpError(400, 'pseudo_reserved');
  return { name, key };
}
function checkPassword(p) {
  if (typeof p !== 'string' || p.length < 8 || p.length > 128) throw new HttpError(400, 'password_invalid');
  return p;
}

/* ---------- Mots de passe : scrypt, sel aléatoire, comparaison à temps constant ---------- */
const SCRYPT = { N: 32768, r: 8, p: 1, maxmem: 96 * 1024 * 1024 };
const scrypt = (pw, salt, len, opt) => new Promise((res, rej) => crypto.scrypt(pw.normalize('NFKC'), salt, len, opt, (e, k) => (e ? rej(e) : res(k))));
async function hashPassword(pw) {
  const salt = crypto.randomBytes(16);
  const k = await scrypt(pw, salt, 32, SCRYPT);
  return ['s1', SCRYPT.N, SCRYPT.r, SCRYPT.p, salt.toString('base64'), k.toString('base64')].join('$');
}
async function verifyPassword(pw, stored) {
  const [v, N, r, p, s, h] = String(stored).split('$');
  if (v !== 's1') return false;
  const want = Buffer.from(h, 'base64');
  const got = await scrypt(pw, Buffer.from(s, 'base64'), want.length, { N: +N, r: +r, p: +p, maxmem: SCRYPT.maxmem });
  return got.length === want.length && crypto.timingSafeEqual(got, want);
}
let dummyHash;   // pour égaliser le temps de réponse quand le pseudo n'existe pas
const sha256 = s => crypto.createHash('sha256').update(s).digest('hex');

/* ---------- Limitation de débit en mémoire ---------- */
function makeLimiter() {
  const hits = new Map();
  const timer = setInterval(() => { const now = Date.now(); for (const [k, v] of hits) if (!v.some(t => now - t < 3600e3)) hits.delete(k); }, 60e3);
  timer.unref();
  return {
    // true si l'appel est autorisé (et compté)
    take(key, max, windowMs) {
      const now = Date.now(), arr = (hits.get(key) || []).filter(t => now - t < windowMs);
      if (arr.length >= max) { hits.set(key, arr); return false; }
      arr.push(now); hits.set(key, arr); return true;
    },
    reset() { hits.clear(); },
    stop() { clearInterval(timer); }
  };
}

function createApp(opts = {}) {
  const env = { ...process.env, ...opts };
  const origins = String(env.ALLOWED_ORIGINS || 'https://drfrrobin.github.io').split(',').map(s => s.trim()).filter(Boolean);
  const trustProxy = env.TRUST_PROXY === '1' || env.TRUST_PROXY === true;
  const db = new DatabaseSync(env.DB_PATH || ':memory:');
  db.exec('PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=3000;');
  db.exec(`
    CREATE TABLE IF NOT EXISTS users(
      id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, name_key TEXT NOT NULL UNIQUE, hash TEXT NOT NULL,
      emoji TEXT, xp INTEGER NOT NULL DEFAULT 0, stats TEXT NOT NULL, created INTEGER NOT NULL, last_login INTEGER NOT NULL,
      last_result INTEGER NOT NULL DEFAULT 0, fails INTEGER NOT NULL DEFAULT 0, locked_until INTEGER NOT NULL DEFAULT 0);
    CREATE TABLE IF NOT EXISTS sessions(
      token_hash TEXT PRIMARY KEY, user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
      created INTEGER NOT NULL, expires INTEGER NOT NULL);
    CREATE INDEX IF NOT EXISTS sessions_user ON sessions(user_id);`);
  const limiter = makeLimiter();
  const q = {
    byKey: db.prepare('SELECT * FROM users WHERE name_key=?'),
    byId: db.prepare('SELECT * FROM users WHERE id=?'),
    insert: db.prepare('INSERT INTO users(name,name_key,hash,emoji,xp,stats,created,last_login) VALUES(?,?,?,?,?,?,?,?)'),
    session: db.prepare('SELECT user_id, expires FROM sessions WHERE token_hash=?'),
    addSession: db.prepare('INSERT INTO sessions(token_hash,user_id,created,expires) VALUES(?,?,?,?)'),
    delSession: db.prepare('DELETE FROM sessions WHERE token_hash=?'),
    delOthers: db.prepare('DELETE FROM sessions WHERE user_id=? AND token_hash<>?'),
    delUserSessions: db.prepare('DELETE FROM sessions WHERE user_id=?'),
    purge: db.prepare('DELETE FROM sessions WHERE expires<?'),
    fail: db.prepare('UPDATE users SET fails=?, locked_until=? WHERE id=?'),
    login: db.prepare('UPDATE users SET fails=0, locked_until=0, last_login=? WHERE id=?'),
    setPass: db.prepare('UPDATE users SET hash=? WHERE id=?'),
    setEmoji: db.prepare('UPDATE users SET emoji=? WHERE id=?'),
    setGame: db.prepare('UPDATE users SET xp=?, stats=?, last_result=? WHERE id=?'),
    delUser: db.prepare('DELETE FROM users WHERE id=?')
  };

  const profile = u => ({ name: u.name, emoji: u.emoji || null, xp: u.xp, created: u.created, stats: { ...emptyStats(), ...JSON.parse(u.stats) } });
  function newSession(userId) {
    const token = crypto.randomBytes(32).toString('base64url'), now = Date.now();
    q.addSession.run(sha256(token), userId, now, now + SESSION_MS);
    return token;
  }
  function auth(req) {
    const m = /^Bearer ([A-Za-z0-9_-]{43})$/.exec(req.headers.authorization || '');
    if (!m) throw new HttpError(401, 'unauthorized');
    const th = sha256(m[1]), s = q.session.get(th);
    if (!s || s.expires < Date.now()) throw new HttpError(401, 'unauthorized');
    const user = q.byId.get(s.user_id);
    if (!user) throw new HttpError(401, 'unauthorized');
    return { user, th };
  }
  const int = (v, max) => { if (!Number.isInteger(v) || v < 0 || v > max) throw new HttpError(400, 'invalid_result'); return v; };
  const okEmoji = e => e === null || (typeof e === 'string' && e.length > 0 && e.length <= EMOJIS_MAX);

  // Reprend un profil local lors de la création du compte, en bornant ce qui est invraisemblable.
  function sanitizeImport(p) {
    const st = emptyStats();
    if (!p || typeof p !== 'object') return { xp: 0, stats: st, emoji: null };
    const s = p.stats && typeof p.stats === 'object' ? p.stats : {};
    const n = (v, max) => (Number.isFinite(v) ? Math.min(Math.max(Math.floor(v), 0), max) : 0);
    for (const k of STAT_KEYS) st[k] = n(s[k], 1e6);
    st.games = Math.min(st.games, 20000);
    st.wins = Math.min(st.wins, st.games); st.losses = Math.min(st.losses, st.games - st.wins);
    st.hits = Math.min(st.hits, st.shots); st.bestStreak = Math.min(st.bestStreak, st.wins); st.streak = Math.min(st.streak, st.bestStreak);
    st.fastestWin = Number.isFinite(s.fastestWin) && s.fastestWin >= 1 && s.fastestWin <= 500 ? Math.floor(s.fastestWin) : null;
    return { xp: Math.min(n(p.xp, 1e7), st.games * 400), stats: st, emoji: okEmoji(p.emoji) ? (p.emoji || null) : null };
  }

  /* ---------- Routes ---------- */
  const routes = {
    'GET /api/health': async () => ({ ok: true }),

    'POST /api/register': async (req, body, ip) => {
      const { name, key } = normName(body.name); const pw = checkPassword(body.password);
      if (!limiter.take('reg:' + ip, 5, 3600e3)) throw new HttpError(429, 'rate_limited');   // seules les inscriptions valides comptent
      if (q.byKey.get(key)) throw new HttpError(409, 'pseudo_taken');
      const imp = sanitizeImport(body.profile), now = Date.now();
      let id;
      try { id = Number(q.insert.run(name, key, await hashPassword(pw), imp.emoji, imp.xp, JSON.stringify(imp.stats), now, now).lastInsertRowid); }
      catch (e) { throw new HttpError(409, 'pseudo_taken'); }   // course : le UNIQUE de la base tranche
      return { token: newSession(id), profile: profile(q.byId.get(id)) };
    },

    'POST /api/login': async (req, body, ip) => {
      if (!limiter.take('login:' + ip, 20, 600e3)) throw new HttpError(429, 'rate_limited');
      let key; try { key = normName(body.name).key; } catch (e) { key = null; }
      const pw = typeof body.password === 'string' ? body.password.slice(0, 128) : '';
      const u = key && q.byKey.get(key), now = Date.now();
      if (u && u.locked_until > now) throw new HttpError(429, 'locked', { retryAfter: Math.ceil((u.locked_until - now) / 1000) });
      dummyHash = dummyHash || await hashPassword('dummy-password-for-timing');
      const good = await verifyPassword(pw, u ? u.hash : dummyHash);
      if (!u || !good) {
        if (u) { const f = u.fails + 1; if (f >= 5) q.fail.run(0, now + 15 * 60e3, u.id); else q.fail.run(f, 0, u.id); }
        throw new HttpError(401, 'bad_credentials');
      }
      q.login.run(now, u.id); q.purge.run(now);
      return { token: newSession(u.id), profile: profile(q.byId.get(u.id)) };
    },

    'POST /api/logout': async req => { const { th } = auth(req); q.delSession.run(th); return { ok: true }; },
    'GET /api/me': async req => ({ profile: profile(auth(req).user) }),

    'PATCH /api/me': async (req, body) => {
      const { user } = auth(req);
      if (!('emoji' in body) || !okEmoji(body.emoji)) throw new HttpError(400, 'invalid');
      q.setEmoji.run(body.emoji || null, user.id);
      return { profile: profile(q.byId.get(user.id)) };
    },

    'POST /api/password': async (req, body, ip) => {
      const { user, th } = auth(req);
      if (!limiter.take('pw:' + user.id, 10, 3600e3)) throw new HttpError(429, 'rate_limited');
      const np = checkPassword(body.newPassword);
      if (!(await verifyPassword(String(body.password || '').slice(0, 128), user.hash))) throw new HttpError(401, 'bad_credentials');
      q.setPass.run(await hashPassword(np), user.id); q.delOthers.run(user.id, th);   // les autres appareils sont déconnectés
      return { ok: true };
    },

    'POST /api/delete': async (req, body) => {
      const { user } = auth(req);
      if (!limiter.take('del:' + user.id, 5, 3600e3)) throw new HttpError(429, 'rate_limited');
      if (!(await verifyPassword(String(body.password || '').slice(0, 128), user.hash))) throw new HttpError(401, 'bad_credentials');
      q.delUserSessions.run(user.id); q.delUser.run(user.id);
      return { ok: true };
    },

    // Résultat d'une partie : le serveur recalcule l'XP lui-même (le client ne choisit jamais le gain).
    'POST /api/result': async (req, body) => {
      const { user } = auth(req), now = Date.now();
      if (now - user.last_result < MIN_RESULT_GAP_MS) throw new HttpError(429, 'too_fast');
      if (typeof body.win !== 'boolean') throw new HttpError(400, 'invalid_result');
      const shots = int(body.shots, 500), hits = int(body.hits, 500), sunk = int(body.sunk, 12);
      const upgrades = int(body.upgrades, 30), duration = int(body.duration, 7200);
      if (hits > shots || sunk > hits || duration < 5 || (body.win && hits < 1)) throw new HttpError(400, 'invalid_result');
      const half = body.bot === true || body.std !== true;
      if (duration < shots * 1.5) throw new HttpError(400, 'invalid_result');   // impossible de tirer aussi vite : durée falsifiée
      const { gain } = xpParts({ win: body.win, shots, hits, sunk, duration, half });
      const s = { ...emptyStats(), ...JSON.parse(user.stats) };
      s.games++; s.shots += shots; s.hits += hits; s.sunk += sunk; s.upgrades += upgrades; s.timePlayed += duration;
      if (body.win) { s.wins++; s.streak++; s.bestStreak = Math.max(s.bestStreak, s.streak); if (shots > 0 && (!s.fastestWin || shots < s.fastestWin)) s.fastestWin = shots; }
      else { s.losses++; s.streak = 0; }
      q.setGame.run(user.xp + gain, JSON.stringify(s), now, user.id);
      return { gain, profile: profile(q.byId.get(user.id)) };
    },

    'POST /api/interrupted': async (req, body) => {
      const { user } = auth(req);
      const s = { ...emptyStats(), ...JSON.parse(user.stats) };
      s.interrupted++; s.timePlayed += int(Number.isInteger(body.duration) ? body.duration : 0, 7200);
      q.setGame.run(user.xp, JSON.stringify(s), user.last_result, user.id);
      return { profile: profile(q.byId.get(user.id)) };
    }
  };

  /* ---------- HTTP ---------- */
  function send(res, status, obj, headers = {}) {
    const data = JSON.stringify(obj);
    res.writeHead(status, { 'content-type': 'application/json; charset=utf-8', 'cache-control': 'no-store',
      'x-content-type-options': 'nosniff', 'referrer-policy': 'no-referrer', 'content-length': Buffer.byteLength(data), ...headers });
    res.end(data);
  }
  function readBody(req) {
    return new Promise((resolve, reject) => {
      const chunks = []; let size = 0;
      req.on('data', c => { size += c.length; if (size > 16384) { reject(new HttpError(413, 'too_large')); req.destroy(); } else chunks.push(c); });
      req.on('end', () => {
        if (!size) return resolve({});
        try { const o = JSON.parse(Buffer.concat(chunks).toString('utf8')); resolve(o && typeof o === 'object' && !Array.isArray(o) ? o : {}); }
        catch (e) { reject(new HttpError(400, 'bad_json')); }
      });
      req.on('error', () => reject(new HttpError(400, 'bad_request')));
    });
  }
  const server = http.createServer(async (req, res) => {
    const origin = req.headers.origin;
    const allowed = origin && origins.includes(origin);
    const cors = allowed ? { 'access-control-allow-origin': origin, vary: 'Origin' } : { vary: 'Origin' };
    if (req.method === 'OPTIONS') {
      return allowed ? (res.writeHead(204, { ...cors, 'access-control-allow-methods': 'GET,POST,PATCH,OPTIONS',
        'access-control-allow-headers': 'authorization,content-type', 'access-control-max-age': '86400' }), res.end()) : (res.writeHead(403), res.end());
    }
    if (origin && !allowed) return send(res, 403, { error: 'origin_not_allowed' });
    const ip = (trustProxy && (req.headers['cf-connecting-ip'] || '').trim()) || req.socket.remoteAddress || '?';
    try {
      if (!limiter.take('all:' + ip, 300, 60e3)) throw new HttpError(429, 'rate_limited');
      const route = routes[req.method + ' ' + (req.url || '').split('?')[0]];
      if (!route) throw new HttpError(404, 'not_found');
      let body = {};
      if (req.method !== 'GET') {
        if (!/^application\/json/i.test(req.headers['content-type'] || '')) throw new HttpError(415, 'json_only');
        body = await readBody(req);
      }
      send(res, 200, await route(req, body, ip), cors);
    } catch (e) {
      if (e instanceof HttpError) {
        const h = { ...cors }; if (e.extra && e.extra.retryAfter) h['retry-after'] = String(e.extra.retryAfter);
        return send(res, e.status, { error: e.code, ...(e.extra || {}) }, h);
      }
      console.error('erreur interne :', e && e.message);
      send(res, 500, { error: 'server_error' }, cors);
    }
  });
  server.requestTimeout = 15000; server.headersTimeout = 10000;
  server.on('close', () => { limiter.stop(); try { db.close(); } catch (e) {} });
  return { server, limiter };
}

module.exports = { createApp, xpParts };
