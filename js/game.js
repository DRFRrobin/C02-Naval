'use strict';
/* =========================================================
   Partie
   ========================================================= */
function sanitizeCells(list) {
  return (Array.isArray(list) ? list : [])
    .filter(p => Array.isArray(p) && Number.isInteger(p[0]) && Number.isInteger(p[1]) && inside(p[0], p[1]))
    .slice(0, 5);
}
// Normalise une action reçue : un tir = 1 case, un radar = carré 2 × 2, une frappe en croix = 5 cases.
// Une amélioration non gagnée est ramenée à un simple tir (anti-triche et robustesse).
function normalizeAct(m, canUpgrade) {
  const raw = sanitizeCells(m.cells);
  const okPt = Array.isArray(m.at) && Number.isInteger(m.at[0]) && Number.isInteger(m.at[1]) && inside(m.at[0], m.at[1]);
  const at = okPt ? [m.at[0], m.at[1]] : (raw[0] || null);
  if (!at) return null;
  let kind = ['shot', 'cross', 'radar'].includes(m.kind) ? m.kind : 'shot';
  if (kind !== 'shot' && (!canUpgrade || !upgradeOn(kind))) kind = 'shot';
  const [r, c] = at;
  const allowed = kind === 'radar' ? [[r, c], [r, c + 1], [r + 1, c], [r + 1, c + 1]]
    : kind === 'cross' ? [[r, c], [r - 1, c], [r + 1, c], [r, c - 1], [r, c + 1]] : [[r, c]];
  let cells = raw.filter(([a, b]) => allowed.some(([x, y]) => x === a && y === b));
  cells = cells.filter((p, i) => cells.findIndex(q => q[0] === p[0] && q[1] === p[1]) === i);
  if (!cells.length) cells = [at];
  return { kind, at, cells };
}
// Calcule le résultat de tirs sur une flotte (la sienne ou celle du bot).
function resolveOn(ships, incoming, kind, cells) {
  const occ = occOf(ships), results = [];
  for (const [r, c] of cells) {
    const k = r * N + c, i = occ[k];
    if (kind === 'radar') { results.push({ r, c, v: i >= 0 ? 'occ' : 'empty' }); continue; }
    if (incoming[k] === 'miss' || incoming[k] === 'hit') { results.push({ r, c, v: incoming[k] }); continue; }
    if (i < 0) { incoming[k] = 'miss'; results.push({ r, c, v: 'miss' }); continue; }
    const s = ships[i];
    if (s.shield === 'active') { s.shield = 'used'; incoming[k] = 'blocked'; results.push({ r, c, v: 'blocked' }); continue; }
    incoming[k] = 'hit';
    results.push(sunkOf(s, incoming) ? { r, c, v: 'sunk', ship: s.cells, si: i } : { r, c, v: 'hit' });
  }
  return { results, over: ships.every(s => sunkOf(s, incoming)) };
}

/* ---------- Chronomètre de tour (règle facultative) ---------- */
function startTurnTimer() {
  clearInterval(S.turnTimer);
  if (!CFG.timer) return;
  S.turnTimer = setInterval(() => {
    if (S.phase !== 'game') return clearInterval(S.turnTimer);
    if (!S.myTurn || S.waiting || S.reconnecting || $('choice').classList.contains('show')) { if (S.myTurn) S.turnStart = Date.now(); return renderTimer(); }
    if (Date.now() - S.turnStart >= CFG.timer * 1000) return autoShot();
    renderTimer();
  }, 500);
}
function renderTimer() {
  if (S.phase !== 'game' || !CFG.timer || !S.myTurn || S.waiting || S.reconnecting) return;
  const left = Math.max(0, Math.ceil(CFG.timer - (Date.now() - S.turnStart) / 1000));
  $('status').textContent = t('stMyTurn') + ' · ' + left + ' s';
}
function autoShot() {   // temps écoulé : tir au hasard sur une case encore inconnue
  const free = []; for (let k = 0; k < N * N; k++) if (S.enemy[k] === null) free.push(k);
  if (!free.length) return;
  const k = free[Math.floor(Math.random() * free.length)];
  S.mode = 'shot'; S.aim = null;
  flashHint(t('autoShot'));
  onEnemyTap(Math.floor(k / N), k % N);
}

/* ---------- Mes actions ---------- */
function onEnemyTap(r, c) {
  if (S.phase !== 'game') return;
  if (!S.myTurn || S.waiting) return flashHint(t(S.waiting ? 'fFiring' : 'fWaitTurn'));
  if (S.mode === 'shield') return flashHint(t('fShieldHere'));
  let cells;
  if (S.mode === 'shot') {
    if (!targetable(r * N + c)) return flashHint(t('fAlready'));
    cells = [[r, c]];
  } else {   // radar ou frappe en croix : un premier appui vise, un second sur la même case confirme
    cells = aimCells(S.mode, r, c);
    if (!cells.length) return flashHint(t(S.mode === 'radar' ? 'fNoUnknown' : 'fAllKnown'));
    if (!S.aim || S.aim[0] !== r || S.aim[1] !== c) { S.aim = [r, c]; Snd.play('click'); return renderGame(); }
    S.inv[S.mode]--; S.match.upgrades++;
  }
  S.aim = null;
  S.pendingAct = { kind: S.mode, at: [r, c] };
  Snd.play(S.mode === 'radar' ? 'radar' : 'shot');
  send({ t: 'act', kind: S.mode, at: [r, c], cells });
  S.waiting = true; S.mode = 'shot';
  renderGame();
}
// Cases visées par un radar (carré 2 × 2, cases inconnues) ou une frappe en croix (5 cases).
function aimCells(mode, r, c) {
  if (mode === 'radar') return [[r, c], [r, c + 1], [r + 1, c], [r + 1, c + 1]].filter(([a, b]) => inside(a, b) && S.enemy[a * N + b] === null);
  if (mode === 'cross') return [[r, c], [r - 1, c], [r + 1, c], [r, c - 1], [r, c + 1]].filter(([a, b]) => inside(a, b) && targetable(a * N + b));
  return [[r, c]];
}
function onMyTap(r, c) {
  if (S.phase !== 'game' || S.mode !== 'shield' || !S.myTurn || S.waiting) return;
  const idx = occupancy()[r * N + c];
  if (idx < 0) return flashHint(t('fTouchShip'));
  protectShip(idx);
}
const canShield = s => !!s.cells && !isSunk(s) && s.shield === 'none';
function protectShip(idx) {
  if (S.phase !== 'game' || S.mode !== 'shield' || !S.myTurn || S.waiting) return;
  const s = S.ships[idx];
  if (!s) return;
  if (isSunk(s)) return flashHint(t('fShipSunk'));
  if (s.shield !== 'none') return flashHint(t('fHasShield'));
  s.shield = 'active';
  S.inv.shield--; S.match.upgrades++;
  S.mode = 'shot'; S.myTurn = false;
  send({ t: 'pass' });
  Snd.play('shield');
  setLog(t('shieldPlaced', { ship: shipName(idx) }));
  renderGame();
}
function setMode(m) {
  if (S.phase !== 'game' || !S.myTurn || S.waiting) return;
  if (m !== 'shot' && !S.inv[m]) return;
  S.mode = S.mode === m ? 'shot' : m;
  S.aim = null;
  Snd.play('click');
  renderGame();
}

/* ---------- Réponses ---------- */
function summarize(kind, at, results) {
  const cell = cellName(...at);
  if (kind === 'radar') return t('sumRadar', { cell, n: results.filter(x => x.v === 'occ').length, total: results.length });
  const sunkIdx = results.filter(x => x.v === 'sunk').map(x => x.si);
  const sunkNames = sunkIdx.map(i => (Number.isInteger(i) && i >= 0 && i < LENS.length ? shipName(i) : '?'));
  if (kind === 'shot') {
    const v = results[0] && results[0].v;
    return t('sumShot', { cell, v, ship: sunkNames[0] || '' });
  }
  return t('sumCross', {
    cell, hits: results.filter(x => x.v === 'hit' || x.v === 'sunk').length, miss: results.filter(x => x.v === 'miss').length,
    blk: results.filter(x => x.v === 'blocked').length, sunk: sunkNames.join(t('and')), sunkN: sunkNames.length,
  });
}
function playResultSound(results) {
  if (results.some(x => x.v === 'sunk')) { Snd.play('sunk'); Snd.buzz([80, 40, 140]); }
  else if (results.some(x => x.v === 'hit')) { Snd.play('hit'); Snd.buzz(50); }
  else if (results.some(x => x.v === 'blocked')) Snd.play('blocked');
  else if (results.some(x => x.v === 'miss')) Snd.play('miss');
}

// Le défenseur calcule le résultat sur sa propre grille.
function onAct(m) {
  if (S.phase !== 'game' || S.myTurn || S.waiting) return;  // ce n'est pas le tour de l'adversaire
  const a = normalizeAct(m, S.oppUsed < S.ships.filter(isSunk).length);
  if (!a) return;
  const { kind, cells, at } = a;
  if (kind !== 'shot') S.oppUsed++;
  const { results, over } = resolveOn(S.ships, S.incoming, kind, cells);
  send({ t: 'res', kind, at, results, over });
  S.lastIn = kind === 'radar' ? [] : cells.map(([r, c]) => r * N + c);
  setLog(t('oppPrefix', { name: S.opp ? S.opp.name : '?' }) + summarize(kind, at, results), true);
  if (kind === 'radar') Snd.play('radar'); else playResultSound(results);
  if (over) return endGame(false);
  S.myTurn = true; S.waiting = false; S.turnStart = Date.now();
  setTimeout(() => { if (S.myTurn && S.phase === 'game') Snd.play('turn'); }, 500);
  renderGame();
}

// L'attaquant reçoit le résultat.
function onRes(m) {
  if (S.phase !== 'game' || !S.waiting) return;
  const results = (Array.isArray(m.results) ? m.results : []).filter(x => x && inside(x.r, x.c));
  const act = S.pendingAct || { kind: m.kind, at: m.at };
  let sunk = 0;
  for (const x of results) {
    if (x.v === 'sunk') {
      (Array.isArray(x.ship) ? x.ship : [[x.r, x.c]]).forEach(p => { if (Array.isArray(p) && inside(p[0], p[1])) S.enemy[p[0] * N + p[1]] = 'sunk'; });
      if (Number.isInteger(x.si) && x.si >= 0 && x.si < LENS.length && !S.enemySunk.includes(x.si)) S.enemySunk.push(x.si);
      sunk++;
    } else {
      const st = { miss: 'miss', hit: 'hit', blocked: 'blocked', occ: 'scanOcc', empty: 'scanEmpty' }[x.v];
      if (st) S.enemy[x.r * N + x.c] = st;
    }
  }
  if (act.kind !== 'radar') {
    S.match.shots += results.length;
    S.match.hits += results.filter(x => x.v === 'hit' || x.v === 'sunk').length;
    S.match.sunk += sunk;
    playResultSound(results);
  }
  setLog(summarize(act.kind, act.at, results));
  S.waiting = false; S.pendingAct = null;
  if (m.over) return endGame(true);
  S.myTurn = false;
  if (anyUpgrade()) S.choices += sunk;
  renderGame();
  if (S.choices > 0) setTimeout(openChoice, 500);
}
function onPass() {
  if (S.phase !== 'game' || S.myTurn || S.waiting) return;
  if (!CFG.shield || S.oppUsed >= S.ships.filter(isSunk).length) return;   // bouclier non gagné ou désactivé : ignoré
  S.oppUsed++;
  S.myTurn = true; S.turnStart = Date.now();
  S.lastIn = [];
  setLog(t('oppShield', { name: S.opp ? S.opp.name : '?' }), true);
  Snd.play('turn');
  renderGame();
}

/* ---------- Améliorations ---------- */
const UPG = { radar: ['fa-satellite-dish', 'upRadarN', 'upRadarD'], cross: ['fa-plus', 'upCrossN', 'upCrossD'], shield: ['fa-shield-halved', 'upShieldN', 'upShieldD'] };
function openChoice() {
  if (S.phase !== 'game' || S.choices <= 0) { $('choice').classList.remove('show'); return; }
  $('choiceSub').textContent = S.choices > 1 ? t('choiceSubN', { n: S.choices }) : t('choiceSub');
  const list = $('choiceList'); list.innerHTML = '';
  for (const [k, [icon, nk, dk]] of Object.entries(UPG).filter(([k]) => upgradeOn(k))) {
    const b = document.createElement('button');
    b.className = 'opt';
    b.innerHTML = `<strong><i class="fa-solid ${icon}" style="color:var(--yellow);margin-right:8px"></i>${esc(t(nk))}</strong><span>${esc(t(dk))}</span>`;
    b.onclick = () => { S.inv[k]++; S.choices--; Snd.play('upgrade'); renderGame(); openChoice(); };
    list.appendChild(b);
  }
  $('choice').classList.add('show');
}

/* ---------- Fin de partie ---------- */
function endGame(win, forfeit) {
  S.phase = 'over'; S.reconnecting = false;
  clearInterval(S.hb); clearInterval(S.reconTimer); clearInterval(S.turnTimer);
  S.myTurn = false; S.waiting = false; S.aim = null;
  if (S.bot) S.bot.stop();
  $('choice').classList.remove('show');
  $('endIcon').textContent = win ? '🏆' : '🌊';
  $('endTitle').textContent = t(win ? 'win' : 'lose');
  const acc = S.match.shots ? Math.round(S.match.hits / S.match.shots * 100) : 0;
  $('endText').textContent = t(forfeit ? 'winForfeit' : win ? 'winText' : 'loseText') + ' ' + t('endStats', { shots: S.match.shots, acc });
  // Anti-triche : on dévoile sa grille, et on vérifie celle de l'adversaire s'il s'y était engagé.
  if (!S.vsBot && !forfeit) sendReveal();
  S.verify = !S.vsBot && !forfeit && S.oppCommit ? 'pending' : 'none';
  renderVerify();
  if (S.verify === 'pending') {
    S.win = win;
    clearTimeout(S.verifyTimer);
    S.verifyTimer = setTimeout(() => finishVerify('missing'), VERIFY_MS);
    if (!win) { S.recordPending = true; $('endXp').innerHTML = ''; }   // défaite : comptée seulement si la grille adverse est honnête
    if (S.oppReveal) runVerify();
  }
  if (!S.recordPending) showXp(win);
  renderRematch();
  setTimeout(() => Snd.play(win ? 'win' : 'lose'), 600);
  setTimeout(() => $('end').classList.add('show'), 700);
  renderGame();
}
function showXp(win) {
  const r = recordGame(win);
  if (r) {
    const a = me(), l0 = r.levelBefore, lo0 = xpFor(l0), hi0 = xpFor(l0 + 1), l = r.level, lo = xpFor(l), hi = xpFor(l + 1);
    const half = S.vsBot || !isStdRules(CFG);
    const rows = [['xpBase' + (win ? 1 : 0), r.parts.base], ['xpHits', r.parts.hits], ['xpSunk', r.parts.sunk], ['xpAcc', r.parts.acc], ['xpTime', r.parts.time]];
    $('endXp').innerHTML = `<ul class="xplist">${rows.map(([k, v], i) => `<li class="${v ? '' : 'zero'}" style="animation-delay:${0.9 + i * 0.18}s"><span>${esc(t(k))}</span><b>${v ? '+' + v : '—'}</b></li>`).join('')}</ul>`
      + `<div class="xpgain" id="xpGainN">+0 XP</div><div id="lvlBox"></div>`
      + (S.vsBot ? `<div class="endnote">${esc(t('botXp'))}</div>` : !isStdRules(CFG) ? `<div class="endnote">${esc(t('customXp'))}</div>` : '')
      + `<div class="xpbar" id="endBar"><i id="endFill" style="width:${Math.round((r.xpBefore - lo0) / (hi0 - lo0) * 100)}%"></i></div>`
      + `<div class="xprow"><span>XP: <b id="endXpTxt">${r.xpBefore - lo0} / ${hi0 - lo0}</b></span><span id="endLvl">${esc(t('levelN', { n: l0 }))}</span></div>`;
    animateXp(r, { l0, lo0, hi0, l, lo, hi, a, half });
  } else $('endXp').innerHTML = `<p class="endnote">${esc(t('xpGuest'))}</p>`;
}

/* ---------- Anti-triche ----------
   Chaque joueur décide seul si un tir l'a touché : un client modifié pourrait mentir.
   Au moment de se déclarer prêt, il envoie donc l'empreinte SHA-256 de sa grille (avec un sel aléatoire) :
   il ne peut plus la changer, et l'adversaire n'apprend rien. En fin de partie, chacun dévoile sa grille et son sel ;
   l'autre vérifie l'empreinte, que la flotte est valide et que chaque résultat annoncé (raté, touché, coulé, radar)
   correspond bien à cette grille. Grille truquée ou non dévoilée : la défaite n'est pas comptée.
   Un adversaire d'une ancienne version (sans empreinte) n'est simplement pas vérifié. */
const VERIFY_MS = 10000;
const hexOf = buf => Array.from(new Uint8Array(buf), b => b.toString(16).padStart(2, '0')).join('');
const canCommit = () => !!(window.crypto && crypto.subtle && crypto.getRandomValues);
async function boardHash(salt, ships) {
  const board = ships.map(cells => cells.map(([r, c]) => r * N + c).join(',')).join(';');
  return hexOf(await crypto.subtle.digest('SHA-256', new TextEncoder().encode(`c02-naval-v1|${N}|${LENS.join(',')}|${salt}|${board}`)));
}
function sendReady() {
  if (S.vsBot || !canCommit()) return send({ t: 'ready' });
  const round = S.rounds || 0;
  const salt = hexOf(crypto.getRandomValues(new Uint8Array(16)));
  const ships = S.ships.map(x => x.cells.map(([r, c]) => [r, c]));
  boardHash(salt, ships).then(h => {
    if ((S.rounds || 0) !== round || !S.meReady) return;
    S.myCommit = { salt, ships, h };
    send({ t: 'ready', h });
  }, () => send({ t: 'ready' }));
}
function sendReveal() {
  if (S.myCommit && !S.revealed) { S.revealed = true; send({ t: 'reveal', salt: S.myCommit.salt, ships: S.myCommit.ships }); }
}
// Contrôle de la grille dévoilée par l'adversaire : 'ok' ou 'cheat'.
async function checkReveal(m) {
  try {
    if (!m || typeof m.salt !== 'string' || !/^[0-9a-f]{32}$/.test(m.salt) || !Array.isArray(m.ships) || m.ships.length !== LENS.length) return 'cheat';
    const occ = new Map();
    const ships = [];
    for (let i = 0; i < LENS.length; i++) {
      const raw = m.ships[i];
      if (!Array.isArray(raw) || raw.length !== LENS[i]) return 'cheat';
      const cells = raw.map(p => (Array.isArray(p) && Number.isInteger(p[0]) && Number.isInteger(p[1]) && inside(p[0], p[1]) ? [p[0], p[1]] : null));
      if (cells.includes(null)) return 'cheat';
      const horiz = cells.every(p => p[0] === cells[0][0]), vert = cells.every(p => p[1] === cells[0][1]);
      if (!horiz && !vert) return 'cheat';
      const pos = cells.map(p => (horiz ? p[1] : p[0])).sort((a, b) => a - b);
      if (pos.some((v, j) => j && v !== pos[j - 1] + 1)) return 'cheat';   // cases alignées et contiguës
      for (const [r, c] of cells) { const k = r * N + c; if (occ.has(k)) return 'cheat'; occ.set(k, i); }
      ships.push(cells);
    }
    if (await boardHash(m.salt, ships) !== S.oppCommit) return 'cheat';
    for (let k = 0; k < N * N; k++) {
      const v = S.enemy[k], has = occ.has(k);
      if ((v === 'miss' || v === 'scanEmpty') && has) return 'cheat';
      if ((v === 'hit' || v === 'sunk' || v === 'blocked' || v === 'scanOcc') && !has) return 'cheat';
    }
    // un bateau annoncé coulé l'est en entier
    if (ships.some(cells => cells.some(([r, c]) => S.enemy[r * N + c] === 'sunk') && !cells.every(([r, c]) => S.enemy[r * N + c] === 'sunk'))) return 'cheat';
    return 'ok';
  } catch (e) { return 'cheat'; }
}
function runVerify() {
  const round = S.rounds || 0;
  checkReveal(S.oppReveal).then(res => { if ((S.rounds || 0) === round) finishVerify(res); });
}
function finishVerify(res) {
  if (S.verify !== 'pending') return;
  clearTimeout(S.verifyTimer);
  S.verify = res;
  if (S.recordPending) {
    S.recordPending = false;
    if (res === 'ok') showXp(false);
    else { recordInterrupted(true); $('endXp').innerHTML = ''; }
  }
  renderVerify();
}
function renderVerify() {
  const el = $('verifyNote'), name = S.opp ? S.opp.name : '?';
  const [text, cls] = S.verify === 'pending' ? [S.recordPending ? t('verifyWait') : '', 'wait']
    : S.verify === 'ok' ? [t('verifyOk'), 'ok']
    : S.verify === 'cheat' ? [t(S.win ? 'verifyCheatWin' : 'verifyCheat', { name }), 'bad']
    : S.verify === 'missing' && !S.win ? [t('verifyMissing', { name }), 'bad'] : ['', ''];
  el.textContent = text; el.className = 'verify' + (cls ? ' ' + cls : '');
}
// Défaite en attente de vérification quand on quitte l'écran de fin : elle est comptée.
function flushPendingLoss() {
  if (!S.recordPending) return;
  S.recordPending = false; S.verify = 'none'; clearTimeout(S.verifyTimer);
  recordGame(false);
}

// Animation de fin de partie : compteur d'XP, barre qui se remplit, passage de niveau.
function animateXp(r, d) {
  const t0 = Date.now() + 1900, fill = $('endFill'), bar = $('endBar');
  const num = $('xpGainN');
  const tick = () => {
    const k = clamp01((Date.now() - t0) / 900);
    if (!num.isConnected) return;
    num.textContent = '+' + Math.round(r.gain * (1 - Math.pow(1 - k, 3))) + ' XP';
    if (k < 1) requestAnimationFrame(tick);
  };
  setTimeout(() => { Snd.play('xp'); requestAnimationFrame(tick); }, 1900);
  const txt = (xp, lo, hi) => { const e = $('endXpTxt'); if (e) e.textContent = `${xp - lo} / ${hi - lo}`; };
  const pct = (xp, lo, hi) => Math.round((xp - lo) / (hi - lo) * 100) + '%';
  setTimeout(() => {
    if (!fill.isConnected) return;
    if (r.up) {
      fill.style.width = '100%'; txt(d.hi0, d.lo0, d.hi0);
      setTimeout(() => {
        if (!fill.isConnected) return;
        bar.classList.add('flash'); Snd.play('levelup');
        $('lvlBox').innerHTML = `<div class="lvlup">⬆ ${esc(t('levelUp', { n: d.l, rank: rankOf(d.l) }))}</div>`;
        $('endLvl').textContent = t('levelN', { n: d.l });
        fill.classList.add('snap'); fill.style.width = '0%'; void fill.offsetWidth; fill.classList.remove('snap');
        fill.style.width = pct(d.a.xp, d.lo, d.hi); txt(d.a.xp, d.lo, d.hi);
      }, 1000);
    } else { fill.style.width = pct(d.a.xp, d.lo, d.hi); txt(d.a.xp, d.lo, d.hi); }
  }, 2000);
}

/* ---------- Revanche ---------- */
function resetRound() {
  clearInterval(S.turnTimer);
  S.ships = LENS.map((len, i) => ({ i, len, cells: null, shield: 'none' }));
  Object.assign(S, { sel: 0, horiz: true, preview: null, lifted: null, drag: null, meReady: false, oppReady: false, myTurn: false, waiting: false, mode: 'shot',
    enemy: Array(N * N).fill(null), incoming: Array(N * N).fill(null), enemySunk: [], inv: { radar: 0, cross: 0, shield: 0 }, choices: 0, lastIn: [],
    pendingAct: null, oppUsed: 0, match: { shots: 0, hits: 0, sunk: 0, upgrades: 0, start: 0 }, rematchMe: false, rematchOpp: false,
    aim: null, myCommit: null, oppCommit: null, oppReveal: null, revealed: false, verify: 'none', win: false });
  clearTimeout(S.verifyTimer);
  $('verifyNote').textContent = '';
  msg('placeMsg', '');
}
function renderRematch() {
  const b = $('btnRematch'), n = $('rematchNote'), nm = S.opp ? S.opp.name : '';
  if (S.lost || !S.connected) { b.style.display = 'none'; n.textContent = ''; return; }
  b.style.display = '';
  b.disabled = !!S.oppGone || !!S.rematchMe;
  n.textContent = S.oppGone ? t('rematchGone') : S.rematchMe ? t('rematchWait', { name: nm }) : S.rematchOpp ? t('rematchOpp', { name: nm }) : '';
}
function askRematch() {
  if (S.phase !== 'over' || S.lost || S.oppGone || S.rematchMe) return;
  if (S.vsBot) return startRematch();
  S.rematchMe = true; send({ t: 'rematch' }); renderRematch(); Snd.play('click');
  if (S.rematchOpp) startRematch();
}
function startRematch() {
  if (S.phase !== 'over' || S.lost) return;
  $('end').classList.remove('show');
  flushPendingLoss();
  S.rounds = (S.rounds || 0) + 1;      // le premier tir alterne d'une manche à l'autre
  resetRound();
  if (S.vsBot) S.bot = new Bot(); else startHeartbeat();
  enterPlacement();
  if (S.vsBot) S.bot.start();
}
function leaveToMenu() {
  flushPendingLoss();
  if (S.connected && !S.vsBot && !S.lost) send({ t: 'bye' });
  setTimeout(() => location.reload(), 120);
}

/* ---------- Affichage de la partie ---------- */
let hintTimer = null;
function flashHint(text) {
  const h = $('hint'); h.textContent = text; h.style.color = '#fca5a5';
  clearTimeout(hintTimer);
  hintTimer = setTimeout(() => { h.style.color = ''; renderHint(); }, 1800);
}
function renderHint() {
  if (S.phase === 'game' && S.oppAway) { $('hint').textContent = t('oppAway'); return; }
  const key = { shot: 'hintShot', radar: 'hintRadar', cross: 'hintCross', shield: 'hintShield' }[S.mode];
  const live = S.phase === 'game' && S.myTurn && !S.waiting;
  $('hint').textContent = !live ? '' : S.aim ? t('hintConfirm', { cell: cellName(...S.aim) }) : t(key);
}
function setLog(text, fromOpp) {
  const l = $('log'); l.textContent = text; l.className = 'log' + (fromOpp ? ' opp' : '');
}
function renderGame() {
  const live = S.phase === 'game' && S.myTurn && !S.waiting;
  const st = $('status');
  if (S.lost) { st.textContent = t('stLost'); st.className = 'status'; }
  else if (S.reconnecting) { st.textContent = t('stReconnecting'); st.className = 'status wait'; }
  else if (S.phase === 'over') { st.textContent = t('stOver'); st.className = 'status'; }
  else if (S.waiting) { st.textContent = t('stFiring'); st.className = 'status wait'; }
  else if (S.myTurn) { st.textContent = t('stMyTurn'); st.className = 'status me'; }
  else { st.textContent = t('stOppTurn'); st.className = 'status'; }

  const SYM = { miss: '•', hit: '✕', sunk: '✕', blocked: '🛡', scanOcc: '?', scanEmpty: '·' };
  const eg = $('enemyGrid');
  eg.classList.toggle('live', live && S.mode !== 'shield');
  if (!live || (S.mode !== 'radar' && S.mode !== 'cross')) S.aim = null;
  const aimed = new Set(S.aim ? aimCells(S.mode, ...S.aim).map(([r, c]) => r * N + c) : []);
  eg.cells.forEach((d, k) => {
    const v = S.enemy[k];
    d.className = 'cell' + (v ? ' ' + v : '') + (targetable(k) ? ' target' : '') + (aimed.has(k) ? ' aim' : '');
    d.textContent = v ? SYM[v] : '';
  });

  const occ = occupancy(), mg = $('myGrid');
  mg.classList.toggle('pick', live && S.mode === 'shield');
  mg.cells.forEach((d, k) => {
    const i = occ[k], v = S.incoming[k];
    let cls = 'cell';
    if (i >= 0) cls += isSunk(S.ships[i]) ? ' sunk' : ' ship';
    if (i >= 0 && S.ships[i].shield === 'active') cls += ' shielded';
    if (v === 'hit' && !(i >= 0 && isSunk(S.ships[i]))) cls += ' hit';
    if (v === 'miss') cls += ' miss';
    if (v === 'blocked') cls += ' blocked';
    if (S.lastIn.includes(k)) cls += ' last';   // dernier tir adverse
    d.className = cls;
    d.textContent = v ? SYM[v] : '';
  });

  $('enemyFleet').innerHTML = LENS.map((_, i) => `<span class="${S.enemySunk.includes(i) ? 'down' : ''}">${esc(shipName(i))}</span>`).join('');

  document.querySelectorAll('#modes button').forEach(b => {
    const m = b.dataset.mode;
    if (m !== 'shot') b.querySelector('b').textContent = S.inv[m] || '';
    b.style.display = m === 'shot' || upgradeOn(m) ? '' : 'none';
    b.disabled = !live || (m !== 'shot' && !S.inv[m]);
    b.classList.toggle('on', live && S.mode === m);
  });
  // Bouclier : un bouton par bateau protégeable, à côté des actions (inutile de chercher sa flotte plus bas)
  const sp = $('shieldPick'), pick = live && S.mode === 'shield';
  sp.classList.toggle('show', pick);
  sp.innerHTML = pick ? S.ships.map((x, i) => (canShield(x) ? `<button type="button" data-ship="${i}"><i class="fa-solid fa-shield-halved"></i>${esc(shipName(i))}</button>` : '')).join('') : '';
  $('actBar').style.display = S.phase === 'over' ? 'none' : '';
  renderHint();
}
