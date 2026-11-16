'use strict';
/* =========================================================
   Constantes du jeu et état
   ========================================================= */
let N = 10;
const ROWS = 'ABCDEFGHIJKLMNOP';
const ALPHA = 'ABCDEFGHJKLMNPQRSTUVWXYZ23456789';
const PREFIX = 'c02-naval-v1-';
const MM_PREFIX = 'c02-naval-mm-v1-', MM_SLOTS = 6, MM_WAIT_MS = 45000;
let LENS = [5, 4, 3, 3, 2];
function shipName(i) {
  const len = LENS[i], list = (t('shipNames') || {})[len] || ['#' + len];
  const same = LENS.filter(l => l === len).length;
  const occ = LENS.slice(0, i).filter(l => l === len).length;
  if (same === 1) return list[0];
  return list[occ] || `${list[0]} ${occ + 1}`;
}
const HEARTBEAT_MS = 2000, TIMEOUT_MS = 20000, AWAY_TIMEOUT_MS = 120000, JOIN_TIMEOUT_MS = 15000;

/* Serveurs utilisés pour établir la liaison entre les deux appareils.
   - STUN + relais public de PeerJS (port 3478) : déjà inclus, sans compte, mais sans garantie.
   - Relais personnel (TURN) : facultatif. Sur les réseaux très fermés (entreprise, certains
     opérateurs 4G), seul un relais joignable en TLS sur le port 443 passe. Crée un compte gratuit
     chez un fournisseur TURN (ExpressTURN, Metered…) et recopie ses informations ci-dessous, par ex. :
     { urls: ['turn:xxx:3478', 'turns:xxx:443?transport=tcp'], username: '…', credential: '…' } */
const MY_TURN = null;
const ICE_SERVERS = [
  { urls: ['stun:stun.l.google.com:19302', 'stun:stun1.l.google.com:19302'] },
  { urls: ['turn:eu-0.turn.peerjs.com:3478', 'turn:us-0.turn.peerjs.com:3478'], username: 'peerjs', credential: 'peerjsp' },
  ...(MY_TURN ? [MY_TURN] : []),
];
// Relais TURN personnel saisi dans Paramètres → Avancé (enregistré uniquement dans ce navigateur).
function userTurn() {
  const u = String(settings.turnUrl || '').trim();
  if (!/^turns?:[\w.\-]+(:\d+)?(\?transport=(tcp|udp))?$/i.test(u)) return null;
  return { urls: [u], username: String(settings.turnUser || ''), credential: String(settings.turnCred || '') };
}
function peerOpts() {
  const extra = userTurn();
  return { debug: 0, config: { iceServers: extra ? [...ICE_SERVERS, extra] : ICE_SERVERS, sdpSemantics: 'unified-plan' } };
}
const RECONNECT_MS = 45000, PLACE_LIMIT_MS = 90000, TIMER_CHOICES = [0, 15, 30, 60, 90];
const SEQ = new Set(['ready', 'act', 'res', 'pass', 'quit', 'reveal']);   // messages numérotés (renvoyés après une coupure)

const $ = id => document.getElementById(id);
const S = {
  kind: null,                          // 'host' | 'join' | 'random' | 'bot'
  peer: null, conn: null, connected: false, isHost: false, code: '', phase: 'menu', lost: false, vsBot: false, bot: null,
  ships: LENS.map((len, i) => ({ i, len, cells: null, shield: 'none' })),
  sel: 0, horiz: true, preview: null, lifted: null, drag: null,
  meReady: false, oppReady: false, myTurn: false, waiting: false, mode: 'shot',
  enemy: Array(N * N).fill(null),     // null | miss | hit | sunk | blocked | scanOcc | scanEmpty
  incoming: Array(N * N).fill(null),  // null | miss | hit | blocked
  enemySunk: [], inv: { radar: 0, cross: 0, shield: 0 }, choices: 0, lastIn: [],
  lastSeen: 0, hb: null, joinTimer: null, pendingAct: null, oppAway: false, wakeLock: null,
  mmTimer: null, mmStopped: false, onReject: null,
  sid: null, rid: null, hostId: null, txn: 0, rxn: 0, outbox: [], reconnecting: false, reconDeadline: 0, reconTimer: null,
  rpeer: null, gpeer: null, attempting: false, turnStart: 0, placeDeadline: 0, turnTimer: null,
  oppUsed: 0, opp: null, match: { shots: 0, hits: 0, sunk: 0, upgrades: 0, start: 0 },
  aim: null,                           // tir spécial visé, en attente de confirmation : [r, c]
  myCommit: null, oppCommit: null, oppReveal: null, verify: 'none', verifyTimer: null, recordPending: false,   // anti-triche
  blockedSeen: 0, oppShields: 0,       // « bouclier » annoncés par l'adversaire / boucliers qu'il a réellement posés
};

/* ---------- Outils ---------- */
const cellName = (r, c) => ROWS[r] + (c + 1);
const inside = (r, c) => r >= 0 && r < N && c >= 0 && c < N;
const esc = s => String(s).replace(/[&<>"']/g, ch => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[ch]));
// Seuls ces emojis (tous adaptés à partir de 12 ans) sont acceptés comme avatar, en local comme reçus de l'adversaire.
const EMOJIS = ['⚓', '🚢', '⛵', '🛥️', '🐙', '🦈', '🐬', '🐳', '🐋', '🐟', '🐠', '🐡', '🦀', '🦞', '🐢', '🐧', '🦅', '🦉', '🦜', '🐺', '🦊', '🐻', '🐼', '🦁', '🐯', '🐸', '🐵', '🦄', '🐲',
  '🌊', '🏝️', '🌴', '☀️', '🌙', '⭐', '🚀', '🛸', '🔥', '⚡', '🌈', '🍀', '🎯', '🏆', '👑', '🎮', '🎲', '🧭', '🔭', '🤖', '👾', '😀', '😎', '🤠', '🥳', '😺'];
const validEmoji = e => typeof e === 'string' && EMOJIS.includes(e);
const THEMES = { ocean: ['#0a84d0', '#0f2b49'], night: ['#7c3aed', '#1d1740'], forest: ['#059669', '#0f3a2c'], sunset: ['#ea580c', '#3a2a4a'], light: ['#0a6fbf', '#dce9f7'],
  arcade: ['#ff3d9a', '#0a0a1c'], xmas: ['#ff3d5a', '#0a2016'] };
const THEME_BAR = { ocean: '#050d1b', night: '#0d0a1c', forest: '#06150f', sunset: '#1a0d10', light: '#dce9f7', arcade: '#0a0a1c', xmas: '#07150e' };
const RETRO_THEMES = ['arcade', 'xmas'];   // thèmes rétro : polices pixel, bordures carrées, ombres franches (voir « Thèmes rétro » dans le CSS)
// Thème par défaut (aucun choix du joueur) : « Rétro Noël » du 1er au 26 décembre inclus (date de l'appareil), « Océan » le reste de l'année.
function defaultTheme(d = new Date()) { return d.getMonth() === 11 && d.getDate() <= 26 ? 'xmas' : 'ocean'; }
const currentTheme = () => (THEMES[settings.theme] ? settings.theme : defaultTheme());
function applyTheme() {
  const th = currentTheme();
  const retro = RETRO_THEMES.includes(th);
  document.documentElement.dataset.theme = th;
  document.documentElement.classList.toggle('retro', retro);
  if (retro && !document.getElementById('retroFonts')) {   // polices pixel chargées seulement si un thème rétro est choisi
    const l = document.createElement('link');
    l.id = 'retroFonts'; l.rel = 'stylesheet';
    l.href = 'https://fonts.googleapis.com/css2?family=Press+Start+2P&family=VT323&display=swap';
    document.head.appendChild(l);
  }
  const m = document.querySelector('meta[name=theme-color]'); if (m) m.content = THEME_BAR[th];
}
function showView(id) { document.querySelectorAll('.view').forEach(e => e.classList.toggle('active', e.id === id)); window.scrollTo(0, 0); }
function msg(el, text, kind) { const e = $(el); e.textContent = text || ''; e.className = 'msg' + (kind === true ? ' err' : kind ? ' ' + kind : ''); }
function genCode() {
  const a = new Uint32Array(6); crypto.getRandomValues(a);
  return Array.from(a, x => ALPHA[x % ALPHA.length]).join('');
}
function occOf(ships) {
  const m = Array(N * N).fill(-1);
  ships.forEach((s, i) => s.cells && s.cells.forEach(([r, c]) => { m[r * N + c] = i; }));
  return m;
}
const occupancy = () => occOf(S.ships);
const sunkOf = (s, incoming) => !!s.cells && s.cells.every(([r, c]) => incoming[r * N + c] === 'hit');
const isSunk = s => sunkOf(s, S.incoming);
const targetable = k => S.enemy[k] === null || S.enemy[k] === 'scanOcc' || S.enemy[k] === 'scanEmpty' || S.enemy[k] === 'blocked';
function randomCells(lens) {
  for (let attempt = 0; attempt < 50; attempt++) {
    const out = randomCellsOnce(lens);
    if (out.every(Boolean)) return out;
  }
  return randomCellsOnce(lens);
}
function randomCellsOnce(lens) {
  const ships = lens.map(len => ({ len, cells: null }));
  for (const s of ships) {
    for (let tries = 0; tries < 3000 && !s.cells; tries++) {
      const occ = occOf(ships), h = Math.random() < 0.5;
      const r = Math.floor(Math.random() * N), c = Math.floor(Math.random() * N), cells = [];
      for (let i = 0; i < s.len; i++) {
        const rr = r + (h ? 0 : i), cc = c + (h ? i : 0);
        if (!inside(rr, cc) || occ[rr * N + cc] !== -1) break;
        cells.push([rr, cc]);
      }
      if (cells.length === s.len) s.cells = cells;
    }
  }
  return ships.map(s => s.cells);
}
function avatarHTML(emoji, name) {
  if (validEmoji(emoji)) return `<span class="mini-av">${emoji}</span>`;
  return `<span class="mini-av">${esc((name || '?').trim().charAt(0).toUpperCase() || '?')}</span>`;
}
function fmtTime(sec) {
  sec = Math.round(sec || 0);
  if (sec < 60) return sec + 's';
  if (sec < 3600) return Math.floor(sec / 60) + 'm ' + String(sec % 60).padStart(2, '0') + 's';
  return Math.floor(sec / 3600) + 'h ' + String(Math.floor(sec % 3600 / 60)).padStart(2, '0') + 'm';
}

/* =========================================================
   Règles personnalisables (taille de grille, flotte, améliorations)
   ========================================================= */
const MIN_SIZE = 6, MAX_SIZE = 15, MAX_LEN = 6, MAX_SHIPS = 10;
const DEFAULT_RULES = { size: 10, fleet: [5, 4, 3, 3, 2], radar: true, cross: true, shield: true, timer: 0 };
const maxCells = size => Math.floor(size * size * 0.4);
const sumOf = a => a.reduce((x, y) => x + y, 0);
// Valide et normalise des règles (venant du stockage ou du réseau). Renvoie null si invalides.
function sanitizeRules(c) {
  if (!c || typeof c !== 'object') return null;
  const size = c.size;
  if (!Number.isInteger(size) || size < MIN_SIZE || size > MAX_SIZE) return null;
  if (!Array.isArray(c.fleet) || c.fleet.length < 1 || c.fleet.length > MAX_SHIPS) return null;
  if (!c.fleet.every(l => Number.isInteger(l) && l >= 1 && l <= MAX_LEN)) return null;
  const fleet = c.fleet.slice().sort((a, b) => b - a);
  if (sumOf(fleet) > maxCells(size)) return null;
  return { size, fleet, radar: c.radar !== false, cross: c.cross !== false, shield: c.shield !== false, timer: TIMER_CHOICES.includes(c.timer) ? c.timer : 0 };
}
const cloneRules = r => ({ ...r, fleet: r.fleet.slice() });
const sameRules = (a, b) => a.size === b.size && a.fleet.join() === b.fleet.join() && a.radar === b.radar && a.cross === b.cross && a.shield === b.shield && a.timer === b.timer;
const isStdRules = r => sameRules(r, DEFAULT_RULES);
let RULES = sanitizeRules(settings.rules) || cloneRules(DEFAULT_RULES);   // règles choisies par le joueur (hôte / bot)
let CFG = cloneRules(RULES);                                              // règles de la partie en cours
if (typeof settings.bot !== 'boolean') settings.bot = true;
if (![0, 1, 2].includes(settings.skill)) settings.skill = 2;
const upgradeOn = k => !!CFG[k];
const anyUpgrade = () => CFG.radar || CFG.cross || CFG.shield;

function applyConfig(rules) {
  CFG = cloneRules(rules);
  N = CFG.size; LENS = CFG.fleet.slice();
  S.ships = LENS.map((len, i) => ({ i, len, cells: null, shield: 'none' }));
  S.enemy = Array(N * N).fill(null); S.incoming = Array(N * N).fill(null);
  S.enemySunk = []; S.lastIn = []; S.sel = 0; S.preview = null; S.lifted = null; S.drag = null;
  buildGrids();
}

function buildGrid(el, onTap) {
  // Repères : chiffres 1–10 en haut, lettres A–J à gauche (comme dans les messages « C5 »).
  el.innerHTML = ''; el.style.setProperty('--n', N);
  const lbl = txt => { const s = document.createElement('div'); s.className = 'lbl'; s.textContent = txt; el.appendChild(s); };
  el.cells = [];
  lbl('');
  for (let c = 0; c < N; c++) lbl(c + 1);
  for (let r = 0; r < N; r++) {
    lbl(ROWS[r]);
    for (let c = 0; c < N; c++) {
      const d = document.createElement('div');
      d.className = 'cell'; d.dataset.r = r; d.dataset.c = c;
      el.appendChild(d); el.cells.push(d);
    }
  }
  el.onclick = onTap ? e => {
    const d = e.target.closest('.cell');
    if (d && el.contains(d)) onTap(+d.dataset.r, +d.dataset.c);
  } : null;
}
