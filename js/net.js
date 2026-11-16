'use strict';
/* =========================================================
   Réseau (PeerJS / WebRTC)
   ========================================================= */
function netErrorText(err) {
  switch (err && err.type) {
    case 'browser-incompatible': return t('errBrowser');
    case 'network': case 'server-error': case 'socket-error': case 'socket-closed': return t('errNet');
    default: return t('errNetOther', { type: (err && err.type) || '?' });
  }
}
function checkPeerLib(where) {
  if (typeof Peer === 'undefined') { msg(where, t('errPeerLib'), true); return false; }
  return true;
}
function keepPeerAlive(peer) {
  // Lien perdu avec le service de mise en relation (veille, réseau) : on se réenregistre
  // pour que le code reste valable tant que l'adversaire n'est pas arrivé.
  if (S.connected || peer.destroyed) return;
  msg('lobbyMsg', t('reconnecting'));
  setTimeout(() => { if (!S.connected && !peer.destroyed && peer.disconnected) peer.reconnect(); }, 1000);
}
function setupLobby(kind) {
  const host = kind === 'host';
  $('hostBox').style.display = host ? '' : 'none';
  $('lobbyIcon').className = 'mode-ic ' + (host ? 'ic-purple' : 'ic-cyan');
  $('lobbyIcon').innerHTML = `<i class="fa-solid ${host ? 'fa-tower-broadcast' : 'fa-shuffle'}"></i>`;
  $('lobbyTitle').textContent = t(host ? 'lobbyHostT' : 'lobbyRandomT');
  $('lobbyDesc').textContent = t(host ? 'lobbyHostD' : 'lobbyRandomD');
  $('lobbyCount').textContent = '';
  msg('lobbyMsg', t(host ? 'creating' : 'searching'));
  showView('lobby');
}
function acceptConn(conn) {
  if (S.connected && S.sid) return acceptResume(conn);   // partie en cours : seule une reprise de CETTE partie est acceptée
  if (S.conn || S.connected) {          // déjà un adversaire : on refuse poliment
    conn.on('open', () => { try { conn.send({ t: 'busy' }); } catch (e) {} setTimeout(() => conn.close(), 300); });
    return;
  }
  setupConn(conn, () => { S.conn = null; if (!S.connected) msg('lobbyMsg', t('waitingOpp')); });
}

// Salon privé
function hostGame() {
  if (!checkPeerLib('dashMsg')) return;
  S.kind = 'host'; S.isHost = true; S.code = genCode(); S.phase = 'lobby';
  applyConfig(RULES);
  setupLobby('host');
  $('codeBox').textContent = S.code;
  const peer = new Peer(PREFIX + S.code, peerOpts());
  S.peer = peer;
  peer.on('open', () => msg('lobbyMsg', t('waitingOpp')));
  peer.on('disconnected', () => keepPeerAlive(peer));
  peer.on('connection', acceptConn);
  peer.on('error', err => {
    if (err.type === 'unavailable-id') { peer.destroy(); hostGame(); return; }  // code déjà pris : on en tire un autre
    if (S.connected) return;
    msg('lobbyMsg', netErrorText(err), true);
  });
}

// Rejoindre avec un code
function openJoin(code) {
  $('codeInput').value = (code || '').toUpperCase().slice(0, 6);
  msg('joinMsg', '');
  $('btnJoin').disabled = false;
  $('joinModal').classList.add('show');
  setTimeout(() => $('codeInput').focus(), 50);
}
function joinGame() {
  if (S.peer) return;
  const code = $('codeInput').value.toUpperCase().replace(/\s+/g, '');
  if (code.length !== 6) return msg('joinMsg', t('errCodeLen'), true);
  if ([...code].some(ch => !ALPHA.includes(ch))) return msg('joinMsg', t('errCodeChars'), true);
  if (!checkPeerLib('joinMsg')) return;
  msg('joinMsg', t('connecting'));
  $('btnJoin').disabled = true;
  S.kind = 'join'; S.isHost = false;
  applyConfig(DEFAULT_RULES);
  const peer = new Peer(peerOpts());
  S.peer = peer; S.code = code;
  let failed = false;
  const fail = text => {
    clearTimeout(S.joinTimer);
    if (S.connected || failed) return;
    failed = true;
    try { peer.destroy(); } catch (e) {}
    S.peer = null; S.conn = null;
    $('btnJoin').disabled = false;
    msg('joinMsg', text, true);
  };
  peer.on('open', () => {
    setupConn(peer.connect(PREFIX + code, { reliable: true }), busy => fail(busy ? t('errFull') : t('errJoinTimeout')));
    S.joinTimer = setTimeout(() => fail(t('errJoinTimeout')), JOIN_TIMEOUT_MS);
  });
  peer.on('error', err => {
    if (S.connected) return;
    fail(err.type === 'peer-unavailable' ? t('errNoGame') : netErrorText(err));
  });
}

// Adversaire aléatoire : quelques « créneaux » publics. On essaie d'en occuper un (on attend),
// et s'il est déjà pris, quelqu'un y attend : on le rejoint. Bot au bout de 45 s.
function findRandom() {
  if (!checkPeerLib('dashMsg')) return;
  S.kind = 'random'; S.phase = 'lobby'; S.mmStopped = false;
  applyConfig(MATCH_RULES);
  setupLobby('random');
  const deadline = Date.now() + MM_WAIT_MS;
  const tick = () => {
    if (S.connected) { clearInterval(S.mmTimer); $('lobbyCount').textContent = ''; return; }
    const left = Math.max(0, Math.ceil((deadline - Date.now()) / 1000));
    $('lobbyCount').textContent = t('countdown', { s: left });
    if (left <= 0) {
      clearInterval(S.mmTimer);
      stopSearch();
      if (!settings.bot) { msg('lobbyMsg', t('noBotMsg'), true); setTimeout(() => location.reload(), 2500); return; }
      msg('lobbyMsg', t('noOneFound'), 'ok');
      setTimeout(() => startBot(true), 1000);
    }
  };
  tick(); S.mmTimer = setInterval(tick, 250);
  trySlot(0);
}
function stopSearch() {
  S.mmStopped = true;
  const c = S.conn, p = S.peer;
  S.conn = null; S.peer = null;
  try { if (c) c.close(); } catch (e) {}
  try { if (p) p.destroy(); } catch (e) {}
}
function trySlot(k) {
  if (S.connected || S.mmStopped) return;
  if (k >= MM_SLOTS) { setTimeout(() => trySlot(0), 1500); return; }
  const peer = new Peer(MM_PREFIX + k, peerOpts());
  S.peer = peer;
  peer.on('open', () => { if (S.mmStopped) return peer.destroy(); S.isHost = true; msg('lobbyMsg', t('waitingOpp')); });
  peer.on('connection', conn => { if (!S.mmStopped) acceptConn(conn); });
  peer.on('disconnected', () => keepPeerAlive(peer));
  peer.on('error', err => {
    if (S.connected || S.mmStopped || S.peer !== peer) return;
    if (err.type === 'unavailable-id') { peer.destroy(); joinSlot(k); return; }
    msg('lobbyMsg', netErrorText(err), true);
  });
}
function joinSlot(k) {
  if (S.connected || S.mmStopped) return;
  const peer = new Peer(peerOpts());
  S.peer = peer; S.isHost = false;
  let done = false, tm = null;
  const next = () => {
    if (done || S.connected || S.mmStopped) return;
    done = true; clearTimeout(tm);
    S.conn = null;
    try { peer.destroy(); } catch (e) {}
    trySlot(k + 1);
  };
  peer.on('open', () => {
    if (S.mmStopped) return peer.destroy();
    setupConn(peer.connect(MM_PREFIX + k, { reliable: true }), next);
    tm = setTimeout(() => { if (!S.connected) next(); }, 8000);
  });
  peer.on('error', () => { if (!S.connected) next(); });
}

// Connexion de données commune. La partie démarre quand chacun a reçu le « hello » de l'autre.
function setupConn(conn, onFail) {
  S.conn = conn; S.onReject = onFail || null;
  conn.on('open', () => {
    if (S.conn !== conn) return;
    const extra = {};
    if (S.isHost && S.kind !== 'join') {       // l'hôte annonce la partie : règles, identifiant de partie et point de reprise
      S.sid = S.sid || genCode() + genCode();
      S.rid = S.rid || (S.kind === 'host' ? PREFIX + S.code : PREFIX + genCode());
      Object.assign(extra, { sid: S.sid, rid: S.rid });
      if (S.kind === 'host') extra.rules = CFG;
    }
    send({ t: 'hello', ...myIdentity(), ...extra });
  });
  conn.on('data', m => { if (S.conn === conn) onData(m); });
  const closed = () => {
    if (S.conn !== conn) return;
    if (S.connected) connDown();
    else if (S.onReject) S.onReject(false);
  };
  conn.on('close', closed);
  conn.on('error', closed);
}
function startHeartbeat() {
  clearInterval(S.hb);
  S.lastSeen = Date.now();
  S.hb = setInterval(() => {
    send({ t: 'ping', a: S.rxn });
    if (S.reconnecting) { if (Date.now() > S.reconDeadline) lost(); return; }
    // Adversaire parti dans une autre appli ou écran éteint : on lui laisse plus de temps.
    if (Date.now() - S.lastSeen > (S.oppAway ? AWAY_TIMEOUT_MS : TIMEOUT_MS)) connDown();
  }, HEARTBEAT_MS);
}
function onConnected() {
  S.connected = true;
  clearTimeout(S.joinTimer); clearInterval(S.mmTimer);
  $('joinModal').classList.remove('show');
  startHeartbeat();
  // Créneau aléatoire : on le libère pour le prochain joueur (la liaison directe reste ouverte).
  // L'hôte ouvre en plus un point de rendez-vous privé pour une éventuelle reprise après coupure.
  if (S.kind === 'random' && S.peer) try { S.peer.disconnect(); } catch (e) {}
  if (S.kind === 'random' && S.isHost && S.rid && typeof Peer !== 'undefined') {
    try {
      const rp = new Peer(S.rid, peerOpts());
      S.rpeer = rp;
      rp.on('connection', acceptResume);
      rp.on('disconnected', () => { if (!rp.destroyed) setTimeout(() => { try { rp.reconnect(); } catch (e) {} }, 1000); });
      rp.on('error', () => {});
    } catch (e) {}
  }
  keepAwake();
  enterPlacement();
}

/* ---------- Envoi fiable : messages numérotés, renvoyés après une coupure ---------- */
function send(m) {
  try {
    if (S.vsBot) { if (S.bot) S.bot.receive(m); return; }
    if (SEQ.has(m.t)) { m.n = ++S.txn; S.outbox.push(m); }
    if (S.conn && S.conn.open && !S.reconnecting) S.conn.send(m);
  } catch (e) {}
}
function flushOutbox(theirRx) {
  S.outbox = S.outbox.filter(x => x.n > theirRx);
  for (const m of S.outbox) { try { S.conn.send(m); } catch (e) {} }
}

/* ---------- Reprise après coupure (45 s) ---------- */
function connDown() {
  if (S.phase === 'over' && S.connected && !S.lost && !S.vsBot) { S.oppGone = true; renderRematch(); return; }   // il a quitté l'écran de fin
  if (!S.connected || S.lost || S.phase === 'over' || S.vsBot || S.reconnecting) return;
  if (!S.sid || (!S.isHost && !S.hostId)) return lost();   // adversaire sans reprise possible
  S.reconnecting = true; S.reconDeadline = Date.now() + RECONNECT_MS;
  try { if (S.conn && S.conn.close) S.conn.close(); } catch (e) {}
  clearInterval(S.reconTimer); S.reconTimer = setInterval(reconTick, 3000);
  renderConn(); reconTick();
}
function reconTick() {
  if (!S.reconnecting) return;
  if (Date.now() > S.reconDeadline) return lost();
  if (S.isHost) {   // l'hôte reste joignable : on s'assure que son point de rendez-vous est bien enregistré
    const pr = S.rpeer || S.peer;
    try { if (pr && pr.disconnected && !pr.destroyed) pr.reconnect(); } catch (e) {}
  } else guestAttempt();
}
function guestAttempt() {
  if (S.attempting || typeof Peer === 'undefined') return;
  S.attempting = true;
  try { if (S.gpeer) S.gpeer.destroy(); } catch (e) {}
  let peer; try { peer = new Peer(peerOpts()); } catch (e) { S.attempting = false; return; }
  S.gpeer = peer;
  let iv = null;
  const end = () => { clearTimeout(to); clearInterval(iv); S.attempting = false; };
  const to = setTimeout(() => { try { peer.destroy(); } catch (e) {} end(); }, 8000);
  peer.on('error', end);
  peer.on('open', () => {
    const c = peer.connect(S.hostId, { reliable: true });
    const ask = () => { try { if (c.open) c.send({ t: 'resume', sid: S.sid, rx: S.rxn }); } catch (e) {} };
    c.on('open', () => { ask(); iv = setInterval(ask, 1500); });   // la demande est répétée tant qu'aucune réponse n'arrive
    c.on('data', m => { if (m && m.t === 'resume' && m.sid === S.sid && S.reconnecting) { end(); adoptConn(c, m.rx); } });
    c.on('close', end); c.on('error', end);
  });
}
function acceptResume(conn) {   // côté hôte : un invité demande à reprendre la partie
  let adopted = false;
  conn.on('data', m => {
    if (adopted) return;   // une fois adoptée, la connexion est gérée par adoptConn
    if (!m || m.t !== 'resume' || !S.sid || m.sid !== S.sid || !S.connected || S.lost || S.phase === 'over') { try { conn.close(); } catch (e) {} return; }
    adopted = true;
    try { conn.send({ t: 'resume', sid: S.sid, rx: S.rxn }); } catch (e) {}
    adoptConn(conn, m.rx);
  });
}
function adoptConn(conn, theirRx) {
  const old = S.conn; S.conn = conn;
  if (old && old !== conn) try { old.close(); } catch (e) {}
  conn.on('data', m => { if (S.conn === conn) onData(m); });
  const closed = () => { if (S.conn === conn) connDown(); };
  conn.on('close', closed); conn.on('error', closed);
  S.reconnecting = false; clearInterval(S.reconTimer);
  S.lastSeen = Date.now(); S.oppAway = false;
  flushOutbox(Number.isInteger(theirRx) ? theirRx : 0);
  renderConn();
}
function renderConn() {
  if (S.phase === 'game') renderGame();
  else if (S.phase === 'place') { if (S.reconnecting) msg('placeMsg', t('reconnecting')); else { msg('placeMsg', ''); renderPlace(); } }
}

function lost() {
  if (!S.connected || S.lost || S.phase === 'over') return;
  recordInterrupted();
  S.lost = true; S.phase = 'over'; S.reconnecting = false;
  clearInterval(S.hb); clearInterval(S.reconTimer); clearInterval(S.turnTimer);
  $('choice').classList.remove('show');
  $('endIcon').textContent = '📴';
  $('endTitle').textContent = t('lostTitle');
  $('endText').textContent = t('lostText');
  $('endXp').innerHTML = '';
  $('btnShare').style.display = 'none';   // partie interrompue : rien à partager
  renderRematch();
  $('end').classList.add('show');
  if ($('game').classList.contains('active')) renderGame();
}
function quitGame() {
  if (!confirm(t('quitConfirm'))) return;
  recordInterrupted();
  S.phase = 'over';
  if (!S.vsBot) { send({ t: 'quit' }); S.reconnecting = false; }
  if (S.bot) S.bot.stop();
  // Petit délai pour que le message d'abandon parte avant la fermeture du lien.
  setTimeout(() => {
    try { if (S.conn && S.conn.close) S.conn.close(); } catch (e) {}
    location.reload();
  }, S.vsBot ? 0 : 250);
}

/* Veille et arrière-plan */
async function keepAwake() {
  // Empêche l'écran de s'éteindre pendant la partie (navigateurs récents, en HTTPS ou localhost).
  try {
    if (!('wakeLock' in navigator) || S.wakeLock || document.visibilityState !== 'visible') return;
    S.wakeLock = await navigator.wakeLock.request('screen');
    S.wakeLock.addEventListener('release', () => { S.wakeLock = null; });
  } catch (e) { S.wakeLock = null; }
}
function onVisibility() {
  if (!S.connected || S.vsBot || S.lost || S.phase === 'over') return;
  if (document.visibilityState === 'hidden') { send({ t: 'away' }); return; }
  // De retour : nos minuteurs ont pu être ralentis, on repart d'un délai neuf avant de juger la liaison.
  S.lastSeen = Date.now();
  send({ t: 'back' });
  keepAwake();
}
function showAway() {
  if (S.phase === 'game') return renderHint();
  if (S.phase === 'place') {
    if (S.oppAway) msg('placeMsg', t('oppAway'));
    else renderPlace();
  }
}

function onData(m) {
  S.lastSeen = Date.now();
  if (!m || typeof m !== 'object' || S.lost) return;
  if (Number.isInteger(m.n)) {            // message numéroté : on ignore les doublons (renvoi après coupure)
    if (m.n <= S.rxn) return;
    S.rxn = m.n;
  }
  switch (m.t) {
    case 'ping': if (Number.isInteger(m.a)) S.outbox = S.outbox.filter(x => x.n > m.a); break;
    case 'hello':
      S.opp = {
        name: String(m.name || '?').slice(0, 16),
        emoji: validEmoji(m.emoji) ? m.emoji : null,
        level: Math.max(0, Math.min(999, parseInt(m.level, 10) || 0)),
        bot: !!m.bot && S.vsBot,
      };
      if (!S.connected && !S.isHost && !S.vsBot && typeof m.sid === 'string' && typeof m.rid === 'string' && m.sid.length <= 40 && m.rid.length <= 60) { S.sid = m.sid; S.hostId = m.rid; }
      if (!S.connected && S.kind === 'join') applyConfig(sanitizeRules(m.rules) || DEFAULT_RULES);
      if (!S.connected) onConnected(); else renderVs();
      break;
    case 'busy': if (!S.connected && S.onReject) S.onReject(true); break;
    case 'rematch': S.rematchOpp = true; if (S.phase === 'over') { if (S.rematchMe) startRematch(); else renderRematch(); } break;
    case 'bye': S.oppGone = true; if (S.phase === 'over') renderRematch(); break;
    case 'away': S.oppAway = true; showAway(); break;
    case 'back': S.oppAway = false; showAway(); break;
    case 'ready':
      S.oppCommit = typeof m.h === 'string' && /^[0-9a-f]{64}$/.test(m.h) ? m.h : null;   // empreinte de sa grille (absente : ancienne version)
      S.oppReady = true; renderPlace(); maybeStart(); break;
    case 'reveal': S.oppReveal = m; if (S.phase === 'over' && S.verify === 'pending') runVerify(); break;
    case 'act': onAct(m); break;
    case 'res': onRes(m); break;
    case 'pass': onPass(); break;
    case 'quit':
      if (S.phase === 'game') endGame(true, true);
      else lost();
      break;
  }
}
