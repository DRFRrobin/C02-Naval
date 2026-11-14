/* Faux serveur Supabase pour les tests (côté Node, partagé entre toutes les pages d'un contexte).
   Reproduit la logique de supabase/schema.sql (profils, record_game, matchmaking) et les salons Realtime
   (diffusion + présence). La logique SQL elle-même est testée séparément sur un vrai PostgreSQL. */
const fs = require('node:fs');
const path = require('node:path');

const EMOJIS = (() => {
  const src = fs.readFileSync(path.join(__dirname, '..', 'index.html'), 'utf8');
  const block = /const EMOJIS = \[(.*?)\];/s.exec(src)[1];
  return [...block.matchAll(/'([^']+)'/g)].map(m => m[1]);
})();

class FakeBackend {
  constructor() {
    this.profiles = new Map();     // uid -> profil (colonnes de la table profiles)
    this.queue = [];               // [{ ticket, room, at }]
    this.rooms = new Map();        // nom du canal -> Map(id -> { page, key, meta })
    this.rateLimitMs = 20000;
    this.calls = [];               // journal des appels RPC
  }

  async attach(context) {
    await context.exposeBinding('__sbCall', (source, op, args) => this.handle(source.page, op, args || {}));
  }

  /* ----- Livraison vers les pages (ordre garanti par page) ----- */
  deliver(page, id, kind, data) {
    page.__q = (page.__q || Promise.resolve()).then(() =>
      page.evaluate(([i, k, d]) => window.__sbDeliver && window.__sbDeliver(i, k, d), [id, kind, data]).catch(() => {}));
  }
  presenceOf(room) {
    const st = {};
    for (const [, m] of room) if (m.meta) st[m.key] = [{ ...m.meta, presence_ref: m.key }];
    return st;
  }
  syncRoom(name) {
    const room = this.rooms.get(name); if (!room) return;
    const st = this.presenceOf(room);
    for (const [id, m] of room) this.deliver(m.page, id, 'presence', st);
  }

  async handle(page, op, a) {
    switch (op) {
      case 'rpc': this.calls.push(a.name); return this.rpc(a.uid, a.name, a.args || {});
      case 'profileGet': { const p = this.profiles.get(a.uid); return { data: p ? { ...p } : null, error: null }; }
      case 'join': {
        if (!this.rooms.has(a.name)) this.rooms.set(a.name, new Map());
        this.rooms.get(a.name).set(a.id, { page, key: a.key, meta: null, name: a.name });
        setTimeout(() => this.deliver(page, a.id, 'status', 'SUBSCRIBED'), 5);
        return 'ok';
      }
      case 'track': {
        for (const [name, room] of this.rooms) if (room.has(a.id)) { room.get(a.id).meta = a.meta; this.syncRoom(name); }
        return 'ok';
      }
      case 'leave': {
        for (const [name, room] of this.rooms) if (room.delete(a.id)) { this.syncRoom(name); if (!room.size) this.rooms.delete(name); }
        return 'ok';
      }
      case 'send': {
        for (const [, room] of this.rooms) if (room.has(a.id)) for (const [id, m] of room) if (id !== a.id) this.deliver(m.page, id, 'broadcast', a.payload);
        return 'ok';
      }
    }
    throw new Error('op inconnue : ' + op);
  }

  /* ----- Fonctions SQL (miroir de supabase/schema.sql) ----- */
  err(message) { return { data: null, error: { message } }; }
  rpc(uid, name, a) {
    const norm = s => String(s ?? '').trim().replace(/\s+/g, ' ');
    const badName = n => n.length < 2 || n.length > 16 || /[\u0001-\u001F​-‏‪-‮⁠-⁯﻿]/.test(n);
    const taken = (n, except) => [...this.profiles.entries()].some(([id, p]) => id !== except && p.name.toLowerCase() === n.toLowerCase());
    const needAuth = () => (uid ? null : this.err('not_authenticated'));
    const clamp = (v, lo, hi) => Math.min(Math.max(Number.isFinite(+v) ? Math.trunc(+v) : 0, lo), hi);

    if (name === 'matchmake') {
      const { p_ticket: tk, p_room: room } = a;
      if (!tk || tk.length < 6 || tk.length > 40 || !room || room.length !== 6) return this.err('bad_request');
      const now = Date.now();
      this.queue = this.queue.filter(q => now - q.at < 60000 && q.ticket !== tk);
      const picked = this.queue.shift();
      if (picked) return { data: { role: 'guest', room: picked.room }, error: null };
      this.queue.push({ ticket: tk, room, at: now });
      return { data: { role: 'host', room }, error: null };
    }
    if (name === 'leave_queue') { this.queue = this.queue.filter(q => q.ticket !== a.p_ticket); return { data: null, error: null }; }

    const e0 = needAuth(); if (e0) return e0;
    const prof = this.profiles.get(uid);
    if (name === 'create_profile') {
      const n = norm(a.p_name);
      if (badName(n)) return this.err('name_invalid');
      if (!EMOJIS.includes(a.p_emoji)) return this.err('emoji_invalid');
      if (prof) return { data: { ...prof }, error: null };
      if (taken(n)) return this.err('name_taken');
      const p = { id: uid, name: n, emoji: a.p_emoji, xp: 0, games: 0, wins: 0, losses: 0, interrupted: 0, streak: 0, best_streak: 0, shots: 0, hits: 0, sunk: 0,
        upgrades: 0, fastest_win: null, time_played: 0, imported: false, last_game_at: null, created_at: new Date().toISOString() };
      this.profiles.set(uid, p); return { data: { ...p }, error: null };
    }
    if (!prof) return this.err('no_profile');
    if (name === 'update_profile') {
      const n = norm(a.p_name);
      if (badName(n)) return this.err('name_invalid');
      if (!EMOJIS.includes(a.p_emoji)) return this.err('emoji_invalid');
      if (taken(n, uid)) return this.err('name_taken');
      prof.name = n; prof.emoji = a.p_emoji; return { data: { ...prof }, error: null };
    }
    if (name === 'delete_account') { this.profiles.delete(uid); return { data: null, error: null }; }
    if (name === 'import_local') {
      if (prof.imported || prof.games > 0) return this.err('already_imported');
      const s = a.p_stats || {};
      const g = clamp(s.games, 0, 500), w = clamp(s.wins, 0, g), l = clamp(s.losses, 0, g - w), shots = clamp(s.shots, 0, 50000);
      Object.assign(prof, { xp: clamp(s.xp, 0, 5000), games: g, wins: w, losses: l, interrupted: clamp(s.interrupted, 0, 500),
        best_streak: clamp(s.bestStreak, 0, w), shots, hits: clamp(s.hits, 0, shots), sunk: clamp(s.sunk, 0, 5000), upgrades: clamp(s.upgrades, 0, 5000),
        fastest_win: clamp(s.fastestWin, 0, 400) || null, time_played: clamp(s.timePlayed, 0, 3600000), imported: true });
      return { data: { ...prof }, error: null };
    }
    if (name === 'record_game') {
      const sec = clamp(a.p_seconds, 0, 7200);
      if (a.p_interrupted) { prof.interrupted++; prof.time_played += sec; return { data: { gain: 0, profile: { ...prof } }, error: null }; }
      if (prof.last_game_at && Date.now() - prof.last_game_at < this.rateLimitMs) return { data: { gain: 0, limited: true, profile: { ...prof } }, error: null };
      const win = !!a.p_win, shots = clamp(a.p_shots, 0, 400), hits = clamp(a.p_hits, 0, shots), sunk = clamp(a.p_sunk, 0, 10), up = clamp(a.p_upgrades, 0, 20);
      const acc = shots ? hits / shots : 0;
      let gain = (win ? 100 : 30) + hits * 5 + sunk * 20 + (win ? Math.round(acc * 50) : 0);
      if (a.p_halved) gain = Math.round(gain / 2);
      gain = Math.min(gain, 400);
      Object.assign(prof, { xp: prof.xp + gain, games: prof.games + 1, wins: prof.wins + (win ? 1 : 0), losses: prof.losses + (win ? 0 : 1),
        streak: win ? prof.streak + 1 : 0, best_streak: Math.max(prof.best_streak, win ? prof.streak + 1 : 0), shots: prof.shots + shots, hits: prof.hits + hits,
        sunk: prof.sunk + sunk, upgrades: prof.upgrades + up, time_played: prof.time_played + sec, last_game_at: Date.now() });
      if (win && shots > 0 && (prof.fastest_win === null || shots < prof.fastest_win)) prof.fastest_win = shots;
      return { data: { gain, profile: { ...prof } }, error: null };
    }
    return this.err('fonction inconnue : ' + name);
  }
}
module.exports = { FakeBackend, EMOJIS };
