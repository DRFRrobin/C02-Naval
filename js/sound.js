'use strict';
/* =========================================================
   Effets sonores (synthétisés, aucun fichier à charger)
   ========================================================= */
const Snd = {
  ctx: null, bus: null, rev: null,
  get() {
    if (!this.ctx) {
      const AC = window.AudioContext || window.webkitAudioContext; if (!AC) return null;
      const c = this.ctx = new AC();
      // Bus principal : volume réglable -> compresseur (pas de saturation quand plusieurs sons se superposent)
      this.bus = c.createGain();
      const comp = c.createDynamicsCompressor();
      comp.threshold.value = -16; comp.ratio.value = 4; comp.attack.value = 0.003; comp.release.value = 0.25;
      this.bus.connect(comp).connect(c.destination);
      // Réverbération synthétique (écho d'un grand espace) : bruit à décroissance exponentielle
      const len = Math.floor(c.sampleRate * 1.5), buf = c.createBuffer(2, len, c.sampleRate);
      for (let ch = 0; ch < 2; ch++) { const d = buf.getChannelData(ch); for (let i = 0; i < len; i++) d[i] = (Math.random() * 2 - 1) * Math.pow(1 - i / len, 2.8); }
      this.rev = c.createConvolver(); this.rev.buffer = buf;
      const wet = c.createGain(); wet.gain.value = 0.55; this.rev.connect(wet).connect(this.bus);
    }
    this.bus.gain.value = Math.min(1, Math.max(0, typeof settings.volume === 'number' ? settings.volume : 0.8));
    if (this.ctx.state === 'suspended') this.ctx.resume();
    return this.ctx;
  },
  // sortie commune : panoramique et envoi vers la réverbération
  out(c, node, o) {
    let out = node;
    if (o.pan && c.createStereoPanner) { const p = c.createStereoPanner(); p.pan.value = o.pan; node.connect(p); out = p; }
    out.connect(this.bus);
    if (o.rev) { const r = c.createGain(); r.gain.value = o.rev; out.connect(r).connect(this.rev); }
  },
  // note : attaque courte (pas de « clic »), glissando facultatif, filtre passe-bas facultatif
  tone(c, f, dur, type = 'sine', vol = 0.15, when = 0, to = null, o = {}) {
    const osc = c.createOscillator(), g = c.createGain(), t0 = c.currentTime + when, att = o.att || 0.006;
    osc.type = type; osc.frequency.setValueAtTime(f, t0);
    if (to) osc.frequency.exponentialRampToValueAtTime(to, t0 + dur);
    g.gain.setValueAtTime(0.0001, t0); g.gain.linearRampToValueAtTime(vol, t0 + att); g.gain.exponentialRampToValueAtTime(0.0001, t0 + dur);
    if (o.lp) { const lp = c.createBiquadFilter(); lp.type = 'lowpass'; lp.frequency.value = o.lp; osc.connect(lp).connect(g); } else osc.connect(g);
    this.out(c, g, o);
    osc.start(t0); osc.stop(t0 + dur + 0.05);
  },
  // bruit filtré dont la fréquence glisse de `freq` vers `o.to`
  noise(c, dur, freq, vol = 0.3, when = 0, type = 'lowpass', o = {}) {
    const len = Math.floor(c.sampleRate * dur), b = c.createBuffer(1, len, c.sampleRate), d = b.getChannelData(0);
    for (let i = 0; i < len; i++) d[i] = Math.random() * 2 - 1;
    const s = c.createBufferSource(), f = c.createBiquadFilter(), g = c.createGain(), t0 = c.currentTime + when;
    s.buffer = b; f.type = type; f.Q.value = o.q || 0.7; f.frequency.setValueAtTime(freq, t0);
    if (o.to) f.frequency.exponentialRampToValueAtTime(o.to, t0 + dur);
    g.gain.setValueAtTime(0.0001, t0); g.gain.linearRampToValueAtTime(vol, t0 + (o.att || 0.004)); g.gain.exponentialRampToValueAtTime(0.0001, t0 + dur);
    s.connect(f).connect(g); this.out(c, g, o); s.start(t0);
  },
  // cloche : fondamentale + partiels inharmoniques
  bell(c, f, dur, vol, when = 0, rev = 0.3, pan = 0) {
    [[1, 1], [2.76, 0.38], [5.4, 0.14]].forEach(([m, a]) => this.tone(c, f * m, dur / (m > 1 ? 1.8 : 1), 'sine', vol * a, when, null, { rev, pan }));
  },
  // petites bulles qui remontent
  bubbles(c, when, n, vol = 0.05) {
    for (let i = 0; i < n; i++) { const f = 280 + Math.random() * 520, w = when + i * 0.07 + Math.random() * 0.05; this.tone(c, f, 0.09, 'sine', vol, w, f * 1.9, { pan: Math.random() - 0.5, rev: 0.2 }); }
  },
  sounds: {
    click(c, S) { S.tone(c, 1200, 0.04, 'triangle', 0.05, 0, 750); },
    place(c, S) { S.tone(c, 210, 0.11, 'sine', 0.26, 0, 85); S.noise(c, 0.05, 3200, 0.05, 0, 'highpass'); },
    shot(c, S) {   // tir de canon : souffle, coup sourd et sifflement
      S.noise(c, 0.35, 2600, 0.28, 0, 'lowpass', { to: 260 }); S.tone(c, 180, 0.32, 'sine', 0.38, 0, 42);
      S.noise(c, 0.14, 5200, 0.1, 0, 'highpass'); S.tone(c, 1500, 0.35, 'sine', 0.025, 0.05, 420);
    },
    miss(c, S) { S.noise(c, 0.85, 1500, 0.2, 0, 'bandpass', { to: 340, q: 0.9, rev: 0.35 }); S.tone(c, 150, 0.3, 'sine', 0.1, 0.02, 60); S.bubbles(c, 0.12, 3); },
    hit(c, S) {    // explosion + choc métallique
      S.noise(c, 0.75, 1900, 0.48, 0, 'lowpass', { to: 110, rev: 0.25 }); S.tone(c, 125, 0.6, 'sine', 0.52, 0, 33);
      S.tone(c, 410, 0.3, 'sawtooth', 0.05, 0.01, 300, { lp: 1400 }); S.tone(c, 617, 0.22, 'sawtooth', 0.04, 0.02, 480, { lp: 1600 });
      S.noise(c, 0.06, 4200, 0.12, 0.06, 'highpass'); S.noise(c, 0.06, 4200, 0.09, 0.17, 'highpass');
    },
    sunk(c, S) {   // explosion, seconde détonation, coque qui gémit, bulles
      S.sounds.hit(c, S);
      S.noise(c, 1.5, 800, 0.42, 0.32, 'lowpass', { to: 80, rev: 0.4 }); S.tone(c, 75, 1.3, 'sawtooth', 0.12, 0.3, 24, { lp: 320 });
      S.tone(c, 62, 0.8, 'sine', 0.4, 0.34, 30); S.bubbles(c, 0.7, 7, 0.045);
    },
    blocked(c, S) { S.bell(c, 1568, 0.55, 0.12, 0, 0.45); S.noise(c, 0.12, 6000, 0.06, 0, 'highpass'); },
    radar(c, S) {  // ping de sonar avec échos
      [0, 0.42, 0.84].forEach((w, i) => S.tone(c, 1320, 1.0, 'sine', 0.16 / (1 + i * 1.1), w, 1240, { rev: 0.55 }));
    },
    shield(c, S) { [784, 988, 1319, 1760].forEach((f, i) => S.tone(c, f, 0.4, 'sine', 0.085, i * 0.06, null, { rev: 0.45, pan: (i - 1.5) / 3 })); S.noise(c, 0.35, 5000, 0.04, 0, 'highpass', { rev: 0.4 }); },
    turn(c, S) { S.bell(c, 880, 0.55, 0.1, 0, 0.25); S.bell(c, 1175, 0.65, 0.09, 0.11, 0.25); },
    upgrade(c, S) { [659, 784, 988, 1319].forEach((f, i) => S.bell(c, f, 0.55, 0.08, i * 0.07, 0.3)); },
    xp(c, S) { [880, 1175, 1568].forEach((f, i) => S.tone(c, f, 0.14, 'triangle', 0.06, i * 0.05, null, { rev: 0.2 })); },
    levelup(c, S) {
      [523, 659, 784, 1047, 1319, 1568].forEach((f, i) => S.bell(c, f, 0.9, 0.09, i * 0.09, 0.45));
      [523, 784, 1047].forEach(f => S.tone(c, f, 1.3, 'triangle', 0.06, 0.62, null, { rev: 0.5 }));
    },
    win(c, S) {    // fanfare de cuivres puis accord final
      [[523, 0], [659, 0.15], [784, 0.3], [1047, 0.46]].forEach(([f, w]) => { S.tone(c, f, 0.34, 'sawtooth', 0.075, w, null, { lp: 2400, att: 0.02, rev: 0.25 }); S.tone(c, f / 2, 0.34, 'triangle', 0.08, w); });
      [523, 659, 784, 1047].forEach(f => S.tone(c, f, 1.5, 'triangle', 0.07, 0.8, null, { att: 0.03, rev: 0.5 }));
    },
    lose(c, S) {   // notes descendantes et bourdon grave
      [392, 370, 330, 262].forEach((f, i) => S.tone(c, f, 0.55, 'triangle', 0.1, i * 0.24, null, { lp: 1800, att: 0.03, rev: 0.4 }));
      S.tone(c, 98, 1.6, 'sine', 0.12, 0.25, 82, { att: 0.12 });
    },
    start(c, S) {  // corne de brume navale
      S.tone(c, 196, 0.5, 'sawtooth', 0.075, 0, null, { lp: 700, att: 0.07, rev: 0.25 });
      S.tone(c, 294, 0.6, 'sawtooth', 0.065, 0.2, null, { lp: 800, att: 0.07, rev: 0.25 });
    },
  },
  play(name) {
    if (!settings.sound) return;
    try {
      const fn = this.sounds[name]; if (!fn) return;
      const c = this.get(); if (!c) return;
      fn(c, this);
    } catch (e) {}
  },
  buzz(pattern) { if (settings.sound && navigator.vibrate) try { navigator.vibrate(pattern); } catch (e) {} },
};
// iOS / Chrome n'autorisent le son qu'après un geste : on « réveille » l'audio dès la première interaction.
['pointerdown', 'keydown'].forEach(ev => document.addEventListener(ev, () => { if (settings.sound) try { Snd.get(); } catch (e) {} }, { once: true, passive: true }));
