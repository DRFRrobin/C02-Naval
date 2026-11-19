'use strict';
/* =========================================================
   Comptes locaux : pseudo, mot de passe, photo, XP, statistiques
   Chaque compte est rangé sous sa propre clé : deux onglets qui jouent l'un contre
   l'autre dans le même navigateur ne peuvent pas écraser l'XP de l'autre.
   ========================================================= */
const STORE_KEY = 'c02naval.v1';
const ACC_PREFIX = STORE_KEY + '.acc.', CUR_KEY = STORE_KEY + '.current', TAB_KEY = STORE_KEY + '.tab';
const xpFor = l => 50 * l * (l - 1);               // XP total pour atteindre le niveau l : 0, 100, 300, 600, 1000…
function levelOf(xp) { let l = 1; while (xp >= xpFor(l + 1)) l++; return l; }
const rankOf = l => { const r = t('ranks'); return r[Math.min(l - 1, r.length - 1)]; };
const emptyStats = () => ({ games: 0, wins: 0, losses: 0, interrupted: 0, streak: 0, bestStreak: 0,
  shots: 0, hits: 0, sunk: 0, upgrades: 0, fastestWin: null, timePlayed: 0 });

let accounts = {};
let storageOk = true;
let curId = null;            // compte connecté dans CET onglet (null = invité)
let loggedIn = false;
function loadAccounts() {
  try {
    const out = {};
    for (let i = 0; i < localStorage.length; i++) {
      const k = localStorage.key(i);
      if (!k || !k.startsWith(ACC_PREFIX)) continue;
      try { const a = JSON.parse(localStorage.getItem(k)); if (a && a.id && a.name) out[a.id] = a; } catch (e) {}
    }
    accounts = out; storageOk = true;
  } catch (e) { storageOk = false; }   // stockage bloqué : on garde ce qui est en mémoire
}
function saveAccount(a) {
  accounts[a.id] = a;
  try { localStorage.setItem(ACC_PREFIX + a.id, JSON.stringify(a)); storageOk = true; } catch (e) { storageOk = false; }
}
const me = () => (curId ? accounts[curId] || null : null);
function setSession(id) {
  curId = id; loggedIn = true;
  try {
    if (id) { sessionStorage.setItem(TAB_KEY, id); localStorage.setItem(CUR_KEY, id); }
    else sessionStorage.setItem(TAB_KEY, 'guest');
  } catch (e) {}
}
function clearSession() {
  curId = null; loggedIn = false;
  try { sessionStorage.removeItem(TAB_KEY); localStorage.removeItem(CUR_KEY); } catch (e) {}
}
function restoreSession() {
  let id = null;
  try { id = sessionStorage.getItem(TAB_KEY) || localStorage.getItem(CUR_KEY); } catch (e) {}
  if (id === 'guest') { curId = null; loggedIn = true; return; }
  if (id && accounts[id] && (!accounts[id].cloud || accounts[id].tok)) { curId = id; loggedIn = true; }
}

const nameTaken = (name, exceptId) => Object.values(accounts).some(a => a.id !== exceptId && a.name.toLowerCase() === name.toLowerCase());

/* ---------- Comptes en ligne (API sur le NAS) ---------- */
const API_URL = (typeof window.C02_API === 'string' ? window.C02_API : 'https://api.c02.app').replace(/\/+$/, '');
let authMode = 'login', cloudPaneOn = true, syncing = false, syncTimer = null, apiUp = null;
async function api(method, path, body, tok) {
  let r;
  try {
    r = await fetch(API_URL + path, { method, headers: { 'content-type': 'application/json', ...(tok ? { authorization: 'Bearer ' + tok } : {}) },
      body: body === undefined ? undefined : JSON.stringify(body), signal: AbortSignal.timeout ? AbortSignal.timeout(12000) : undefined });
  } catch (e) { throw { code: 'network' }; }
  let j = null; try { j = await r.json(); } catch (e) {}
  if (!r.ok) throw { code: (j && j.error) || 'server_error', status: r.status, retryAfter: j && j.retryAfter };
  return j;
}
// API injoignable (pas encore installée, NAS éteint…) : on bascule sur le pseudo local au lieu d'afficher une erreur.
async function probeApi() {
  if (!API_URL) return;
  try { const r = await fetch(API_URL + '/api/health', { signal: AbortSignal.timeout ? AbortSignal.timeout(4000) : undefined }); apiUp = r.ok; }
  catch (e) { apiUp = false; }
  if (!apiUp && $('auth').classList.contains('active') && cloudPaneOn) { setAuthPane(false); msg('authMsg', t('apiDown')); }
}
function apiErr(e) {
  const k = 'e_' + (e && e.code);
  if (e && e.code === 'locked') return t('e_locked', { min: Math.max(1, Math.ceil((e.retryAfter || 900) / 60)) });
  return I18N[settings.lang][k] ? t(k) : t('e_server_error');
}
// Le profil du serveur fait foi : on le recopie dans le compte local (cache hors ligne).
function applyCloud(p, tok) {
  loadAccounts();
  const id = 'c:' + p.name.normalize('NFKC').toLowerCase(), prev = accounts[id] || {};
  const a = { id, cloud: true, tok: tok || prev.tok || null, name: p.name, emoji: validEmoji(p.emoji) ? p.emoji : null, xp: p.xp,
    created: new Date(p.created).toISOString(), lastLogin: new Date().toISOString(), stats: { ...emptyStats(), ...p.stats }, pending: prev.pending || [] };
  saveAccount(a); return a;
}
function setAuthPane(cloud) {
  cloudPaneOn = !!API_URL && cloud;
  $('cloudPane').style.display = cloudPaneOn ? '' : 'none';
  $('loginPane').style.display = cloudPaneOn ? 'none' : '';
  $('localNote').style.display = cloudPaneOn ? 'none' : '';
  $('btnCloudMode').style.display = API_URL && !cloudPaneOn ? '' : 'none';
  setAuthMode(authMode);
}
function setAuthMode(m) {
  authMode = m;
  $('tabLogin').classList.toggle('on', m === 'login'); $('tabReg').classList.toggle('on', m === 'reg');
  $('cPass').autocomplete = m === 'reg' ? 'new-password' : 'current-password';
  $('btnCloudT').textContent = t(m === 'reg' ? 'btnRegister' : 'btnLogin');
  msg('authMsg', '');
}
async function cloudSubmit() {
  const name = $('cName').value.trim().replace(/\s+/g, ' '), pw = $('cPass').value;
  if (name.length < 2 || name.length > 16) { Snd.play('blocked'); return msg('authMsg', t('errPseudoLen'), true); }
  if (pw.length < 8) { Snd.play('blocked'); return msg('authMsg', t('e_password_invalid'), true); }
  const btn = $('btnCloud'); btn.disabled = true; msg('authMsg', '…');
  try {
    let r;
    if (authMode === 'reg') {
      loadAccounts();   // un profil local du même pseudo est repris (borné par le serveur)
      const loc = Object.values(accounts).find(x => !x.cloud && x.name.toLowerCase() === name.toLowerCase());
      r = await api('POST', '/api/register', { name, password: pw, profile: loc ? { xp: loc.xp, emoji: loc.emoji, stats: loc.stats } : undefined });
    } else r = await api('POST', '/api/login', { name, password: pw });
    const a = applyCloud(r.profile, r.token);
    $('cName').value = ''; $('cPass').value = ''; msg('authMsg', '');
    setSession(a.id); openDash('play');
    msg('dashMsg', t('welcome', { name: a.name }), 'ok');
  } catch (e) { Snd.play('blocked'); msg('authMsg', apiErr(e), true); }
  finally { btn.disabled = false; }
}
function queueResult(a, type, body) { a.pending = a.pending || []; a.pending.push({ type, body }); saveAccount(a); syncCloud(); }
// Envoie les parties en attente (dans l'ordre) puis recharge le profil depuis le serveur.
async function syncCloud() {
  const a0 = me(); if (!a0 || !a0.cloud || !a0.tok || syncing) return;
  syncing = true; clearTimeout(syncTimer);
  try {
    for (;;) {
      loadAccounts(); const a = me(); if (!a) return;
      const it = (a.pending || [])[0]; if (!it) break;
      try { await api('POST', it.type === 'i' ? '/api/interrupted' : '/api/result', it.body, a.tok); }
      catch (e) { if (e.status !== 400) throw e; }          // 400 : résultat refusé par le serveur, on l'abandonne
      loadAccounts(); const b = me(); if (b) { b.pending.shift(); saveAccount(b); }
    }
    const a = me(); if (!a) return;
    const r = await api('GET', '/api/me', undefined, a.tok);
    applyCloud(r.profile);
    if ($('dash').classList.contains('active')) renderDash();
  } catch (e) {
    if (e.status === 401) { const a = me(); if (a) { a.tok = null; saveAccount(a); } clearSession(); openAuth(); msg('authMsg', t('e_session'), true); }
    else if (e.status === 429 || e.code === 'network') syncTimer = setTimeout(syncCloud, 20000);   // trop rapide / hors ligne : on réessaie
  } finally { syncing = false; }
}
async function cloudChangePassword() {
  const a = me(); if (!a || !a.cloud) return;
  const o = $('pwOld').value, n = $('pwNew').value;
  if (n.length < 8) return msg('pwMsg', t('e_password_invalid'), true);
  try { await api('POST', '/api/password', { password: o, newPassword: n }, a.tok); $('pwOld').value = ''; $('pwNew').value = ''; msg('pwMsg', t('pwDone'), 'ok'); }
  catch (e) { msg('pwMsg', apiErr(e), true); }
}

/* ---------- Écran de connexion ---------- */
function openAuth() {
  loadAccounts();
  setAuthPane(cloudPaneOn && apiUp !== false);
  if (apiUp === null) probeApi();
  const accs = Object.values(accounts).filter(a => !a.cloud).sort((a, b) => (b.lastLogin || '').localeCompare(a.lastLogin || ''));
  $('accBox').style.display = accs.length ? '' : 'none';
  const list = $('accPick'); list.innerHTML = '';
  accs.forEach(a => {
    const b = document.createElement('button');
    b.className = 'acc';
    b.innerHTML = `${avatarHTML(a.emoji, a.name)}<span><strong>${esc(a.name)}</strong><small>${esc(t('levelN', { n: levelOf(a.xp) }))} · ${a.xp} XP</small></span>`;
    b.onclick = () => { $('suName').value = a.name; doPlay(); };
    list.appendChild(b);
  });
  showView('auth');
}
// Entre avec ce pseudo : le profil existe déjà sur cet appareil (on le reprend) ou il est créé.
function doPlay() {
  loadAccounts();
  const name = $('suName').value.trim().replace(/\s+/g, ' ');
  if (name.length < 2 || name.length > 16) { Snd.play('blocked'); return msg('authMsg', t('errPseudoLen'), true); }
  const now = new Date().toISOString();
  let a = Object.values(accounts).find(x => !x.cloud && x.name.toLowerCase() === name.toLowerCase());
  const isNew = !a;
  if (a) a.lastLogin = now;
  else a = { id: 'a' + Date.now().toString(36) + Math.random().toString(36).slice(2, 6), name, emoji: EMOJIS[Math.floor(Math.random() * EMOJIS.length)], xp: 0, created: now, lastLogin: now, stats: emptyStats() };
  saveAccount(a);
  $('suName').value = '';
  msg('authMsg', '');
  setSession(a.id);
  openDash('play');
  msg('dashMsg', storageOk ? t('welcome', { name: a.name }) : t('errStorage'), storageOk ? 'ok' : true);
}

/* ---------- Tableau de bord ---------- */
let pane = 'play';
let pendingCode = null;
function openDash(p) {
  S.phase = 'menu';
  pane = p || pane;
  if (renderDash() === false) return;
  showView('dash');
  syncCloud();
  if (pendingCode) { const c = pendingCode; pendingCode = null; openJoin(c); }
}
function renderDash() {
  loadAccounts();
  const a = me();
  if (curId && !a) { clearSession(); openAuth(); return false; }   // compte supprimé ailleurs
  // Barre du haut
  const up = $('avUp');
  up.classList.toggle('has', !!(a && validEmoji(a.emoji)));
  up.classList.toggle('guest', !a);
  up.title = a ? t('photoChange') : '';
  $('avImg').innerHTML = a && validEmoji(a.emoji) ? esc(a.emoji) : `<i class="fa-solid ${a ? 'fa-face-smile' : 'fa-user'}"></i>`;
  $('tbName').textContent = a ? a.name : t('guestName');
  $('tbBadge').style.display = $('tbXpWrap').style.display = a ? '' : 'none';
  if (a) {
    const l = levelOf(a.xp), lo = xpFor(l), hi = xpFor(l + 1);
    $('tbBadge').textContent = t('badge', { n: l });
    $('tbXp').textContent = `${a.xp - lo} / ${hi - lo}`;
    $('tbLevel').textContent = t('levelN', { n: l }) + ' · ' + rankOf(l);
    const pctXp = Math.round((a.xp - lo) / (hi - lo) * 100) + '%';
    if (!$('dash').classList.contains('active')) $('tbXpFill').style.width = '0%';   // la barre se remplit à l'ouverture du tableau de bord
    requestAnimationFrame(() => requestAnimationFrame(() => { $('tbXpFill').style.width = pctXp; }));
  }
  document.querySelectorAll('.tabsnav [data-pane]').forEach(b => b.classList.toggle('on', b.dataset.pane === pane));
  document.querySelectorAll('#dash .pane').forEach(e => e.classList.toggle('active', e.id === 'pane-' + pane));
  if (pane === 'stats') renderStats();
  if (pane === 'settings') renderSettings();
  if (pane === 'rules') renderRules();
  renderInstall();
  $('mBot').classList.toggle('off', !settings.bot);
  $('mBot').style.opacity = settings.bot ? '' : '.45';
  $('rulesSum').innerHTML = (isStdRules(RULES) ? esc(t('rSumStd')) : esc(t('rSumCustom', { size: RULES.size, n: RULES.fleet.length })))
    + ` — <a id="rulesLink">${esc(t('rEdit'))}</a>`;
  $('rulesLink').onclick = () => { pane = 'rules'; renderDash(); };
}

/* ---------- Installation comme application (écran d'accueil) ---------- */
const IS_IOS = /iPad|iPhone|iPod/.test(navigator.userAgent) || (navigator.platform === 'MacIntel' && navigator.maxTouchPoints > 1);
const isStandalone = () => navigator.standalone === true || (window.matchMedia && matchMedia('(display-mode: standalone)').matches);
const TIP_KEY = STORE_KEY + '.installTipHidden';
let installEvt = null;
window.addEventListener('beforeinstallprompt', e => { e.preventDefault(); installEvt = e; if ($('dash').classList.contains('active')) renderInstall(); });
window.addEventListener('appinstalled', () => { installEvt = null; if ($('dash').classList.contains('active')) renderInstall(); });
function iosSteps() { return `<ol class="how">${[1, 2, 3, 4].map(n => `<li>${t('instIosS' + n)}</li>`).join('')}</ol>`; }
async function doInstall() {
  if (!installEvt) return false;
  const e = installEvt; installEvt = null;
  try { e.prompt(); await e.userChoice; } catch (err) {}
  renderInstall(); return true;
}
function renderInstall() {
  const standalone = isStandalone();
  let hidden = false; try { hidden = !!localStorage.getItem(TIP_KEY); } catch (e) {}
  // bandeau sur l'onglet Jouer : seulement sur iPhone/iPad ou quand le navigateur propose l'installation
  const canTip = !standalone && !hidden && (IS_IOS || !!installEvt);
  $('installTip').classList.toggle('show', canTip);
  if (canTip) {
    $('instTipD').textContent = t('instTipHow');
    $('btnInstTip').textContent = t(installEvt ? 'instBtnInstall' : 'instBtnHow');
  }
  // bloc dans les réglages
  const b = $('instBody');
  if (standalone) b.innerHTML = `<p class="msg ok" style="text-align:left">${esc(t('instDone'))}</p>`;
  else if (installEvt) b.innerHTML = `<button id="btnInstall" class="btn primary wide" style="margin-top:12px"><i class="fa-solid fa-download"></i><span>${esc(t('instBtnInstall'))}</span></button>`;
  else if (IS_IOS) b.innerHTML = iosSteps();
  else b.innerHTML = `<p class="msg" style="text-align:left">${esc(t('instGen'))}</p>`;
  const bi = $('btnInstall'); if (bi) bi.onclick = doInstall;
}

/* ---------- Écran « Règles » ---------- */
function setRules(fn) {
  const c = cloneRules(RULES); fn(c);
  const ok = sanitizeRules(c); if (!ok) return;
  RULES = ok; settings.rules = cloneRules(ok); saveSettings(); renderDash();
}
function renderRules() {
  const R = RULES, tot = sumOf(R.fleet), cap = maxCells(R.size);
  const cnt = len => R.fleet.filter(l => l === len).length;
  const step = (id, val, minus, plus) => `<div class="stepper"><button data-r="${id}-" ${minus ? '' : 'disabled'}>−</button><output>${val}</output><button data-r="${id}+" ${plus ? '' : 'disabled'}>+</button></div>`;
  const tog = (k, on) => `<button class="toggle ${on ? 'on' : 'off'}" data-r="t-${k}">${t(on ? 'rOn' : 'rOff')}</button>`;
  let h = `<div class="set-row"><i class="fa-solid fa-table-cells"></i><div><b>${esc(t('rGridT'))}</b><small>${esc(t('rGridD'))}</small></div>`
    + step('size', `${R.size} × ${R.size}`, R.size > MIN_SIZE, R.size < MAX_SIZE) + '</div>';
  h += `<div class="set-block"><b><i class="fa-solid fa-ship"></i>${esc(t('rFleetT'))}</b><small>${esc(t('rFleetD'))}</small>`;
  for (let len = MAX_LEN; len >= 1; len--) {
    const n = cnt(len), nm = (t('shipNames')[len] || ['?'])[0];
    h += `<div class="rule-ship"><span class="nm">${esc(nm)}</span><span class="blocks">${'<i></i>'.repeat(len)}</span>`
      + step('f' + len, n, n > 0 && R.fleet.length > 1, R.fleet.length < MAX_SHIPS && tot + len <= cap) + '</div>';
  }
  h += `<small style="margin-top:12px;color:var(--sky)">${esc(t('rFleetTot', { n: R.fleet.length, cells: tot, pct: Math.round(tot / (R.size * R.size) * 100) }))}</small></div>`;
  h += `<div class="set-block"><b><i class="fa-solid fa-wand-magic-sparkles"></i>${esc(t('rUpT'))}</b><small>${esc(t('rUpD'))}</small>`;
  for (const k of ['radar', 'cross', 'shield']) {
    h += `<div class="set-row" style="margin-top:10px"><i class="fa-solid ${UPG[k][0]}"></i><div><b>${esc(t(UPG[k][1]))}</b><small>${esc(t(UPG[k][2]))}</small></div>${tog(k, R[k])}</div>`;
  }
  h += '</div>';
  h += `<div class="set-row"><i class="fa-solid fa-stopwatch"></i><div><b>${esc(t('rTimerT'))}</b><small>${esc(t('rTimerD'))}</small></div>`
    + step('timer', R.timer ? t('timerN', { n: R.timer }) : t('timerOff'), TIMER_CHOICES.indexOf(R.timer) > 0, TIMER_CHOICES.indexOf(R.timer) < TIMER_CHOICES.length - 1) + '</div>';
  h += `<div class="set-row"><i class="fa-solid fa-robot"></i><div><b>${esc(t('rBotT'))}</b><small>${esc(t('rBotD'))}</small></div>${tog('bot', settings.bot)}</div>`;
  h += `<div class="set-block"><b>${esc(t('rSkillT'))}</b><select id="skillSel" ${settings.bot ? '' : 'disabled'}>`
    + [0, 1, 2].map(i => `<option value="${i}" ${settings.skill === i ? 'selected' : ''}>${esc(t('skill' + i))}</option>`).join('') + '</select></div>';
  h += `<div class="row" style="margin-top:18px"><button class="btn" data-r="reset"><i class="fa-solid fa-rotate-left"></i><span>${esc(t('rReset'))}</span></button></div>`;
  $('rulesBox').innerHTML = h;
  const sel = $('skillSel'); if (sel) sel.onchange = e => { settings.skill = +e.target.value; saveSettings(); };
}
$('rulesBox').addEventListener('click', e => {
  const b = e.target.closest('button[data-r]'); if (!b || b.disabled) return;
  const a = b.dataset.r; Snd.play('click');
  if (a === 'reset') return setRules(c => Object.assign(c, cloneRules(DEFAULT_RULES)));
  if (a === 't-bot') { settings.bot = !settings.bot; saveSettings(); return renderDash(); }
  if (a.startsWith('t-')) return setRules(c => { c[a.slice(2)] = !c[a.slice(2)]; });
  if (a.startsWith('timer')) return setRules(c => { const i = TIMER_CHOICES.indexOf(c.timer) + (a.endsWith('+') ? 1 : -1); c.timer = TIMER_CHOICES[Math.max(0, Math.min(TIMER_CHOICES.length - 1, i))]; });
  if (a.startsWith('size')) return setRules(c => {
    c.size += a.endsWith('+') ? 1 : -1;
    c.fleet.sort((x, y) => y - x);
    while (c.fleet.length > 1 && sumOf(c.fleet) > maxCells(c.size)) c.fleet.shift();   // flotte trop grande : on retire les plus gros bateaux
  });
  const m = /^f(\d)([+-])$/.exec(a);
  if (m) setRules(c => {
    const len = +m[1];
    if (m[2] === '+') c.fleet.push(len); else { const i = c.fleet.indexOf(len); if (i >= 0) c.fleet.splice(i, 1); }
  });
});
function renderStats() {
  const a = me();
  const s = { ...emptyStats(), ...(a ? a.stats : {}) };
  const pct = (x, y) => (y ? Math.round(x / y * 100) : 0) + '%';
  const rate = (x, y) => (y ? x / y : 0).toFixed(2);
  const tile = (icon, color, val, label, valColor) =>
    `<div class="tile"><i class="fa-solid ${icon} ${color}"></i><b class="${valColor || 'c-white'}">${val}</b><span>${esc(label)}</span></div>`;
  $('careerGrid').innerHTML = [
    tile('fa-clock', 'c-sky', fmtTime(s.timePlayed), t('stTime')),
    tile('fa-gamepad', 'c-purple', s.games, t('stGames')),
    tile('fa-trophy', 'c-green', s.wins, t('stWins'), 'c-green'),
    tile('fa-skull-crossbones', 'c-red', s.losses, t('stLosses'), 'c-red'),
    tile('fa-percent', 'c-sky', pct(s.wins, s.games), t('stWinPct'), 'c-sky'),
    tile('fa-percent', 'c-yellow', pct(s.losses, s.games), t('stLossPct'), 'c-yellow'),
    tile('fa-arrow-trend-up', 'c-green', rate(s.wins, s.games), t('stWinRate'), 'c-green'),
    tile('fa-arrow-trend-down', 'c-red', rate(s.losses, s.games), t('stLossRate'), 'c-red'),
  ].join('');
  $('combatGrid').innerHTML = [
    tile('fa-crosshairs', 'c-sky', s.shots, t('stShots')),
    tile('fa-bullseye', 'c-green', pct(s.hits, s.shots), t('stAcc'), 'c-green'),
    tile('fa-ship', 'c-red', s.sunk, t('stSunk')),
    tile('fa-wand-magic-sparkles', 'c-purple', s.upgrades, t('stUpg')),
    tile('fa-fire', 'c-yellow', s.streak, t('stStreak'), 'c-yellow'),
    tile('fa-medal', 'c-yellow', s.bestStreak, t('stBest'), 'c-yellow'),
    tile('fa-bolt', 'c-sky', s.fastestWin ? t('shotsN', { n: s.fastestWin }) : '—', t('stFast')),
    tile('fa-plug-circle-xmark', 'c-red', s.interrupted, t('stInt')),
  ].join('');
  $('statsGuest').style.display = a ? 'none' : '';
}
function renderSettings() {
  const a = me();
  $('sndToggle').className = 'toggle ' + (settings.sound ? 'on' : 'off');
  $('sndToggle').textContent = settings.sound ? 'ON' : 'OFF';
  $('verNum').textContent = 'w' + APP_VERSION;   // le site s'appelle « w1.2.4 » (le nombre seul sert aux comparaisons)
  $('volRange').value = Math.round((typeof settings.volume === 'number' ? settings.volume : 0.8) * 100);
  $('volRange').disabled = !settings.sound;
  $('acctSettings').style.display = a ? '' : 'none';
  $('cloudSettings').style.display = a && a.cloud ? '' : 'none';
  $('localProfile').style.display = a && !a.cloud ? '' : 'none';
  $('backupBlock').style.display = a && !a.cloud ? '' : 'none';
  if (a) {
    if (document.activeElement !== $('editName')) $('editName').value = a.name;
    $('btnPhotoDel').disabled = !validEmoji(a.emoji);
  }
  renderThemes();
  if (document.activeElement !== $('turnUrl')) { $('turnUrl').value = settings.turnUrl || ''; $('turnUser').value = settings.turnUser || ''; $('turnCred').value = settings.turnCred || ''; }
}
function saveProfile() {
  loadAccounts();
  const a = me(); if (!a) return;
  const name = $('editName').value.trim().replace(/\s+/g, ' ');
  if (name.length < 2 || name.length > 16) return msg('setMsg', t('errPseudoLen'), true);
  if (nameTaken(name, a.id)) return msg('setMsg', t('errPseudoTaken'), true);
  if (name === a.name) return msg('setMsg', t('nothingChanged'));
  a.name = name;
  saveAccount(a);
  renderDash();
  msg('setMsg', t('saved'), 'ok');
}
// ===== Sauvegarde chiffrée du profil (transfert entre appareils) =====
// PBKDF2-SHA256 (600 000 itérations) -> AES-256-GCM : confidentialité + intégrité, sans aucun serveur.
const BK_MAGIC = 'C02S1.';
const b64e = u => { let s = ''; u.forEach(c => s += String.fromCharCode(c)); return btoa(s).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, ''); };
const b64d = str => { const b = atob(str.replace(/-/g, '+').replace(/_/g, '/')); return Uint8Array.from(b, c => c.charCodeAt(0)); };
async function bkKey(pass, salt, usage) {
  const base = await crypto.subtle.importKey('raw', new TextEncoder().encode(pass.normalize('NFKC')), 'PBKDF2', false, ['deriveKey']);
  return crypto.subtle.deriveKey({ name: 'PBKDF2', salt, iterations: 600000, hash: 'SHA-256' }, base, { name: 'AES-GCM', length: 256 }, false, [usage]);
}
async function encryptBackup(obj, pass) {
  const salt = crypto.getRandomValues(new Uint8Array(16)), iv = crypto.getRandomValues(new Uint8Array(12));
  const ct = new Uint8Array(await crypto.subtle.encrypt({ name: 'AES-GCM', iv }, await bkKey(pass, salt, 'encrypt'), new TextEncoder().encode(JSON.stringify(obj))));
  const out = new Uint8Array(28 + ct.length); out.set(salt); out.set(iv, 16); out.set(ct, 28);
  return BK_MAGIC + b64e(out);
}
async function decryptBackup(code, pass) {
  code = String(code || '').trim();
  if (!code.startsWith(BK_MAGIC)) throw new Error('fmt');
  let raw; try { raw = b64d(code.slice(BK_MAGIC.length).replace(/\s+/g, '')); } catch (e) { throw new Error('fmt'); }
  if (raw.length < 45 || raw.length > 20000) throw new Error('fmt');
  let plain; try { plain = await crypto.subtle.decrypt({ name: 'AES-GCM', iv: raw.slice(16, 28) }, await bkKey(pass, raw.slice(0, 16), 'decrypt'), raw.slice(28)); } catch (e) { throw new Error('bad'); }
  let o; try { o = JSON.parse(new TextDecoder().decode(plain)); } catch (e) { throw new Error('fmt'); }
  return sanitizeBackup(o);
}
// Ne jamais faire confiance au contenu importé : on reconstruit un profil propre.
function sanitizeBackup(o) {
  if (!o || o.v !== 1 || typeof o.name !== 'string') throw new Error('fmt');
  const name = o.name.trim().replace(/\s+/g, ' ');
  if (name.length < 2 || name.length > 16) throw new Error('fmt');
  const num = (v, max) => (Number.isFinite(v) ? Math.min(Math.max(Math.floor(v), 0), max) : 0);
  const st = { ...emptyStats() }, src = o.stats && typeof o.stats === 'object' ? o.stats : {};
  for (const k of Object.keys(st)) {
    if (k === 'fastestWin') st[k] = Number.isFinite(src[k]) && src[k] >= 0 ? Math.min(Math.floor(src[k]), 1e7) : null;
    else st[k] = num(src[k], 1e9);
  }
  // date de création : texte ISO (format du profil) ou nombre (anciennes sauvegardes)
  const ts = typeof o.created === 'string' ? Date.parse(o.created) : o.created;
  const created = new Date(Number.isFinite(ts) && ts > 0 && ts < 4e12 ? ts : Date.now()).toISOString();
  return { name, emoji: validEmoji(o.emoji) ? o.emoji : null, xp: num(o.xp, 1e9), created, stats: st };
}
function bkPass() {
  const p = $('bkPass').value;
  if (p.length < 8) { msg('bkMsg', t('bkNeedPass'), true); return null; }
  return p;
}
async function bkCreate() {
  loadAccounts(); const a = me(); const p = bkPass(); if (!a || !p) return null;
  msg('bkMsg', '…');
  return encryptBackup({ v: 1, name: a.name, emoji: a.emoji || null, xp: a.xp, created: a.created, stats: a.stats, at: Date.now() }, p);
}
async function bkExportFile() {
  const code = await bkCreate(); if (!code) return;
  const link = document.createElement('a');
  link.href = URL.createObjectURL(new Blob([code], { type: 'text/plain' }));
  link.download = 'c02-naval-' + me().name.replace(/[^\w-]+/g, '_') + '.c02save';
  document.body.appendChild(link); link.click(); link.remove();
  setTimeout(() => URL.revokeObjectURL(link.href), 4000);
  msg('bkMsg', t('bkDone'), 'ok');
}
async function bkExportCopy() {
  const code = await bkCreate(); if (!code) return;
  $('bkCode').value = code;
  try { await navigator.clipboard.writeText(code); msg('bkMsg', t('bkCopied'), 'ok'); }
  catch (e) { $('bkCode').select(); msg('bkMsg', t('bkDone'), 'ok'); }
}
async function bkImport() {
  const code = $('bkCode').value.trim();
  if (!code) return msg('bkMsg', t('bkNeedCode'), true);
  const p = bkPass(); if (!p) return;
  msg('bkMsg', '…');
  let d;
  try { d = await decryptBackup(code, p); } catch (e) { return msg('bkMsg', t(e.message === 'bad' ? 'bkBad' : 'bkFmt'), true); }
  loadAccounts();
  // Profil de même pseudo déjà présent : on garde le plus avancé (XP puis parties jouées).
  const cur = Object.values(accounts).find(x => x.name.toLowerCase() === d.name.toLowerCase());
  if (cur) {
    const ahead = cur.xp > d.xp || (cur.xp === d.xp && (cur.stats?.games || 0) >= d.stats.games);
    if (ahead && !confirm(t('bkNewer', { name: cur.name }))) return msg('bkMsg', t('bkKept'));
    Object.assign(cur, { emoji: d.emoji || cur.emoji, xp: d.xp, created: new Date(Math.min(Date.parse(cur.created) || Infinity, Date.parse(d.created))).toISOString(), stats: d.stats });
    saveAccount(cur); setSession(cur.id);
  } else {
    const id = 'a' + Date.now().toString(36) + Math.random().toString(36).slice(2, 6);
    saveAccount({ id, name: d.name, emoji: d.emoji, xp: d.xp, created: d.created, lastLogin: new Date().toISOString(), stats: d.stats });
    setSession(id);
  }
  $('bkCode').value = ''; $('bkPass').value = '';
  renderDash(); msg('bkMsg', t('bkOk', { name: d.name }), 'ok');
}
async function deleteAccount() {
  const a = me(); if (!a) return;
  if (!confirm(t('deleteConfirm', { name: a.name }))) return;
  if (a.cloud) {
    const pw = prompt(t('delPrompt'));
    if (pw === null) return;
    try { await api('POST', '/api/delete', { password: pw }, a.tok); } catch (e) { return msg('pwMsg', apiErr(e), true); }
  }
  try { localStorage.removeItem(ACC_PREFIX + a.id); } catch (e) {}
  delete accounts[a.id];
  clearSession();
  openAuth();
}
// Avatar : un emoji choisi dans la liste EMOJIS.
function pickPhoto() {
  const a = me(); if (!a) return;
  const g = $('emojiGrid'); g.innerHTML = '';
  EMOJIS.forEach(e => {
    const b = document.createElement('button');
    b.type = 'button'; b.textContent = e; b.className = e === a.emoji ? 'on' : '';
    b.onclick = () => { loadAccounts(); const acc = me(); if (acc) { acc.emoji = e; saveAccount(acc); renderDash(); if (acc.cloud) api('PATCH', '/api/me', { emoji: e }, acc.tok).catch(() => {}); } Snd.play('click'); $('emojiModal').classList.remove('show'); };
    g.appendChild(b);
  });
  $('emojiModal').classList.add('show');
}
function renderThemes() {
  // « Auto » : aucun choix enregistré, le thème suit la saison (Noël du 1er au 26 décembre).
  const auto = `<button class="theme-btn${THEMES[settings.theme] ? '' : ' on'}" data-th="auto"><span class="sw" style="background:linear-gradient(135deg,${THEMES.ocean[0]} 50%,${THEMES.xmas[0]} 50%)"></span>${esc(t('themeNames').auto)}</button>`;
  $('themeBox').innerHTML = auto + Object.keys(THEMES).map(k =>
    `<button class="theme-btn${settings.theme === k ? ' on' : ''}" data-th="${k}"><span class="sw" style="background:linear-gradient(135deg,${THEMES[k][1]} 50%,${THEMES[k][0]} 50%)"></span>${esc(t('themeNames')[k])}</button>`).join('');
}

/* ---------- Enregistrement des parties ---------- */
// Formule d'XP — identique à celle du serveur (server/app.js) : le serveur fait foi pour les comptes en ligne.
const clamp01 = x => Math.min(1, Math.max(0, x));
function xpParts(win, shots, hits, sunk, duration, half) {
  const acc = shots ? hits / shots : 0;
  const parts = { base: win ? 100 : 30, hits: hits * 5, sunk: sunk * 20, acc: Math.round(acc * (win ? 100 : 30)), time: win ? Math.round(60 * clamp01((900 - duration) / 720)) : 0 };
  const sum = parts.base + parts.hits + parts.sunk + parts.acc + parts.time;
  return { parts, gain: half ? Math.round(sum / 2) : sum };
}
function recordGame(win) {
  loadAccounts();
  const a = me(); if (!a) return null;
  const m = S.match, s = a.stats = { ...emptyStats(), ...a.stats };
  const shots = Math.min(m.shots, 500), hits = Math.min(m.hits, shots), sunk = Math.min(m.sunk, hits, 12);
  const dur = Math.min(7200, Math.max(5, m.start ? Math.round((Date.now() - m.start) / 1000) : 5));
  const { parts, gain } = xpParts(win, shots, hits, sunk, dur, S.vsBot || !isStdRules(CFG));
  const before = levelOf(a.xp), xpBefore = a.xp;
  a.xp += gain;
  s.games++; s.shots += m.shots; s.hits += m.hits; s.sunk += m.sunk; s.upgrades += m.upgrades;
  s.timePlayed += m.start ? Math.round((Date.now() - m.start) / 1000) : 0;
  if (win) {
    s.wins++; s.streak++; s.bestStreak = Math.max(s.bestStreak, s.streak);
    if (m.shots > 0 && (!s.fastestWin || m.shots < s.fastestWin)) s.fastestWin = m.shots;
  } else { s.losses++; s.streak = 0; }
  saveAccount(a);
  if (a.cloud) {   // le serveur recalcule l'XP lui-même : on lui envoie seulement ce qui s'est passé
    queueResult(a, 'r', { win: !!win, shots, hits, sunk, upgrades: Math.min(m.upgrades, 30), duration: dur, bot: !!S.vsBot, std: isStdRules(CFG), forfeit: !!(win && S.forfeit) });
  }
  return { gain, parts, xpBefore, levelBefore: before, level: levelOf(a.xp), up: levelOf(a.xp) > before };
}
function recordInterrupted(force) {
  if (S.phase !== 'game' && !force) return;
  loadAccounts();
  const a = me(); if (!a) return;
  a.stats = { ...emptyStats(), ...a.stats };
  a.stats.interrupted++;
  a.stats.timePlayed += S.match.start ? Math.round((Date.now() - S.match.start) / 1000) : 0;
  saveAccount(a);
  if (a.cloud) queueResult(a, 'i', { duration: Math.min(7200, S.match.start ? Math.round((Date.now() - S.match.start) / 1000) : 0) });
}
function myIdentity() {
  const a = me();
  return a ? { name: a.name, emoji: validEmoji(a.emoji) ? a.emoji : null, level: levelOf(a.xp) } : { name: t('guestName'), emoji: null, level: 0 };
}
function renderVs() {
  const i = myIdentity(), o = S.opp;
  const who = (p, bot) => `<span class="who2">${bot ? '<span class="mini-av" style="color:var(--green)"><i class="fa-solid fa-robot"></i></span>' : avatarHTML(p.emoji, p.name)}<span>${esc(p.name)}</span>${p.level ? `<span class="lv">${esc(t('levelShort', { n: p.level }))}</span>` : ''}</span>`;
  const html = who(i) + (o ? `<span class="vsep">${esc(t('vs'))}</span>` + who(o, o.bot) : '');
  $('placeVs').innerHTML = $('gameVs').innerHTML = html;
  $('enemyLabel').textContent = t('enemyGrid', { name: o ? o.name : '?' });
}
