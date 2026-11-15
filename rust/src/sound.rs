//! Effets sonores synthétisés (aucun fichier audio), reprenant ceux de la version web :
//! oscillateurs (sinus, carré, triangle, dent de scie) et bruit filtré, avec
//! enveloppes exponentielles.

use std::f32::consts::PI;
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rand::Rng;

#[derive(Clone, Copy)]
enum Wave {
    Sine,
    Square,
    Triangle,
    Saw,
}

#[derive(Clone, Copy)]
enum Filter {
    Low,
    High,
}

struct Voice {
    data: Vec<f32>,
    pos: usize,
}

type Mixer = Arc<Mutex<Vec<Voice>>>;

pub struct Sound {
    stream: Option<cpal::Stream>,
    mixer: Mixer,
    rate: f32,
    tried: bool,
}

impl Sound {
    pub fn new() -> Sound {
        Sound { stream: None, mixer: Arc::default(), rate: 44100.0, tried: false }
    }

    fn init(&mut self) -> bool {
        if self.stream.is_some() {
            return true;
        }
        if self.tried {
            return false;
        }
        self.tried = true;
        let host = cpal::default_host();
        let Some(dev) = host.default_output_device() else { return false };
        let Ok(cfg) = dev.default_output_config() else { return false };
        self.rate = cfg.sample_rate().0 as f32;
        let channels = cfg.channels() as usize;
        let mixer = self.mixer.clone();
        let err = |_| {};
        let stream = match cfg.sample_format() {
            cpal::SampleFormat::F32 => dev.build_output_stream(
                &cfg.into(),
                move |out: &mut [f32], _| fill(out, channels, &mixer, |x| x),
                err,
                None,
            ),
            cpal::SampleFormat::I16 => dev.build_output_stream(
                &cfg.into(),
                move |out: &mut [i16], _| fill(out, channels, &mixer, |x| (x * i16::MAX as f32) as i16),
                err,
                None,
            ),
            cpal::SampleFormat::U16 => dev.build_output_stream(
                &cfg.into(),
                move |out: &mut [u16], _| fill(out, channels, &mixer, |x| ((x * 0.5 + 0.5) * u16::MAX as f32) as u16),
                err,
                None,
            ),
            _ => return false,
        };
        let Ok(stream) = stream else { return false };
        if stream.play().is_err() {
            return false;
        }
        self.stream = Some(stream);
        true
    }

    pub fn play(&mut self, name: &str, enabled: bool) {
        if !enabled || !self.init() {
            return;
        }
        let mut b = Buf { rate: self.rate, data: vec![] };
        use Wave::*;
        match name {
            "click" => b.tone(640.0, 0.05, Square, 0.04, 0.0, None),
            "place" => {
                b.tone(520.0, 0.08, Triangle, 0.1, 0.0, None);
                b.tone(780.0, 0.08, Triangle, 0.08, 0.05, None);
            }
            "shot" => {
                b.noise(0.18, 2600.0, 0.18, 0.0, Filter::High);
                b.tone(220.0, 0.15, Sine, 0.15, 0.0, Some(70.0));
            }
            "miss" => {
                b.noise(0.7, 900.0, 0.22, 0.0, Filter::Low);
                b.tone(300.0, 0.25, Sine, 0.05, 0.02, Some(120.0));
            }
            "hit" => {
                b.noise(0.55, 420.0, 0.5, 0.0, Filter::Low);
                b.tone(95.0, 0.45, Sine, 0.4, 0.0, Some(40.0));
            }
            "sunk" => {
                b.noise(0.5, 450.0, 0.5, 0.0, Filter::Low);
                b.tone(90.0, 0.5, Sine, 0.4, 0.0, Some(35.0));
                b.noise(1.1, 220.0, 0.45, 0.25, Filter::Low);
                b.tone(60.0, 0.9, Saw, 0.08, 0.2, Some(28.0));
            }
            "blocked" => {
                b.tone(880.0, 0.3, Triangle, 0.14, 0.0, None);
                b.tone(1320.0, 0.35, Triangle, 0.08, 0.04, None);
            }
            "radar" => {
                b.tone(900.0, 0.22, Sine, 0.1, 0.0, Some(1700.0));
                b.tone(900.0, 0.22, Sine, 0.08, 0.28, Some(1700.0));
            }
            "shield" => {
                b.tone(660.0, 0.2, Triangle, 0.12, 0.0, None);
                b.tone(990.0, 0.3, Triangle, 0.1, 0.1, None);
            }
            "turn" => {
                b.tone(740.0, 0.09, Sine, 0.1, 0.0, None);
                b.tone(988.0, 0.13, Sine, 0.1, 0.1, None);
            }
            "upgrade" => {
                for (i, f) in [523.0, 659.0, 784.0].iter().enumerate() {
                    b.tone(*f, 0.14, Triangle, 0.1, i as f32 * 0.08, None);
                }
            }
            "win" => {
                for (i, f) in [523.0, 659.0, 784.0, 1047.0].iter().enumerate() {
                    b.tone(*f, 0.3, Triangle, 0.14, i as f32 * 0.13, None);
                }
            }
            "lose" => {
                for (i, f) in [392.0, 330.0, 262.0, 196.0].iter().enumerate() {
                    b.tone(*f, 0.35, Triangle, 0.12, i as f32 * 0.16, None);
                }
            }
            "start" => {
                b.tone(440.0, 0.12, Square, 0.05, 0.0, None);
                b.tone(660.0, 0.18, Square, 0.05, 0.12, None);
            }
            _ => return,
        }
        if let Ok(mut m) = self.mixer.lock() {
            if m.len() < 16 {
                m.push(Voice { data: b.data, pos: 0 });
            }
        }
    }
}

fn fill<T: Copy>(out: &mut [T], channels: usize, mixer: &Mixer, conv: impl Fn(f32) -> T) {
    let mut m = match mixer.lock() {
        Ok(m) => m,
        Err(_) => return,
    };
    for frame in out.chunks_mut(channels) {
        let mut s = 0.0f32;
        for v in m.iter_mut() {
            if v.pos < v.data.len() {
                s += v.data[v.pos];
                v.pos += 1;
            }
        }
        let s = conv(s.clamp(-1.0, 1.0));
        for x in frame.iter_mut() {
            *x = s;
        }
    }
    m.retain(|v| v.pos < v.data.len());
}

/// Tampon où l'on additionne les sons d'un effet.
struct Buf {
    rate: f32,
    data: Vec<f32>,
}

impl Buf {
    fn ensure(&mut self, len: usize) {
        if self.data.len() < len {
            self.data.resize(len, 0.0);
        }
    }

    /// Volume qui décroît exponentiellement de `vol` à 0,0001 en `dur` secondes.
    fn env(vol: f32, t: f32, dur: f32) -> f32 {
        if t >= dur {
            0.0
        } else {
            vol * (0.0001f32 / vol).powf(t / dur)
        }
    }

    fn tone(&mut self, f: f32, dur: f32, w: Wave, vol: f32, when: f32, to: Option<f32>) {
        let start = (when * self.rate) as usize;
        let n = (dur * self.rate) as usize;
        self.ensure(start + n);
        let mut phase = 0.0f32;
        for i in 0..n {
            let t = i as f32 / self.rate;
            let freq = match to {
                Some(to) => f * (to / f).powf(t / dur),
                None => f,
            };
            phase = (phase + freq / self.rate).fract();
            let x = match w {
                Wave::Sine => (2.0 * PI * phase).sin(),
                Wave::Square => {
                    if phase < 0.5 {
                        1.0
                    } else {
                        -1.0
                    }
                }
                Wave::Triangle => 1.0 - 4.0 * (phase - 0.5).abs(),
                Wave::Saw => 2.0 * phase - 1.0,
            };
            self.data[start + i] += x * Self::env(vol, t, dur);
        }
    }

    fn noise(&mut self, dur: f32, freq: f32, vol: f32, when: f32, filter: Filter) {
        let start = (when * self.rate) as usize;
        let n = (dur * self.rate) as usize;
        self.ensure(start + n);
        // Filtre biquadratique (formules RBJ), Q = 1 comme le BiquadFilter de Web Audio.
        let w0 = 2.0 * PI * freq.min(self.rate * 0.45) / self.rate;
        let (sn, cs) = w0.sin_cos();
        let alpha = sn / 2.0;
        let (b0, b1, b2) = match filter {
            Filter::Low => ((1.0 - cs) / 2.0, 1.0 - cs, (1.0 - cs) / 2.0),
            Filter::High => ((1.0 + cs) / 2.0, -(1.0 + cs), (1.0 + cs) / 2.0),
        };
        let (a0, a1, a2) = (1.0 + alpha, -2.0 * cs, 1.0 - alpha);
        let (mut x1, mut x2, mut y1, mut y2) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
        let mut rng = rand::thread_rng();
        for i in 0..n {
            let x: f32 = rng.gen::<f32>() * 2.0 - 1.0;
            let y = (b0 * x + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2) / a0;
            x2 = x1;
            x1 = x;
            y2 = y1;
            y1 = y;
            let t = i as f32 / self.rate;
            self.data[start + i] += y * Self::env(vol, t, dur);
        }
    }
}
