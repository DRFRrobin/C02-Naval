'use strict';
/* =========================================================
   Démarrage
   ========================================================= */
function buildGrids() {
  buildGrid($('placeGrid'), null);
  buildGrid($('enemyGrid'), onEnemyTap);
  buildGrid($('myGrid'), onMyTap);
}
applyConfig(RULES);
const pg = $('placeGrid');
pg.addEventListener('pointerdown', onPlaceDown);
pg.addEventListener('pointermove', onPlaceMove);
pg.addEventListener('pointerup', onPlaceUp);
pg.addEventListener('pointercancel', () => { S.drag = null; });

// Connexion
$('btnPlay').onclick = doPlay;
$('btnCloud').onclick = cloudSubmit;
$('cPass').onkeydown = $('cName').onkeydown = e => { if (e.key === 'Enter') cloudSubmit(); };
$('tabLogin').onclick = () => setAuthMode('login');
$('tabReg').onclick = () => setAuthMode('reg');
$('btnLocalMode').onclick = () => setAuthPane(false);
$('btnCloudMode').onclick = () => { setAuthPane(true); apiUp = null; msg('authMsg', ''); probeApi(); };
$('btnPw').onclick = cloudChangePassword;
$('suName').addEventListener('keydown', e => { if (e.key === 'Enter') doPlay(); });

// Installation
$('btnInstTip').onclick = async () => { if (!(await doInstall())) { pane = 'settings'; renderDash(); } };
$('btnInstTipX').onclick = () => { try { localStorage.setItem(TIP_KEY, '1'); } catch (e) {} renderInstall(); };
/* ---------- Version et mises à jour ---------- */
const APP_VERSION = '1.2.4';
const verCmp = (a, b) => { const x = a.split('.').map(Number), y = b.split('.').map(Number); for (let i = 0; i < 3; i++) if ((x[i] || 0) !== (y[i] || 0)) return (x[i] || 0) - (y[i] || 0); return 0; };
let newVersion = null;
// Lit version.json sur le serveur (jamais depuis un cache) : 'new' | 'ok' | 'err'
async function checkUpdate() {
  try {
    const r = await fetch('version.json?t=' + Date.now(), { cache: 'no-store' });
    if (!r.ok) return 'err';
    const j = await r.json();
    if (!j || typeof j.version !== 'string' || !/^\d+\.\d+\.\d+$/.test(j.version)) return 'err';
    if (verCmp(j.version, APP_VERSION) > 0) { newVersion = j.version; return 'new'; }
    newVersion = null; return 'ok';
  } catch (e) { return 'err'; }
}
function showUpdateBar() {
  if (!newVersion) { $('updBar').style.display = 'none'; return; }
  $('updTxt').textContent = t('updNew', { v: newVersion });
  $('updBar').style.display = '';
}
async function applyUpdate() {
  try { sessionStorage.setItem('c02.upd', newVersion || '1'); } catch (e) {}   // garde-fou : un seul rechargement automatique par version
  try { for (const k of await caches.keys()) await caches.delete(k); } catch (e) {}
  try { for (const r of await navigator.serviceWorker.getRegistrations()) await r.update().catch(() => {}); } catch (e) {}
  location.reload();
}
const canAutoUpdate = () => (S.phase === 'menu' || !S.phase) && ($('auth').classList.contains('active') || $('dash').classList.contains('active'));
// À chaque ouverture : si une version plus récente existe, on recharge tout de suite (sinon bandeau).
async function updateOnOpen() {
  if (await checkUpdate() !== 'new') return;
  let tried = null; try { tried = sessionStorage.getItem('c02.upd'); } catch (e) {}
  if (tried !== newVersion && canAutoUpdate()) applyUpdate(); else showUpdateBar();
}
async function manualCheck() {
  const say = (txt, color) => { const e = $('verMsg'); e.textContent = txt; e.style.color = color || ''; };
  say(t('verChecking'));
  const r = await checkUpdate();
  if (r === 'new') { showUpdateBar(); say(t('updNew', { v: newVersion }), 'var(--yellow)'); }
  else say(t(r === 'ok' ? 'verOk' : 'verErr'), r === 'ok' ? 'var(--green)' : 'var(--err)');
}
$('btnUpd').onclick = applyUpdate;
$('btnVer').onclick = manualCheck;
window.addEventListener('load', () => setTimeout(updateOnOpen, 300));
// Application laissée ouverte longtemps (iPhone) : on revérifie au retour au premier plan, et toutes les 30 minutes.
document.addEventListener('visibilitychange', () => { if (!document.hidden) checkUpdate().then(r => { if (r === 'new') showUpdateBar(); }); });
setInterval(() => checkUpdate().then(r => { if (r === 'new') showUpdateBar(); }), 30 * 60 * 1000);

if ('serviceWorker' in navigator && /^https?:$/.test(location.protocol)) window.addEventListener('load', () => { navigator.serviceWorker.register('sw.js').catch(() => {}); });

// Tableau de bord
document.querySelectorAll('.tabsnav [data-pane]').forEach(b => { b.onclick = () => { pane = b.dataset.pane; msg('setMsg', ''); Snd.play('click'); renderDash(); }; });
$('btnLogout').onclick = () => {
  const a = me();
  if (a && a.cloud) { if (a.tok) api('POST', '/api/logout', {}, a.tok).catch(() => {}); a.tok = null; saveAccount(a); }   // le jeton est révoqué : le mot de passe sera redemandé
  clearSession(); msg('authMsg', ''); openAuth();
};
$('avUp').addEventListener('click', e => { e.preventDefault(); pickPhoto(); });
['turnUrl', 'turnUser', 'turnCred'].forEach(id => $(id).addEventListener('change', () => {
  settings.turnUrl = $('turnUrl').value.trim(); settings.turnUser = $('turnUser').value.trim(); settings.turnCred = $('turnCred').value;
  saveSettings(); msg('turnMsg', userTurn() || !settings.turnUrl ? t('turnSaved') : t('errNetOther', { type: 'TURN' }), userTurn() || !settings.turnUrl ? 'ok' : true);
}));
$('btnEmojiCancel').onclick = () => $('emojiModal').classList.remove('show');
$('emojiModal').addEventListener('click', e => { if (e.target === $('emojiModal')) $('emojiModal').classList.remove('show'); });
$('themeBox').addEventListener('click', e => {
  const b = e.target.closest('[data-th]'); if (!b) return;
  if (b.dataset.th === 'auto') { delete settings.theme; settings.themeChosen = false; } else { settings.theme = b.dataset.th; settings.themeChosen = true; }
  saveSettings(); applyTheme(); renderThemes(); Snd.play('click');
});
// Thème automatique : la date a pu changer pendant que le jeu était en arrière-plan (passage au 1er ou au 27 décembre)
document.addEventListener('visibilitychange', () => { if (document.visibilityState === 'visible' && !THEMES[settings.theme]) applyTheme(); });
$('btnPhoto').onclick = pickPhoto;
$('btnPhotoDel').onclick = () => { loadAccounts(); const a = me(); if (a) { a.emoji = null; saveAccount(a); renderDash(); if (a.cloud) api('PATCH', '/api/me', { emoji: null }, a.tok).catch(() => {}); } };
$('volRange').oninput = e => { settings.volume = +e.target.value / 100; saveSettings(); };
$('volRange').onchange = () => Snd.play('hit');
$('sndToggle').onclick = () => { settings.sound = !settings.sound; saveSettings(); renderSettings(); Snd.play('click'); };
const setLang = v => { settings.lang = I18N[v] ? v : 'fr'; saveSettings(); applyLang(); if ($('dash').classList.contains('active')) renderDash(); else if ($('auth').classList.contains('active')) openAuth(); };
$('langSel').onchange = e => setLang(e.target.value);
$('langMini').onchange = e => setLang(e.target.value);
$('btnSaveProfile').onclick = saveProfile;
$('btnDelete').onclick = deleteAccount;
$('btnBkFile').onclick = bkExportFile;
$('btnBkCopy').onclick = bkExportCopy;
$('btnBkImport').onclick = bkImport;
$('btnBkPick').onclick = () => $('bkFile').click();
$('bkFile').onchange = e => { const f = e.target.files[0]; if (!f || f.size > 30000) return; f.text().then(x => { $('bkCode').value = x.trim(); }); e.target.value = ''; };
$('mRandom').onclick = () => { Snd.play('click'); findRandom(); };
$('mBot').onclick = () => { Snd.play('click'); if (!settings.bot) return msg('dashMsg', t('botOff'), true); startBot(); };
$('mJoin').onclick = () => { Snd.play('click'); openJoin(''); };
$('mHost').onclick = () => { Snd.play('click'); hostGame(); };

// Rejoindre
$('btnJoin').onclick = joinGame;
$('btnJoinCancel').onclick = () => {
  if (S.peer && !S.connected) { try { S.peer.destroy(); } catch (e) {} S.peer = null; S.conn = null; clearTimeout(S.joinTimer); }
  $('joinModal').classList.remove('show');
};
$('codeInput').addEventListener('keydown', e => { if (e.key === 'Enter') joinGame(); });
$('codeInput').addEventListener('input', e => { e.target.value = e.target.value.toUpperCase(); msg('joinMsg', ''); });

// Salon
const copy = async (text) => {
  try { await navigator.clipboard.writeText(text); msg('lobbyMsg', t('copied'), 'ok'); }
  catch (e) { msg('lobbyMsg', t('copyFail'), true); }
};
$('btnCopy').onclick = () => copy(S.code);
$('btnCopyLink').onclick = () => copy(location.origin + location.pathname + '?code=' + S.code);

// Placement et partie
$('btnRotate').onclick = rotateShip;
$('btnRandom').onclick = randomFleet;
$('btnClear').onclick = clearFleet;
$('btnConfirm').onclick = confirmPlacement;
$('btnRemove').onclick = removeShip;
$('btnReady').onclick = setReady;
document.querySelectorAll('.btnQuit').forEach(b => { b.onclick = quitGame; });
document.querySelectorAll('#modes button').forEach(b => { b.onclick = () => setMode(b.dataset.mode); });
// La barre d'actions fixe change de hauteur (liste du Bouclier, texte d'aide) : la page garde la place qu'il faut dessous.
// Une barre cachée (écran non affiché) mesure 0 px : on garde alors la dernière hauteur connue (ou la valeur par défaut du CSS),
// sinon la page n'aurait presque plus de marge sous la barre pendant l'instant où elle réapparaît.
const watchBarHeight = (el, cssVar) => new ResizeObserver(() => { if (el.offsetHeight) document.documentElement.style.setProperty(cssVar, el.offsetHeight + 'px'); }).observe(el);
if (window.ResizeObserver) { watchBarHeight($('actBar'), '--actbar-h'); watchBarHeight($('placeBar'), '--placebar-h'); }
$('shieldPick').onclick = e => { const b = e.target.closest('button[data-ship]'); if (b) protectShip(+b.dataset.ship); };
$('btnSeeGrids').onclick = () => $('end').classList.remove('show');
$('btnBackEnd').onclick = () => $('end').classList.add('show');
$('btnShare').onclick = $('btnShare2').onclick = shareResult;
$('btnRematch').onclick = askRematch;
$('btnNewGame').onclick = leaveToMenu;

// Raccourcis clavier pendant le placement (PC) : R pivote, Entrée pose, Suppr retire.
document.addEventListener('keydown', e => {
  if (S.phase !== 'place' || e.target.tagName === 'INPUT') return;
  if (e.key === 'r' || e.key === 'R') rotateShip();
  else if (e.key === 'Enter') confirmPlacement();
  else if (e.key === 'Delete' || e.key === 'Backspace') removeShip();
});

// Un autre onglet a modifié les comptes : on rafraîchit l'affichage.
window.addEventListener('storage', e => {
  if (e.key !== null && !e.key.startsWith(STORE_KEY)) return;
  if ($('dash').classList.contains('active')) renderDash();
  else if ($('auth').classList.contains('active')) openAuth();
});

// Bloque le zoom par pincement (Safari iOS ignore user-scalable=no).
document.addEventListener('gesturestart', e => e.preventDefault());
document.addEventListener('touchmove', e => { if (e.touches.length > 1) e.preventDefault(); }, { passive: false });
document.addEventListener('visibilitychange', onVisibility);
window.addEventListener('pagehide', e => {
  if (e.persisted) return;
  try { if (S.conn && S.conn.close) S.conn.close(); } catch (e) {}
  try { if (S.peer) S.peer.destroy(); } catch (e) {}
  try { if (S.rpeer) S.rpeer.destroy(); } catch (e) {}
  try { if (S.gpeer) S.gpeer.destroy(); } catch (e) {}
});

// Lien d'invitation (?code=…) : la fenêtre « Rejoindre » s'ouvre dès qu'on arrive au tableau de bord.
pendingCode = (new URLSearchParams(location.search).get('code') || '').toUpperCase().slice(0, 6) || null;
try { if (pendingCode !== null || location.search) history.replaceState(null, '', location.pathname + location.hash); } catch (e) {}
applyTheme();
document.addEventListener('contextmenu', e => { if (!e.target.closest('input,textarea,.code')) e.preventDefault(); });
document.addEventListener('selectstart', e => { if (!e.target.closest || !e.target.closest('input,textarea,.code')) e.preventDefault(); });
try { if (screen.orientation && screen.orientation.lock && /Android|iPhone|iPod/.test(navigator.userAgent)) screen.orientation.lock('portrait').catch(() => {}); } catch (e) {}
applyLang();
loadAccounts();
restoreSession();
// #stats ou #settings dans l'adresse ouvre directement cet onglet.
const startPane = ['play', 'stats', 'rules', 'settings'].includes(location.hash.slice(1)) ? location.hash.slice(1) : 'play';
if (loggedIn) openDash(startPane); else openAuth();
