'use strict';
/* =========================================================
   Bot tactique : il calcule, pour chaque case, combien de positions
   de bateaux restants y passent (et privilégie celles qui prolongent
   un coup au but), puis tire sur la case la plus probable.
   ========================================================= */
let BOT_SPEED = 1;   // 1 = délais naturels ; réduit par les tests automatiques
class Bot {
  constructor() {
    this.ships = randomCells(LENS).map((cells, i) => ({ i, len: LENS[i], cells, shield: 'none' }));
    this.incoming = Array(N * N).fill(null);
    this.know = Array(N * N).fill(null);   // ce que le bot sait de ta grille : null | miss | hit | sunk | blocked
    this.remaining = LENS.slice();
    this.timers = [];
    this.skill = settings.skill;
  }
  later(fn, ms) { this.timers.push(setTimeout(() => { if (S.vsBot && S.phase !== 'over') fn(); }, ms * BOT_SPEED)); }
  emit(m, ms) { this.later(() => onData(m), ms); }
  start() {
    this.emit({ t: 'hello', name: t('botName'), emoji: '🤖', level: myIdentity().level || 1, bot: true }, 10);
    this.emit({ t: 'ready' }, 900 + Math.random() * 900);
  }
  receive(m) {
    if (m.t === 'act') {
      const a = normalizeAct(m, true); if (!a) return;
      const { kind, cells } = a;
      const { results, over } = resolveOn(this.ships, this.incoming, kind, cells);
      this.emit({ t: 'res', kind, at: a.at, results, over }, 450);
      if (!over) this.later(() => this.move(), 450 + 700 + Math.random() * 600);
    } else if (m.t === 'pass') {
      this.later(() => this.move(), 900);
    } else if (m.t === 'res') {
      for (const x of (m.results || [])) {
        const k = x.r * N + x.c;
        if (x.v === 'sunk') {
          (x.ship || []).forEach(([r, c]) => { this.know[r * N + c] = 'sunk'; });
          const idx = this.remaining.indexOf((x.ship || []).length);
          if (idx >= 0) this.remaining.splice(idx, 1);
        } else if (x.v === 'miss' || x.v === 'hit' || x.v === 'blocked') this.know[k] = x.v;
      }
    }
  }
  move() {
    if (S.phase !== 'game') return;
    const [r, c] = this.choose();
    onData({ t: 'act', kind: 'shot', at: [r, c], cells: [[r, c]] });
  }
  randomUnknown() {
    const free = []; this.know.forEach((v, k) => { if (v === null) free.push(k); });
    const k = free[Math.floor(Math.random() * free.length)];
    return [Math.floor(k / N), k % N];
  }
  choose() {
    const K = this.know;
    if (this.skill === 0) {   // facile : tirs au hasard, avec un peu de bon sens autour d'un touché
      const near = [];
      K.forEach((v, k) => { if (v !== 'hit') return; const r = Math.floor(k / N), c = k % N;
        [[r - 1, c], [r + 1, c], [r, c - 1], [r, c + 1]].forEach(([a, b]) => { if (inside(a, b) && K[a * N + b] === null) near.push([a, b]); }); });
      if (near.length && Math.random() < 0.6) return near[Math.floor(Math.random() * near.length)];
      return this.randomUnknown();
    }
    if (this.skill === 1 && Math.random() < 0.3) return this.randomUnknown();
    const blocked = K.map((v, k) => (v === 'blocked' ? k : -1)).filter(k => k >= 0);
    if (blocked.length) { const k = blocked[0]; return [Math.floor(k / N), k % N]; }   // bouclier : on retire au même endroit
    const hasHits = K.includes('hit');
    const score = Array(N * N).fill(0);
    for (const len of this.remaining) {
      for (let r = 0; r < N; r++) for (let c = 0; c < N; c++) for (const h of [true, false]) {
        const cells = []; let ok = true, nh = 0;
        for (let i = 0; i < len; i++) {
          const rr = r + (h ? 0 : i), cc = c + (h ? i : 0);
          if (!inside(rr, cc)) { ok = false; break; }
          const v = K[rr * N + cc];
          if (v === 'miss' || v === 'sunk') { ok = false; break; }
          if (v === 'hit') nh++;
          cells.push(rr * N + cc);
        }
        if (!ok || (hasHits && !nh)) continue;
        const w = hasHits ? Math.pow(12, nh) : 1;
        cells.forEach(k => { if (K[k] === null) score[k] += w; });
      }
    }
    let best = -1, bestK = -1;
    for (let k = 0; k < N * N; k++) {
      if (K[k] !== null) continue;
      const parity = !hasHits && ((Math.floor(k / N) + k % N) % 2 === 0) ? 1.25 : 1;
      const v = score[k] * parity + Math.random() * 0.5;
      if (v > best) { best = v; bestK = k; }
    }
    if (bestK < 0) bestK = K.indexOf(null);
    return [Math.floor(bestK / N), bestK % N];
  }
  stop() { this.timers.forEach(clearTimeout); }
}
function startBot(fromRandom) {
  if (!settings.bot) return;
  applyConfig(fromRandom ? DEFAULT_RULES : RULES);
  S.kind = 'bot'; S.vsBot = true; S.isHost = true;
  S.bot = new Bot();
  S.connected = true;
  S.opp = { name: t('botName'), emoji: '🤖', level: myIdentity().level || 1, bot: true };
  enterPlacement();
  S.bot.start();
}
