'use strict';
/* =========================================================
   Placement : aperçu, confirmation, glisser-déposer, pivoter
   ========================================================= */
function enterPlacement() {
  S.phase = 'place';
  $('rulesInfo').textContent = t('rulesInfo', { size: N, n: LENS.length, up: anyUpgrade() }) + (CFG.timer ? ' ' + t('placeLimit', { s: PLACE_LIMIT_MS / 1000 }) + ' ' + t('turnLimit', { s: CFG.timer }) : '');
  S.placeDeadline = CFG.timer ? Date.now() + PLACE_LIMIT_MS : 0;
  if (S.placeDeadline) {
    clearInterval(S.turnTimer);
    S.turnTimer = setInterval(() => {
      if (S.phase !== 'place') return clearInterval(S.turnTimer);
      if (!S.meReady && !S.reconnecting && Date.now() > S.placeDeadline) {   // temps écoulé : les bateaux restants sont posés au hasard
        settleLifted();
        fillMissingShips();
        setReady();
      }
    }, 1000);
  }
  S.sel = 0; S.preview = null;
  renderVs();
  showView('place');
  renderPlace();
  Snd.play('start');
}
// Pose au hasard les bateaux manquants autour de ceux déjà placés (sinon, flotte entièrement aléatoire).
function fillMissingShips() {
  for (let a = 0; a < 100; a++) {
    const ships = S.ships.map(sh => ({ len: sh.len, cells: sh.cells }));
    let ok = true;
    for (const sh of ships) {
      if (sh.cells) continue;
      const occ = occOf(ships);
      for (let tr = 0; tr < 200 && !sh.cells; tr++) {
        const h = Math.random() < 0.5, r = Math.floor(Math.random() * N), c = Math.floor(Math.random() * N), cells = [];
        for (let i = 0; i < sh.len; i++) {
          const rr = r + (h ? 0 : i), cc = c + (h ? i : 0);
          if (!inside(rr, cc) || occ[rr * N + cc] !== -1) break;
          cells.push([rr, cc]);
        }
        if (cells.length === sh.len) sh.cells = cells;
      }
      if (!sh.cells) { ok = false; break; }
    }
    if (ok) { S.ships.forEach((sh, i) => { sh.cells = ships[i].cells; }); return; }
  }
  randomCells(LENS).forEach((cells, i) => { S.ships[i].cells = cells; });
}
const nextUnplaced = () => S.ships.findIndex(s => !s.cells);
const shipHoriz = cells => cells.length < 2 || cells[0][0] === cells[1][0];

// Cases de l'aperçu (fantôme) du bateau sélectionné, et validité de la position.
function previewInfo() {
  if (!S.preview || S.sel < 0) return null;
  const s = S.ships[S.sel], { r, c, h } = S.preview, occ = occupancy();
  const cells = [];
  let valid = true;
  for (let i = 0; i < s.len; i++) {
    const rr = r + (h ? 0 : i), cc = c + (h ? i : 0);
    if (!inside(rr, cc)) { valid = false; continue; }
    if (occ[rr * N + cc] !== -1) valid = false;
    cells.push([rr, cc]);
  }
  return { cells, valid };
}
const placeHint = (text, err) => msg('placeMsg', text, err);

// Termine le déplacement d'un bateau soulevé : on le pose à l'aperçu si possible, sinon on le remet.
function settleLifted() {
  if (!S.lifted) return;
  const s = S.ships[S.lifted.idx];
  if (!s.cells) {
    const pv = S.sel === S.lifted.idx ? previewInfo() : null;
    s.cells = pv && pv.valid ? pv.cells : S.lifted.cells;
  }
  S.lifted = null; S.preview = null;
}
function selectShip(i) {
  if (S.meReady) return;
  if (S.lifted && S.lifted.idx === i) return;
  settleLifted();
  S.sel = i;
  const s = S.ships[i];
  if (s.cells) {
    // Bateau déjà posé : on le soulève pour pouvoir le déplacer, le pivoter ou le retirer.
    S.lifted = { idx: i, cells: s.cells };
    S.preview = { r: s.cells[0][0], c: s.cells[0][1], h: shipHoriz(s.cells) };
    S.horiz = S.preview.h;
    s.cells = null;
    placeHint(t('pSelPlaced', { ship: shipName(i) }));
  } else {
    S.preview = null;
    placeHint(t('pSelNew', { ship: shipName(i) }));
  }
  Snd.play('click');
  renderPlace();
}
function confirmPlacement() {
  const pv = previewInfo();
  if (!pv) return;
  if (!pv.valid) return placeHint(t('pBad'), true);
  const s = S.ships[S.sel];
  s.cells = pv.cells;
  S.lifted = null; S.preview = null;
  S.sel = nextUnplaced();
  placeHint(S.sel >= 0 ? t('pPlacedNext', { ship: shipName(s.i), next: shipName(S.sel) }) : t('pDone'), S.sel >= 0 ? '' : 'ok');
  Snd.play('place');
  renderPlace();
}
function tapPlaceCell(r, c) {
  if (S.meReady) return;
  const pv = previewInfo();
  if (pv && pv.cells.some(([a, b]) => a === r && b === c)) return confirmPlacement();   // toucher le fantôme le pose
  const idx = occupancy()[r * N + c];
  if (idx >= 0) return selectShip(idx);
  if (S.sel < 0 || (S.ships[S.sel].cells && !(S.lifted && S.lifted.idx === S.sel))) {
    S.sel = nextUnplaced();
    if (S.sel < 0) return placeHint(t('pAllPlaced'));
  }
  S.preview = { r, c, h: S.preview ? S.preview.h : S.horiz };
  const np = previewInfo();
  placeHint(np.valid ? t('pTapGhost') : t('pNoFit'), !np.valid);
  Snd.play('click');
  renderPlace();
}
function rotateShip() {
  if (S.meReady) return;
  if (S.preview) {
    S.preview.h = !S.preview.h;
    if (!previewInfo().valid) {   // s'il sort de la grille en pivotant, on le recale à l'intérieur
      const s = S.ships[S.sel];
      if (S.preview.h) S.preview.c = Math.min(S.preview.c, N - s.len); else S.preview.r = Math.min(S.preview.r, N - s.len);
    }
    const p2 = previewInfo();
    placeHint(p2.valid ? '' : t('pBlocked'), !p2.valid);
  }
  S.horiz = S.preview ? S.preview.h : !S.horiz;
  Snd.play('click');
  renderPlace();
}
function removeShip() {
  if (S.meReady || !S.lifted) return;
  const i = S.lifted.idx;
  S.lifted = null; S.preview = null;
  placeHint(t('pRemoved', { ship: shipName(i) }));
  renderPlace();
}
function clearFleet() {
  if (S.meReady) return;
  S.ships.forEach(s => { s.cells = null; });
  S.lifted = null; S.preview = null; S.sel = 0;
  placeHint(t('pCleared'));
  renderPlace();
}
function randomFleet() {
  if (S.meReady) return;
  S.lifted = null; S.preview = null;
  randomCells(LENS).forEach((cells, i) => { S.ships[i].cells = cells; });
  S.sel = -1;
  placeHint(t('pRandom'));
  Snd.play('place');
  renderPlace();
}
function setReady() {
  if (S.meReady || nextUnplaced() !== -1 || S.lifted) return;
  S.meReady = true; S.sel = -1; S.preview = null;
  sendReady();
  Snd.play('click');
  renderPlace();
  maybeStart();
}

// Glisser-déposer (souris et doigt) sur la grille de placement.
function placeCellAt(x, y) {
  const el = document.elementFromPoint(x, y);
  const d = el && el.closest && el.closest('#placeGrid .cell');
  return d ? [+d.dataset.r, +d.dataset.c] : null;
}
function onPlaceDown(e) {
  if (S.meReady || (e.pointerType === 'mouse' && e.button !== 0)) return;
  const p = placeCellAt(e.clientX, e.clientY); if (!p) return;
  const [r, c] = p;
  const pv = previewInfo();
  const onPreview = !!pv && pv.cells.some(([a, b]) => a === r && b === c);
  const idx = occupancy()[r * N + c];
  S.drag = { r, c, moved: false, ship: false, wasSelected: onPreview };
  if (idx >= 0 && !onPreview) { selectShip(idx); S.drag.ship = true; }   // soulève le bateau touché
  else if (onPreview) S.drag.ship = true;
  if (S.drag.ship) S.drag.off = [r - S.preview.r, c - S.preview.c];
  try { $('placeGrid').setPointerCapture(e.pointerId); } catch (_) {}
  e.preventDefault();
}
function onPlaceMove(e) {
  const d = S.drag; if (!d || !d.ship || !S.preview) return;
  const p = placeCellAt(e.clientX, e.clientY); if (!p) return;
  if (p[0] === d.r && p[1] === d.c && !d.moved) return;
  d.moved = true;
  S.preview.r = p[0] - d.off[0]; S.preview.c = p[1] - d.off[1];
  const pv = previewInfo();
  placeHint(pv.valid ? t('pRelease') : t('pNotHere'), !pv.valid);
  renderPlace();
}
function onPlaceUp() {
  const d = S.drag; S.drag = null;
  if (!d) return;
  if (d.ship && d.moved) {
    const pv = previewInfo();
    if (pv && pv.valid) return confirmPlacement();
    const i = S.sel;   // position refusée : on remet le bateau à sa place d'origine
    if (S.lifted) { S.ships[S.lifted.idx].cells = S.lifted.cells; S.lifted = null; S.preview = null; }
    placeHint(t('pRestored', { ship: i >= 0 ? shipName(i) : '' }), true);
    return renderPlace();
  }
  if (d.ship && !d.wasSelected) return;   // simple appui sur un bateau posé : il reste sélectionné
  tapPlaceCell(d.r, d.c);
}

function renderPlace() {
  if (S.phase !== 'place') return;
  const occ = occupancy(), pv = previewInfo();
  const pvSet = new Set(pv ? pv.cells.map(([r, c]) => r * N + c) : []);
  $('placeGrid').cells.forEach((d, k) => {
    let cls = 'cell';
    if (pvSet.has(k)) cls += pv.valid ? ' pv-ok' : ' pv-bad';
    else if (occ[k] >= 0) cls += ' ship';
    d.className = cls;
  });
  $('shipList').innerHTML = '';
  S.ships.forEach((s, i) => {
    const b = document.createElement('button');
    const placed = !!s.cells || (S.lifted && S.lifted.idx === i);
    b.className = 'shipbtn' + (i === S.sel && !S.meReady ? ' sel' : '') + (placed ? ' placed' : '');
    b.innerHTML = `<span>${esc(shipName(i))}</span><span class="blocks">${'<i></i>'.repeat(s.len)}</span>`;
    b.disabled = S.meReady;
    b.onclick = () => selectShip(i);
    $('shipList').appendChild(b);
  });
  const complete = nextUnplaced() === -1 && !S.lifted;
  $('btnReady').disabled = !complete || S.meReady;
  $('btnRandom').disabled = $('btnRotate').disabled = $('btnClear').disabled = S.meReady;
  $('btnConfirm').disabled = S.meReady || !pv || !pv.valid;
  $('btnRemove').disabled = S.meReady || !S.lifted;
  $('rotLbl').textContent = t((S.preview ? S.preview.h : S.horiz) ? 'rotH' : 'rotV');
  if (S.meReady) msg('placeMsg', S.oppReady ? '' : t('waitOppReady'));
  else if (S.oppReady && !$('placeMsg').textContent) msg('placeMsg', t('oppReady'));
}
function maybeStart() {
  if (S.phase !== 'place' || !S.meReady || !S.oppReady) return;
  S.phase = 'game';
  S.myTurn = S.isHost !== ((S.rounds || 0) % 2 === 1); S.turnStart = Date.now();
  S.match.start = Date.now();
  startTurnTimer();
  showView('game');
  setLog(t(S.myTurn ? 'startHost' : 'startGuest'));
  if (S.vsBot && !S.myTurn && S.bot) S.bot.later(() => S.bot.move(), 900);   // revanche : le bot ouvre le feu
  Snd.play(S.myTurn ? 'turn' : 'start');
  renderGame();
}
