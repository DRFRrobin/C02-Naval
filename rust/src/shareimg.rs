//! Image de partage du résultat : 1080 × 1350 (format portrait des réseaux sociaux), dessinée sans navigateur,
//! avec la même mise en page que la version web : gagnant, deux grilles côte à côte, légende et chiffres du match.

use ab_glyph::{point, Font, FontArc, PxScale, ScaleFont};
use eframe::egui::Color32;
use image::{Rgba, RgbaImage};

pub const W: u32 = 1080;
pub const H: u32 = 1350;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Empty,
    Ship,
    Hit,
    Sunk,
    Blocked,
    Scan,
    Miss,
    Missed, // bateau adverse jamais trouvé
}

pub struct Palette {
    pub bg: [Color32; 3],
    pub card: Color32,
    pub border: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub water: Color32,
    pub ship: Color32,
    pub hit: Color32,
    pub sunk: Color32,
    pub miss: Color32,
    pub blk: Color32,
    pub scan: Color32,
    pub yellow: Color32,
    pub sky: Color32,
}

pub struct Data {
    pub size: usize,
    pub win: bool,
    pub my_name: String,
    pub opp_name: String,
    pub forfeit: bool,
    pub mine: Vec<Kind>,
    pub theirs: Vec<Kind>,
    pub stats: Vec<(String, String)>, // (libellé, valeur)
    // textes (déjà traduits)
    pub t_winner: String,
    pub t_vs: String,
    pub t_forfeit: String,
    pub t_fleet_me: String,
    pub t_fleet_opp: String,
    pub t_legend: Vec<(Kind, String)>,
    pub t_stats_of: String,
    pub t_footer: String,
}

pub struct Fonts(pub Vec<FontArc>);

impl Fonts {
    /// Polices Windows si présentes (Segoe UI), sinon polices intégrées à egui.
    pub fn load() -> Fonts {
        let mut v = vec![];
        let dir = std::path::Path::new(&std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into())).join("Fonts");
        for f in ["segoeuib.ttf", "seguisym.ttf"] {
            if let Some(a) = std::fs::read(dir.join(f)).ok().and_then(|b| FontArc::try_from_vec(b).ok()) {
                v.push(a);
            }
        }
        let defs = eframe::egui::FontDefinitions::default();
        for k in ["Ubuntu-Light", "NotoEmoji-Regular", "emoji-icon-font"] {
            if let Some(a) = defs.font_data.get(k).and_then(|d| FontArc::try_from_vec(d.font.to_vec()).ok()) {
                v.push(a);
            }
        }
        Fonts(v)
    }

    fn pick(&self, c: char) -> Option<&FontArc> {
        self.0.iter().find(|f| f.glyph_id(c).0 != 0).or_else(|| self.0.first())
    }

    pub fn width(&self, text: &str, px: f32) -> f32 {
        text.chars()
            .filter_map(|c| self.pick(c).map(|f| f.as_scaled(PxScale::from(px)).h_advance(f.glyph_id(c))))
            .sum()
    }
}

type Img = RgbaImage;

fn blend(img: &mut Img, x: i32, y: i32, c: Color32, cover: f32) {
    if x < 0 || y < 0 || x >= img.width() as i32 || y >= img.height() as i32 {
        return;
    }
    let a = (c.a() as f32 / 255.0) * cover.clamp(0.0, 1.0);
    if a <= 0.0 {
        return;
    }
    let p = img.get_pixel_mut(x as u32, y as u32);
    for i in 0..3 {
        let src = [c.r(), c.g(), c.b()][i] as f32;
        p.0[i] = (src * a + p.0[i] as f32 * (1.0 - a)).round() as u8;
    }
    p.0[3] = 255;
}

/// Distance signée à un rectangle arrondi (négative à l'intérieur).
fn sd_rrect(px: f32, py: f32, x: f32, y: f32, w: f32, h: f32, r: f32) -> f32 {
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    let (qx, qy) = ((px - cx).abs() - (w / 2.0 - r), (py - cy).abs() - (h / 2.0 - r));
    qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - r
}

fn fill_rrect(img: &mut Img, x: f32, y: f32, w: f32, h: f32, r: f32, c: Color32) {
    for py in (y.floor() as i32 - 1)..(y + h).ceil() as i32 + 1 {
        for px in (x.floor() as i32 - 1)..(x + w).ceil() as i32 + 1 {
            let d = sd_rrect(px as f32 + 0.5, py as f32 + 0.5, x, y, w, h, r);
            blend(img, px, py, c, 0.5 - d);
        }
    }
}

fn stroke_rrect(img: &mut Img, x: f32, y: f32, w: f32, h: f32, r: f32, lw: f32, c: Color32) {
    for py in (y.floor() as i32 - 2)..(y + h).ceil() as i32 + 2 {
        for px in (x.floor() as i32 - 2)..(x + w).ceil() as i32 + 2 {
            let d = sd_rrect(px as f32 + 0.5, py as f32 + 0.5, x, y, w, h, r).abs() - lw / 2.0;
            blend(img, px, py, c, 0.5 - d);
        }
    }
}

fn line(img: &mut Img, a: (f32, f32), b: (f32, f32), lw: f32, c: Color32) {
    let (minx, maxx) = (a.0.min(b.0) - lw, a.0.max(b.0) + lw);
    let (miny, maxy) = (a.1.min(b.1) - lw, a.1.max(b.1) + lw);
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    for py in miny.floor() as i32..maxy.ceil() as i32 {
        for px in minx.floor() as i32..maxx.ceil() as i32 {
            let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
            let t = (((fx - a.0) * dx + (fy - a.1) * dy) / len2).clamp(0.0, 1.0);
            let d = (fx - (a.0 + t * dx)).hypot(fy - (a.1 + t * dy)) - lw / 2.0;
            blend(img, px, py, c, 0.5 - d);
        }
    }
}

fn disc(img: &mut Img, cx: f32, cy: f32, r: f32, c: Color32) {
    for py in (cy - r - 1.0).floor() as i32..(cy + r + 1.0).ceil() as i32 {
        for px in (cx - r - 1.0).floor() as i32..(cx + r + 1.0).ceil() as i32 {
            blend(img, px, py, c, 0.5 - ((px as f32 + 0.5 - cx).hypot(py as f32 + 0.5 - cy) - r));
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Al {
    Left,
    Center,
}

/// Texte sur la ligne de base `y`.
fn text(img: &mut Img, fonts: &Fonts, s: &str, x: f32, y: f32, px: f32, c: Color32, al: Al) {
    let w = fonts.width(s, px);
    let mut cx = if al == Al::Center { x - w / 2.0 } else { x };
    for ch in s.chars() {
        let Some(f) = fonts.pick(ch) else { continue };
        let sc = f.as_scaled(PxScale::from(px));
        let g = f.glyph_id(ch).with_scale_and_position(PxScale::from(px), point(cx, y));
        if let Some(o) = f.outline_glyph(g) {
            let b = o.px_bounds();
            o.draw(|gx, gy, cov| blend(img, b.min.x as i32 + gx as i32, b.min.y as i32 + gy as i32, c, cov));
        }
        cx += sc.h_advance(f.glyph_id(ch));
    }
}

/// Texte réduit si besoin pour tenir dans la largeur donnée.
fn fit(img: &mut Img, fonts: &Fonts, s: &str, x: f32, y: f32, max_w: f32, size: f32, c: Color32) {
    let mut px = size;
    while fonts.width(s, px) > max_w && px > 12.0 {
        px -= 2.0;
    }
    text(img, fonts, s, x, y, px, c, Al::Center);
}

fn draw_cell(img: &mut Img, p: &Palette, x: f32, y: f32, w: f32, kind: Kind) {
    let fill = match kind {
        Kind::Ship | Kind::Missed => p.ship,
        Kind::Hit => p.hit,
        Kind::Sunk => p.sunk,
        Kind::Blocked => p.blk,
        Kind::Scan => p.scan,
        _ => p.water,
    };
    let fill = if kind == Kind::Missed { Color32::from_rgba_unmultiplied(fill.r(), fill.g(), fill.b(), 153) } else { fill };
    let r = (w * 0.12).max(2.0);
    fill_rrect(img, x, y, w, w, r, fill);
    if kind == Kind::Missed {
        stroke_rrect(img, x + 1.5, y + 1.5, w - 3.0, w - 3.0, r, (w * 0.08).max(2.0), p.yellow);
    }
    let pad = w * 0.3;
    let lw = (w * 0.11).max(2.0);
    let mark = |img: &mut Img, c: Color32| {
        line(img, (x + pad, y + pad), (x + w - pad, y + w - pad), lw, c);
        line(img, (x + w - pad, y + pad), (x + pad, y + w - pad), lw, c);
    };
    match kind {
        Kind::Hit => mark(img, Color32::WHITE),
        Kind::Sunk => mark(img, Color32::from_rgb(0xfe, 0xca, 0xca)),
        Kind::Miss => disc(img, x + w / 2.0, y + w / 2.0, (w * 0.11).max(2.0), p.miss),
        _ => {}
    }
}

fn draw_grid(img: &mut Img, fonts: &Fonts, p: &Palette, x: f32, y: f32, size: f32, n: usize, kinds: &[Kind]) {
    let lab = 26.0;
    let cell = (size - lab) / n as f32;
    let gap = (cell * 0.08).max(1.5);
    let w = cell - gap;
    let px = (cell * 0.42).min(18.0);
    for i in 0..n {
        text(img, fonts, &(i + 1).to_string(), x + lab + i as f32 * cell + w / 2.0, y - 14.0 + px * 0.35, px, p.muted, Al::Center);
        text(img, fonts, &((b'A' + i as u8) as char).to_string(), x + lab / 2.0 - 2.0, y + i as f32 * cell + w / 2.0 + px * 0.35, px, p.muted, Al::Center);
    }
    for r in 0..n {
        for c in 0..n {
            draw_cell(img, p, x + lab + c as f32 * cell, y + r as f32 * cell, w, kinds[r * n + c]);
        }
    }
}

pub fn render(d: &Data, p: &Palette, fonts: &Fonts) -> RgbaImage {
    let mut img = RgbaImage::from_pixel(W, H, Rgba([0, 0, 0, 255]));
    // fond : dégradé vertical
    let stops = [(0.0, p.bg[0]), (0.55, p.bg[1]), (1.0, p.bg[2])];
    for y in 0..H {
        let t = y as f32 / H as f32;
        let (a, b) = if t < 0.55 { (stops[0], stops[1]) } else { (stops[1], stops[2]) };
        let u = if t < 0.55 { t / 0.55 } else { (t - 0.55) / 0.45 };
        let mix = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * u) as u8;
        let c = Rgba([mix(a.1.r(), b.1.r()), mix(a.1.g(), b.1.g()), mix(a.1.b(), b.1.b()), 255]);
        for x in 0..W {
            img.put_pixel(x, y, c);
        }
    }
    let wf = W as f32;
    // en-tête
    text(&mut img, fonts, "⚓ C02 NAVAL", wf / 2.0, 92.0, 34.0, p.sky, Al::Center);
    text(&mut img, fonts, if d.win { "🏆" } else { "🌊" }, wf / 2.0, 210.0, 96.0, p.sky, Al::Center);
    fit(&mut img, fonts, &d.t_winner, wf / 2.0, 296.0, wf - 120.0, 64.0, p.yellow);
    let mut vs = format!("{}  {}  {}", d.my_name, d.t_vs, d.opp_name);
    if d.forfeit {
        vs += &format!(" · {}", d.t_forfeit);
    }
    fit(&mut img, fonts, &vs, wf / 2.0, 350.0, wf - 120.0, 30.0, p.muted);
    // grilles
    let (pad, gap_x, top) = (50.0, 40.0, 470.0);
    let col_w = (wf - pad * 2.0 - gap_x) / 2.0;
    let cols = [(&d.t_fleet_me, &d.mine, d.win), (&d.t_fleet_opp, &d.theirs, !d.win)];
    for (i, (title, kinds, won)) in cols.into_iter().enumerate() {
        let x = pad + i as f32 * (col_w + gap_x);
        let card = Color32::from_rgba_unmultiplied(p.card.r(), p.card.g(), p.card.b(), 217);
        fill_rrect(&mut img, x - 14.0, top - 96.0, col_w + 28.0, col_w + 116.0, 22.0, card);
        stroke_rrect(&mut img, x - 14.0, top - 96.0, col_w + 28.0, col_w + 116.0, 22.0, if won { 4.0 } else { 2.0 }, if won { p.yellow } else { p.border });
        let t = format!("{}{}", if won { "👑 " } else { "" }, title);
        fit(&mut img, fonts, &t, x + col_w / 2.0, top - 52.0, col_w - 10.0, 30.0, if won { p.yellow } else { p.text });
        draw_grid(&mut img, fonts, p, x, top, col_w, d.size, kinds);
    }
    // légende
    let ly = top + col_w + 66.0;
    let widths: Vec<f32> = d.t_legend.iter().map(|(_, l)| 40.0 + fonts.width(l, 24.0)).collect();
    let total: f32 = widths.iter().sum::<f32>() + 28.0 * (widths.len() as f32 - 1.0);
    let mut lx = (wf - total) / 2.0;
    for ((k, label), w) in d.t_legend.iter().zip(&widths) {
        draw_cell(&mut img, p, lx, ly - 15.0, 30.0, *k);
        text(&mut img, fonts, label, lx + 40.0, ly + 8.0, 24.0, p.muted, Al::Left);
        lx += w + 28.0;
    }
    // chiffres du match
    let sy = ly + 60.0;
    fit(&mut img, fonts, &d.t_stats_of, wf / 2.0, sy + 10.0, wf - 120.0, 24.0, p.muted);
    let bw = (wf - pad * 2.0 - 3.0 * 18.0) / 4.0;
    for (i, (label, val)) in d.stats.iter().enumerate() {
        let x = pad + i as f32 * (bw + 18.0);
        let y = sy + 30.0;
        let card = Color32::from_rgba_unmultiplied(p.card.r(), p.card.g(), p.card.b(), 217);
        fill_rrect(&mut img, x, y, bw, 120.0, 18.0, card);
        stroke_rrect(&mut img, x, y, bw, 120.0, 18.0, 2.0, p.border);
        fit(&mut img, fonts, val, x + bw / 2.0, y + 62.0, bw - 20.0, 44.0, p.text);
        fit(&mut img, fonts, label, x + bw / 2.0, y + 100.0, bw - 20.0, 22.0, p.muted);
    }
    // pied
    fit(&mut img, fonts, &d.t_footer, wf / 2.0, H as f32 - 40.0, wf - 100.0, 28.0, p.sky);
    img
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> (Data, Palette) {
        let c = |r, g, b| Color32::from_rgb(r, g, b);
        let p = Palette {
            bg: [c(8, 26, 49), c(5, 13, 27), c(3, 8, 16)],
            card: c(12, 26, 46),
            border: c(40, 80, 130),
            text: c(232, 241, 251),
            muted: c(142, 163, 187),
            water: c(15, 43, 73),
            ship: c(123, 144, 168),
            hit: c(239, 68, 68),
            sunk: c(127, 29, 29),
            miss: c(207, 224, 240),
            blk: c(90, 74, 28),
            scan: c(44, 79, 120),
            yellow: c(251, 191, 36),
            sky: c(56, 189, 248),
        };
        let n = 10;
        let mut mine = vec![Kind::Empty; n * n];
        let mut theirs = vec![Kind::Empty; n * n];
        for i in 0..5 {
            mine[i] = Kind::Ship;
            mine[10 + i] = Kind::Sunk;
            theirs[20 + i] = Kind::Hit;
            theirs[33] = Kind::Miss;
            theirs[50 + i] = Kind::Missed;
        }
        let d = Data {
            size: n,
            win: true,
            my_name: "Robin".into(),
            opp_name: "WebJoueur".into(),
            forfeit: false,
            mine,
            theirs,
            stats: vec![("Tirs".into(), "42".into()), ("Précision".into(), "38 %".into()), ("Coulés".into(), "5 / 5".into()), ("Durée".into(), "6m 12s".into())],
            t_winner: "Gagnant : Robin".into(),
            t_vs: "contre".into(),
            t_forfeit: "abandon".into(),
            t_fleet_me: "Flotte de Robin".into(),
            t_fleet_opp: "Flotte de WebJoueur".into(),
            t_legend: vec![(Kind::Ship, "Bateau".into()), (Kind::Hit, "Touché".into()), (Kind::Sunk, "Coulé".into()), (Kind::Miss, "Raté".into()), (Kind::Missed, "Non trouvé".into())],
            t_stats_of: "Chiffres de Robin".into(),
            t_footer: "C02 Naval".into(),
        };
        (d, p)
    }

    #[test]
    fn renders_a_full_size_image() {
        let (d, p) = sample();
        let fonts = Fonts::load();
        assert!(!fonts.0.is_empty());
        let img = render(&d, &p, &fonts);
        assert_eq!((img.width(), img.height()), (W, H));
        // le haut est le début du dégradé, le bas sa fin
        assert!(img.get_pixel(2, 2).0[2] > img.get_pixel(2, H - 2).0[2]);
        // il y a bien du texte et des marques dessinés (l'image n'est pas unie)
        let distinct: std::collections::HashSet<[u8; 4]> = img.pixels().map(|p| p.0).collect();
        assert!(distinct.len() > 200, "{}", distinct.len());
        if let Ok(path) = std::env::var("C02_SHARE_TEST_OUT") {
            img.save(path).unwrap();
        }
    }
}
