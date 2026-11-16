'use strict';
/* =========================================================
   Partage du résultat : image de fin de partie
   Une image 1080 × 1350 (format portrait des réseaux sociaux) dessinée dans un canvas, aux couleurs du thème :
   le gagnant, les deux grilles côte à côte (sa flotte avec les tirs reçus, la flotte adverse avec ses tirs
   et, si on les connaît, les bateaux non trouvés), puis les chiffres du match.
   L'image est préparée dès la fin de la partie : sur iPhone, le partage doit partir tout de suite après l'appui.
   ========================================================= */
const IMG_W = 1080, IMG_H = 1350;
const SHARE_NAME = 'c02-naval-resultat.png';
let shareJob = null;   // promesse du fichier image de la partie terminée

function prepareShare() {
  if (S.phase !== 'over' || S.lost) { shareJob = null; return; }
  const round = S.rounds || 0;
  shareJob = resultImage().then(blob => ((S.rounds || 0) === round && blob ? new File([blob], SHARE_NAME, { type: 'image/png' }) : null)).catch(() => null);
}

// Texte qui accompagne l'image (et repli si l'image ne peut pas être créée).
function shareCaption() {
  const url = /^https?:$/.test(location.protocol) ? location.origin + location.pathname : '';
  return '⚓ C02 Naval — ' + t(S.win ? 'shareWin' : 'shareLose', { name: S.opp ? S.opp.name : '?' }) + (url ? '\n' + t('shareInvite') + ' ' + url : '');
}

async function shareResult() {
  if (S.phase !== 'over' || S.lost) return;
  Snd.play('click');
  if (!shareJob) prepareShare();
  const file = await shareJob;
  const text = shareCaption();
  if (file && navigator.canShare && navigator.share) {
    let ok = false; try { ok = navigator.canShare({ files: [file] }); } catch (e) {}
    if (ok) {
      try { await navigator.share({ files: [file], title: 'C02 Naval', text }); msg('shareMsg', ''); return; }
      catch (e) { if (e && e.name === 'AbortError') return; }   // partage annulé par le joueur
    }
  }
  if (file) {   // pas de partage d'image (ordinateur) : l'image est téléchargée
    const url = URL.createObjectURL(file);
    const a = document.createElement('a');
    a.href = url; a.download = SHARE_NAME; document.body.appendChild(a); a.click(); a.remove();
    setTimeout(() => URL.revokeObjectURL(url), 30000);
    msg('shareMsg', t('shareSaved'), 'ok');
    return;
  }
  try {   // dernier recours : le texte
    if (navigator.share) { await navigator.share({ title: 'C02 Naval', text }); return; }
    await navigator.clipboard.writeText(text); msg('shareMsg', t('shareCopied'), 'ok');
  } catch (e) { if (!e || e.name !== 'AbortError') msg('shareMsg', t('shareFail'), true); }
}

/* ---------- Dessin ---------- */
function themeColors() {
  const cs = getComputedStyle(document.documentElement), v = k => cs.getPropertyValue(k).trim();
  return { bg1: v('--bg1'), bg2: v('--bg2'), bg3: v('--bg3'), card: v('--modal'), border: v('--border2'), text: v('--text'), muted: v('--muted'),
    water: v('--water'), ship: v('--ship'), hit: v('--hit'), sunk: v('--sunk'), miss: v('--miss'), blk: v('--blk'), scan: v('--scan'),
    yellow: v('--yellow'), sky: v('--sky'), green: v('--green') };
}
function rrect(ctx, x, y, w, h, r) {
  ctx.beginPath();
  ctx.moveTo(x + r, y); ctx.arcTo(x + w, y, x + w, y + h, r); ctx.arcTo(x + w, y + h, x, y + h, r);
  ctx.arcTo(x, y + h, x, y, r); ctx.arcTo(x, y, x + w, y, r); ctx.closePath();
}
// Texte réduit si besoin pour tenir dans la largeur donnée.
function fitText(ctx, text, x, y, maxW, size, weight) {
  let s = size;
  do { ctx.font = `${weight} ${s}px Inter, system-ui, sans-serif`; s -= 2; } while (ctx.measureText(text).width > maxW && s > 12);
  ctx.fillText(text, x, y);
}
function markX(ctx, x, y, w, color) {
  const p = w * 0.3;
  ctx.strokeStyle = color; ctx.lineWidth = Math.max(2, w * 0.11); ctx.lineCap = 'round';
  ctx.beginPath(); ctx.moveTo(x + p, y + p); ctx.lineTo(x + w - p, y + w - p); ctx.moveTo(x + w - p, y + p); ctx.lineTo(x + p, y + w - p); ctx.stroke();
}
function markDot(ctx, x, y, w, color) {
  ctx.fillStyle = color; ctx.beginPath(); ctx.arc(x + w / 2, y + w / 2, Math.max(2, w * 0.11), 0, Math.PI * 2); ctx.fill();
}
// Une case : fond, contour éventuel, marque (✕ touché / coulé, • dans l'eau).
function drawCell(ctx, C, x, y, w, kind) {
  const fill = { ship: C.ship, hit: C.hit, sunk: C.sunk, blocked: C.blk, scan: C.scan, missed: C.ship }[kind] || C.water;
  ctx.globalAlpha = kind === 'missed' ? 0.6 : 1;
  ctx.fillStyle = fill; rrect(ctx, x, y, w, w, Math.max(2, w * 0.12)); ctx.fill();
  ctx.globalAlpha = 1;
  if (kind === 'missed') { ctx.strokeStyle = C.yellow; ctx.lineWidth = Math.max(2, w * 0.08); rrect(ctx, x + 1.5, y + 1.5, w - 3, w - 3, Math.max(2, w * 0.12)); ctx.stroke(); }
  if (kind === 'hit') markX(ctx, x, y, w, '#ffffff');
  if (kind === 'sunk') markX(ctx, x, y, w, '#fecaca');
  if (kind === 'miss') markDot(ctx, x, y, w, C.miss);
}
function drawGrid(ctx, C, x, y, size, kindOf) {
  const lab = 26, cell = (size - lab) / N, gap = Math.max(1.5, cell * 0.08), w = cell - gap;
  ctx.fillStyle = C.muted; ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
  ctx.font = `600 ${Math.round(Math.min(18, cell * 0.42))}px Inter, system-ui, sans-serif`;
  for (let i = 0; i < N; i++) {
    ctx.fillText(String(i + 1), x + lab + i * cell + w / 2, y - 14);
    ctx.fillText(ROWS[i], x + lab / 2 - 2, y + i * cell + w / 2);
  }
  for (let r = 0; r < N; r++) for (let c = 0; c < N; c++) drawCell(ctx, C, x + lab + c * cell, y + r * cell, w, kindOf(r * N + c));
}

async function resultImage() {
  try { if (document.fonts && document.fonts.ready) await Promise.race([document.fonts.ready, new Promise(r => setTimeout(r, 800))]); } catch (e) {}
  const C = themeColors();
  const cv = document.createElement('canvas'); cv.width = IMG_W; cv.height = IMG_H;
  const ctx = cv.getContext('2d');
  const meId = myIdentity(), oppName = S.opp ? S.opp.name : '?', winner = S.win ? meId.name : oppName;

  // Fond
  const g = ctx.createLinearGradient(0, 0, 0, IMG_H);
  g.addColorStop(0, C.bg1); g.addColorStop(0.55, C.bg2); g.addColorStop(1, C.bg3);
  ctx.fillStyle = g; ctx.fillRect(0, 0, IMG_W, IMG_H);

  // En-tête : nom du jeu, gagnant, affiche du match
  ctx.textAlign = 'center'; ctx.textBaseline = 'alphabetic';
  ctx.fillStyle = C.sky; ctx.font = '700 34px Orbitron, Inter, system-ui, sans-serif';
  ctx.fillText('⚓ C02 NAVAL', IMG_W / 2, 92);
  ctx.font = '96px system-ui, sans-serif';
  ctx.fillText(S.win ? '🏆' : '🌊', IMG_W / 2, 210);
  ctx.fillStyle = C.yellow;
  fitText(ctx, t('imgWinner', { name: winner }), IMG_W / 2, 296, IMG_W - 120, 64, 800);
  ctx.fillStyle = C.muted;
  fitText(ctx, `${meId.name}  ${t('vs')}  ${oppName}` + (S.forfeit ? ' · ' + t('imgForfeit') : ''), IMG_W / 2, 350, IMG_W - 120, 30, 600);

  // Deux grilles côte à côte
  const pad = 50, gapX = 40, colW = (IMG_W - pad * 2 - gapX) / 2, top = 470;
  const occ = occupancy(), opp = oppShipCells();
  const mine = k => {
    const i = occ[k], v = S.incoming[k];
    if (i >= 0 && isSunk(S.ships[i])) return 'sunk';
    if (v === 'hit') return 'hit';
    if (v === 'blocked') return 'blocked';
    if (v === 'miss') return 'miss';
    return i >= 0 ? 'ship' : '';
  };
  const theirs = k => {
    const v = S.enemy[k];
    if (v === 'sunk' || v === 'hit' || v === 'miss' || v === 'blocked') return v;
    if (opp.has(k)) return 'missed';
    if (v === 'scanOcc') return 'scan';
    return '';
  };
  const cols = [[meId.name, mine, !!S.win], [oppName, theirs, !S.win]];
  cols.forEach(([name, kindOf, won], i) => {
    const x = pad + i * (colW + gapX);
    ctx.fillStyle = C.card; ctx.globalAlpha = 0.85; rrect(ctx, x - 14, top - 96, colW + 28, colW + 116, 22); ctx.fill(); ctx.globalAlpha = 1;
    ctx.strokeStyle = won ? C.yellow : C.border; ctx.lineWidth = won ? 4 : 2; rrect(ctx, x - 14, top - 96, colW + 28, colW + 116, 22); ctx.stroke();
    ctx.textAlign = 'center'; ctx.textBaseline = 'alphabetic'; ctx.fillStyle = won ? C.yellow : C.text;
    fitText(ctx, (won ? '👑 ' : '') + t('imgFleet', { name }), x + colW / 2, top - 52, colW - 10, 30, 700);
    drawGrid(ctx, C, x, top, colW, kindOf);
  });

  // Légende
  const legend = [['ship', t('lgShip')], ['hit', t('lgHit')], ['sunk', t('lgSunk')], ['miss', t('lgMiss')]].concat(opp.size ? [['missed', t('lgMissed')]] : []);
  ctx.font = '600 24px Inter, system-ui, sans-serif'; ctx.textBaseline = 'middle'; ctx.textAlign = 'left';
  const items = legend.map(([k, label]) => ({ k, label, w: 30 + 10 + ctx.measureText(label).width }));
  const lgW = items.reduce((s, it) => s + it.w, 0) + 28 * (items.length - 1);
  let lx = (IMG_W - lgW) / 2; const ly = top + colW + 66;
  for (const it of items) {
    drawCell(ctx, C, lx, ly - 15, 30, it.k);
    ctx.fillStyle = C.muted; ctx.font = '600 24px Inter, system-ui, sans-serif'; ctx.textAlign = 'left'; ctx.textBaseline = 'middle';
    ctx.fillText(it.label, lx + 40, ly);
    lx += it.w + 28;
  }

  // Chiffres du match (côté du joueur qui partage)
  const m = S.match, acc = m.shots ? Math.round(m.hits / m.shots * 100) : 0;
  const time = fmtTime(m.start ? ((m.end || Date.now()) - m.start) / 1000 : 0);
  const stats = [[t('imgShots'), String(m.shots)], [t('imgAcc'), acc + ' %'], [t('imgSunk'), `${S.enemySunk.length} / ${LENS.length}`], [t('imgTime'), time]];
  const sy = ly + 60;
  ctx.textAlign = 'center'; ctx.textBaseline = 'alphabetic'; ctx.fillStyle = C.muted;
  fitText(ctx, t('imgStatsOf', { name: meId.name }), IMG_W / 2, sy + 10, IMG_W - 120, 24, 600);
  const bw = (IMG_W - pad * 2 - 3 * 18) / 4;
  stats.forEach(([label, val], i) => {
    const x = pad + i * (bw + 18), y = sy + 30;
    ctx.fillStyle = C.card; ctx.globalAlpha = 0.85; rrect(ctx, x, y, bw, 120, 18); ctx.fill(); ctx.globalAlpha = 1;
    ctx.strokeStyle = C.border; ctx.lineWidth = 2; rrect(ctx, x, y, bw, 120, 18); ctx.stroke();
    ctx.fillStyle = C.text; ctx.textAlign = 'center';
    fitText(ctx, val, x + bw / 2, y + 62, bw - 20, 44, 800);
    ctx.fillStyle = C.muted;
    fitText(ctx, label, x + bw / 2, y + 100, bw - 20, 22, 600);
  });

  // Pied : lien du jeu
  const url = /^https?:$/.test(location.protocol) ? (location.host + location.pathname).replace(/\/$/, '') : '';
  ctx.fillStyle = C.sky; ctx.textAlign = 'center';
  fitText(ctx, url ? t('shareInvite') + ' ' + url : 'C02 Naval', IMG_W / 2, IMG_H - 40, IMG_W - 100, 28, 600);

  return new Promise(res => cv.toBlob(b => res(b), 'image/png'));
}
