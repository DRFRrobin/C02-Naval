/* Simulateur de supabase-js côté navigateur (sous-ensemble utilisé par le jeu).
   Toutes les données et les salons Realtime vivent dans tests/fake-backend.js (Node). */
(() => {
  window.C02_CONFIG = { url: 'https://fake.supabase.test', key: 'fake-anon-key' };
  const call = (op, args) => window.__sbCall(op, args);
  const SESSION_KEY = 'fake.sb.session';
  const getSession = () => { try { return JSON.parse(localStorage.getItem(SESSION_KEY)); } catch (e) { return null; } };
  const uid = () => { const s = getSession(); return s && s.user && s.user.id; };
  const authCbs = [], channels = new Map();
  let dropped = false;
  const fire = (ev, s) => authCbs.slice().forEach(cb => setTimeout(() => cb(ev, s), 0));
  const thenable = fn => ({ then: (res, rej) => fn().then(res, rej), catch: rej => fn().catch(rej), finally: f => fn().finally(f) });

  class Channel {
    constructor(name, opts) {
      this.name = name; this.id = name + '#' + Math.random().toString(36).slice(2); this.key = opts && opts.config && opts.config.presence && opts.config.presence.key;
      this.h = { broadcast: [], presence: [] }; this.state = {}; this.cb = null;
    }
    on(type, filter, cb) { (this.h[type] = this.h[type] || []).push({ filter, cb }); return this; }
    subscribe(cb) {
      this.cb = cb; channels.set(this.id, this);
      if (dropped) setTimeout(() => cb && cb('CHANNEL_ERROR'), 0); else call('join', { id: this.id, name: this.name, key: this.key });
      return this;
    }
    async track(meta) { if (dropped) return 'error'; await call('track', { id: this.id, meta }); return 'ok'; }
    send({ payload }) { if (dropped || window.__sbBlackhole) return Promise.resolve('error'); return call('send', { id: this.id, payload }).then(() => 'ok'); }
    presenceState() { return this.state; }
  }
  window.__sbDeliver = (id, kind, data) => {
    const ch = channels.get(id); if (!ch) return;
    if (kind === 'status') { if (ch.cb) ch.cb(data); }
    else if (kind === 'presence') { ch.state = data; ch.h.presence.forEach(x => x.cb()); }
    else if (kind === 'broadcast') ch.h.broadcast.forEach(x => x.cb({ payload: data }));
  };
  // Simulation d'une coupure réseau : le serveur retire la présence, les canaux passent en erreur ; puis retour du réseau.
  window.__sbDrop = () => { dropped = true; for (const ch of channels.values()) { call('leave', { id: ch.id }); if (ch.cb) setTimeout(() => ch.cb('CHANNEL_ERROR'), 0); } };
  window.__sbRestore = () => { dropped = false; for (const ch of channels.values()) call('join', { id: ch.id, name: ch.name, key: ch.key }); };
  window.__sbBlackhole = false;

  const client = {
    auth: {
      async getSession() { return { data: { session: getSession() }, error: null }; },
      onAuthStateChange(cb) { authCbs.push(cb); return { data: { subscription: { unsubscribe() {} } } }; },
      async signInWithOAuth({ provider }) {
        const s = { user: { id: window.__oauthUser || 'user-' + provider }, provider };
        localStorage.setItem(SESSION_KEY, JSON.stringify(s)); fire('SIGNED_IN', s); return { error: null };
      },
      async signOut() { localStorage.removeItem(SESSION_KEY); fire('SIGNED_OUT', null); return { error: null }; },
    },
    from(table) {
      const q = { eqs: {} };
      const b = { select() { return b; }, eq(c, v) { q.eqs[c] = v; return b; }, maybeSingle() { return thenable(() => call('profileGet', { uid: q.eqs.id })); } };
      return b;
    },
    rpc(name, args) { return thenable(() => call('rpc', { name, args, uid: uid() })); },
    channel(name, opts) { return new Channel(name, opts); },
    removeChannel(ch) { channels.delete(ch.id); call('leave', { id: ch.id }); return Promise.resolve('ok'); },
  };
  window.supabase = { createClient: () => client };
})();
