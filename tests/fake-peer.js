/* Simulateur de PeerJS (en mémoire, via BroadcastChannel) pour tester le multijoueur sans réseau.
   Injecté dans chaque page avant le chargement du jeu. Prend en charge le sous-ensemble utilisé par le jeu :
   Peer(id?), on, connect, disconnect, reconnect, destroy, et les connexions de données (send/close/événements). */
(() => {
  const bc = new BroadcastChannel('fake-peerjs');
  const peers = new Map(), conns = new Map(), pendingClaims = new Map(), pendingConns = new Map();
  let seq = 0;
  const uid = Math.random().toString(36).slice(2);
  const mk = () => uid + '-' + (++seq);

  class Conn {
    constructor(peer, remoteId, cid, initiator) {
      Object.assign(this, { peer, peerId: remoteId, cid, initiator, open: false, closed: false, h: {} });
      conns.set(cid + ':' + (initiator ? 'a' : 'b'), this);
    }
    on(n, f) { (this.h[n] = this.h[n] || []).push(f); return this; }
    emit(n, ...a) { (this.h[n] || []).slice().forEach(f => f(...a)); }
    send(m) {
      if (!this.open || window.__blackhole) return;
      bc.postMessage({ k: 'd', cid: this.cid, to: this.initiator ? 'b' : 'a', m: JSON.parse(JSON.stringify(m)) });
    }
    close() { if (this.closed) return; this._closeLocal(); bc.postMessage({ k: 'c', cid: this.cid, to: this.initiator ? 'b' : 'a' }); }
    _closeLocal() { if (this.closed) return; this.closed = true; this.open = false; this.emit('close'); }
  }

  class Peer {
    constructor(a) {
      this.h = {}; this.destroyed = false; this.disconnected = false; this.open = false;
      const id = typeof a === 'string' ? a : null;
      this.id = id || 'anon-' + mk();
      setTimeout(() => this._register(!!id), 0);
    }
    on(n, f) { (this.h[n] = this.h[n] || []).push(f); return this; }
    emit(n, ...a) { (this.h[n] || []).slice().forEach(f => f(...a)); }
    _register(check) {
      if (this.destroyed) return;
      const finish = () => { if (this.destroyed) return; peers.set(this.id, this); this.disconnected = false; this.open = true; this.emit('open', this.id); };
      if (!check) return finish();
      const local = peers.get(this.id);
      if (local && local !== this && !local.disconnected && !local.destroyed) return this.emit('error', { type: 'unavailable-id' });
      const claimId = mk();
      const timer = setTimeout(() => { pendingClaims.delete(claimId); finish(); }, 150);
      pendingClaims.set(claimId, () => { clearTimeout(timer); pendingClaims.delete(claimId); if (!this.destroyed) this.emit('error', { type: 'unavailable-id' }); });
      bc.postMessage({ k: 'claim', id: this.id, claimId });
    }
    connect(id) {
      const cid = mk(), c = new Conn(this, id, cid, true);
      pendingConns.set(cid, setTimeout(() => { pendingConns.delete(cid); if (!c.open) { c.closed = true; this.emit('error', { type: 'peer-unavailable' }); } }, 500));
      bc.postMessage({ k: 'conn', to: id, from: this.id, cid });
      return c;
    }
    disconnect() {
      if (this.destroyed || this.disconnected) return;
      if (peers.get(this.id) === this) peers.delete(this.id);
      this.disconnected = true; this.open = false; this.emit('disconnected', this.id);
    }
    reconnect() { if (this.destroyed || !this.disconnected) throw new Error('not disconnected'); this._register(true); }
    destroy() {
      if (this.destroyed) return;
      this.destroyed = true; this.open = false;
      if (peers.get(this.id) === this) peers.delete(this.id);
      for (const c of [...conns.values()]) if (c.peer === this) c.close();
      this.emit('close');
    }
  }

  bc.onmessage = ev => {
    const m = ev.data;
    if (m.k === 'claim') { const p = peers.get(m.id); if (p && !p.disconnected && !p.destroyed) bc.postMessage({ k: 'taken', claimId: m.claimId }); }
    else if (m.k === 'taken') { const f = pendingClaims.get(m.claimId); if (f) f(); }
    else if (m.k === 'conn') {
      const p = peers.get(m.to);
      if (!p || p.disconnected || p.destroyed) return;
      const c = new Conn(p, m.from, m.cid, false);
      c.open = true; bc.postMessage({ k: 'connok', cid: m.cid });
      p.emit('connection', c); setTimeout(() => c.emit('open'), 0);
    } else if (m.k === 'connok') {
      const c = conns.get(m.cid + ':a'); if (!c) return;
      clearTimeout(pendingConns.get(m.cid)); pendingConns.delete(m.cid);
      c.open = true; setTimeout(() => c.emit('open'), 0);
    } else if (m.k === 'd') { const c = conns.get(m.cid + ':' + m.to); if (c && !c.closed) c.emit('data', m.m); }
    else if (m.k === 'c') { const c = conns.get(m.cid + ':' + m.to); if (c) c._closeLocal(); }
  };

  window.Peer = Peer;
  window.__dropAll = () => { for (const c of [...conns.values()]) if (!c.closed) c.close(); };   // simule une coupure réseau
  window.__blackhole = false;                                                                      // true : les messages sont perdus sans fermeture
})();
