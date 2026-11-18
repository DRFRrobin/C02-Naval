//! Apparence : couleurs, polices, fond, et petits composants dessinés à la main
//! pour retrouver le style de la version web.

use eframe::egui::{
    self, pos2, vec2, Align2, Color32, CursorIcon, FontData, FontDefinitions, FontFamily, FontId, Margin, Mesh,
    Pos2, Rect, Response, Rounding, Sense, Shadow, Stroke, Ui,
};

pub fn sk(w: f32, c: Color32) -> Stroke {
    Stroke::new(w, c)
}

pub const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}
pub fn rgba(r: u8, g: u8, b: u8, a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(r, g, b, (a * 255.0) as u8)
}

/* ---------- Palettes (thèmes) ---------- */

#[path = "palettes.rs"]
pub mod palettes;

/// Couleurs d'un thème : composantes rouge, vert, bleu (0–255) et opacité (0–1).
#[derive(Clone, Copy)]
pub struct Palette {
    pub card: [f32; 4],
    pub card2: [f32; 4],
    pub border: [f32; 4],
    pub border2: [f32; 4],
    pub text: [f32; 4],
    pub muted: [f32; 4],
    pub dim: [f32; 4],
    pub primary: [f32; 4],
    pub sky: [f32; 4],
    pub cyan: [f32; 4],
    pub green: [f32; 4],
    pub yellow: [f32; 4],
    pub purple: [f32; 4],
    pub red: [f32; 4],
    pub btn: [f32; 4],
    pub btn_hi: [f32; 4],
    pub water: [f32; 4],
    pub water_hi: [f32; 4],
    pub water_hov: [f32; 4],
    pub ship: [f32; 4],
    pub hit: [f32; 4],
    pub sunk: [f32; 4],
    pub miss: [f32; 4],
    pub ok: [f32; 4],
    pub input: [f32; 4],
    pub modal: [f32; 4],
    pub err: [f32; 4],
    pub scan: [f32; 4],
    pub scan_hi: [f32; 4],
    pub blk: [f32; 4],
    pub bg1: [f32; 4],
    pub bg2: [f32; 4],
    pub bg3: [f32; 4],
    pub glow: [f32; 4],
    pub badge: [f32; 4],
    pub ac1: [f32; 4],
    pub ac2: [f32; 4],
    pub light: bool,
    pub retro: bool,
}

/// Les mêmes couleurs, prêtes à dessiner.
#[derive(Clone, Copy)]
pub struct Colors {
    pub card: Color32,
    pub card2: Color32,
    pub border: Color32,
    pub border2: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub dim: Color32,
    pub primary: Color32,
    pub sky: Color32,
    pub cyan: Color32,
    pub green: Color32,
    pub yellow: Color32,
    pub purple: Color32,
    pub red: Color32,
    pub btn: Color32,
    pub btn_hi: Color32,
    pub water: Color32,
    pub water_hi: Color32,
    pub water_hov: Color32,
    pub ship: Color32,
    pub hit: Color32,
    pub sunk: Color32,
    pub miss: Color32,
    pub ok: Color32,
    pub input: Color32,
    pub modal: Color32,
    pub err: Color32,
    pub scan: Color32,
    pub scan_hi: Color32,
    pub blk: Color32,
    pub bg: [Color32; 3],
    pub glow: Color32,
    pub badge: Color32,
    pub ac1: Color32,
    pub ac2: Color32,
    pub light: bool,
    pub retro: bool,
}

fn col(c: [f32; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0] as u8, c[1] as u8, c[2] as u8, (c[3] * 255.0).round() as u8)
}

impl Colors {
    pub fn from(p: &Palette) -> Colors {
        Colors {
            card: col(p.card),
            card2: col(p.card2),
            border: col(p.border),
            border2: col(p.border2),
            text: col(p.text),
            muted: col(p.muted),
            dim: col(p.dim),
            primary: col(p.primary),
            sky: col(p.sky),
            cyan: col(p.cyan),
            green: col(p.green),
            yellow: col(p.yellow),
            purple: col(p.purple),
            red: col(p.red),
            btn: col(p.btn),
            btn_hi: col(p.btn_hi),
            water: col(p.water),
            water_hi: col(p.water_hi),
            water_hov: col(p.water_hov),
            ship: col(p.ship),
            hit: col(p.hit),
            sunk: col(p.sunk),
            miss: col(p.miss),
            ok: col(p.ok),
            input: col(p.input),
            modal: col(p.modal),
            err: col(p.err),
            scan: col(p.scan),
            scan_hi: col(p.scan_hi),
            blk: col(p.blk),
            bg: [col(p.bg1), col(p.bg2), col(p.bg3)],
            glow: col(p.glow),
            badge: col(p.badge),
            ac1: col(p.ac1),
            ac2: col(p.ac2),
            light: p.light,
            retro: p.retro,
        }
    }
}

thread_local! {
    static CUR: std::cell::Cell<Colors> = std::cell::Cell::new(Colors::from(&palettes::OCEAN));
}

/// Couleurs du thème actif.
pub fn pal() -> Colors {
    CUR.with(|c| c.get())
}

/// Thèmes, dans l'ordre de `palettes::ALL` et des noms de `i18n::theme_name` (après « auto »).
pub const THEME_IDS: [&str; 7] = ["ocean", "night", "forest", "sunset", "light", "arcade", "xmas"];

/// Thème par défaut sans choix du joueur : « Rétro Noël » du 1er au 26 décembre, « Océan » le reste de l'année.
pub fn default_theme() -> usize {
    use chrono::Datelike;
    let d = chrono::Local::now();
    if d.month() == 12 && d.day() <= 26 {
        6
    } else {
        0
    }
}

/// Active un thème (indice dans `THEME_IDS`) : couleurs et style egui.
pub fn apply_theme(ctx: &egui::Context, idx: usize) {
    let p = palettes::ALL[idx.min(palettes::ALL.len() - 1)];
    CUR.with(|c| c.set(Colors::from(&p)));
    setup_style(ctx);
}

/// Rayon d'angle : nul dans les thèmes rétro (coins carrés).
pub fn rd(x: f32) -> f32 {
    if pal().retro {
        0.0
    } else {
        x
    }
}

pub fn card() -> Color32 {
    pal().card
}
pub fn card2() -> Color32 {
    pal().card2
}
pub fn border() -> Color32 {
    pal().border
}
pub fn border2() -> Color32 {
    pal().border2
}

/* ---------- Polices ---------- */

pub fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let dir = std::path::Path::new(&std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into())).join("Fonts");
    let mut load = |key: &str, file: &str| -> bool {
        match std::fs::read(dir.join(file)) {
            Ok(b) => {
                fonts.font_data.insert(key.into(), FontData::from_owned(b));
                true
            }
            Err(_) => false,
        }
    };
    let reg = load("segoe", "segoeui.ttf");
    let bold = load("segoe-bold", "segoeuib.ttf");
    let semi = load("segoe-semi", "seguisb.ttf");
    let num = load("num", "bahnschrift.ttf") || load("num", "consolab.ttf");
    let sym = load("segoe-sym", "seguisym.ttf");
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

    let base: Vec<String> = fonts.families[&FontFamily::Proportional].clone();
    let fam = |main: Option<&str>| -> Vec<String> {
        let mut v = vec![];
        if let Some(m) = main {
            v.push(m.to_string());
        }
        if reg {
            v.push("segoe".into());
        }
        v.push("phosphor".into());
        v.extend(base.iter().filter(|k| k.as_str() != "phosphor").cloned());
        if sym {
            v.push("segoe-sym".into());
        }
        v
    };
    let p = fam(None);
    let b = fam(bold.then_some("segoe-bold"));
    let s = fam(semi.then_some("segoe-semi"));
    let n = fam(num.then_some("num"));
    fonts.families.insert(FontFamily::Proportional, p);
    fonts.families.insert(FontFamily::Name("bold".into()), b);
    fonts.families.insert(FontFamily::Name("semi".into()), s);
    fonts.families.insert(FontFamily::Name("num".into()), n);
    ctx.set_fonts(fonts);
}

pub fn reg(size: f32) -> FontId {
    FontId::proportional(size)
}
pub fn bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("bold".into()))
}
pub fn semi(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("semi".into()))
}
pub fn num(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("num".into()))
}

pub fn setup_style(ctx: &egui::Context) {
    ctx.style_mut(|s| {
        s.visuals = if pal().light { egui::Visuals::light() } else { egui::Visuals::dark() };
        s.visuals.override_text_color = Some(pal().text);
        s.visuals.panel_fill = Color32::TRANSPARENT;
        s.visuals.window_fill = pal().modal;
        s.visuals.extreme_bg_color = pal().input;
        s.visuals.selection.bg_fill = pal().primary;
        s.visuals.selection.stroke = sk(1.0, pal().sky);
        s.visuals.text_cursor.stroke = sk(2.0, pal().sky);
        s.visuals.widgets.inactive.bg_stroke = sk(1.0, border2());
        s.visuals.widgets.inactive.rounding = Rounding::same(rd(10.0));
        s.visuals.widgets.hovered.bg_stroke = sk(1.0, pal().sky);
        s.visuals.widgets.hovered.rounding = Rounding::same(rd(10.0));
        s.visuals.widgets.active.bg_stroke = sk(1.0, pal().sky);
        s.visuals.widgets.active.rounding = Rounding::same(rd(10.0));
        s.visuals.widgets.inactive.weak_bg_fill = pal().input;
        s.visuals.widgets.hovered.weak_bg_fill = pal().input;
        s.visuals.widgets.active.weak_bg_fill = pal().input;
        s.visuals.widgets.open.weak_bg_fill = pal().input;
        s.visuals.widgets.inactive.bg_fill = pal().input;
        s.visuals.widgets.noninteractive.bg_stroke = sk(1.0, border());
        s.visuals.popup_shadow = Shadow::NONE;
        s.interaction.selectable_labels = false; // les textes ne doivent pas « manger » les clics sur les cartes
        s.spacing.item_spacing = vec2(8.0, 8.0);
        s.spacing.button_padding = vec2(14.0, 10.0);
        s.spacing.interact_size.y = 40.0;
        s.text_styles.insert(egui::TextStyle::Body, reg(15.0));
        s.text_styles.insert(egui::TextStyle::Button, semi(15.0));
        s.text_styles.insert(egui::TextStyle::Small, reg(12.0));
        s.text_styles.insert(egui::TextStyle::Heading, bold(24.0));
    });
}

/// Fond : dégradé vertical et halo bleu en haut à gauche, comme le site.
pub fn paint_background(ctx: &egui::Context) {
    let r = ctx.screen_rect();
    let p = ctx.layer_painter(egui::LayerId::background());
    let mut m = Mesh::default();
    let bg = pal().bg;
    let stops = [(0.0, bg[0]), (0.55, bg[1]), (1.0, bg[2])];
    for w in stops.windows(2) {
        let (t0, c0) = w[0];
        let (t1, c1) = w[1];
        let y0 = r.top() + r.height() * t0;
        let y1 = r.top() + r.height() * t1;
        let i = m.vertices.len() as u32;
        m.colored_vertex(pos2(r.left(), y0), c0);
        m.colored_vertex(pos2(r.right(), y0), c0);
        m.colored_vertex(pos2(r.right(), y1), c1);
        m.colored_vertex(pos2(r.left(), y1), c1);
        m.add_triangle(i, i + 1, i + 2);
        m.add_triangle(i, i + 2, i + 3);
    }
    // Halo radial (ellipse 1100 × 650 centrée à 28 % / -8 %).
    let c = pos2(r.left() + r.width() * 0.28, r.top() - r.height() * 0.08);
    let (rx, ry) = (1100.0, 650.0);
    let center = m.vertices.len() as u32;
    let glow = pal().glow;
    let fade = |a: f32| Color32::from_rgba_unmultiplied(glow.r(), glow.g(), glow.b(), (glow.a() as f32 * a) as u8);
    m.colored_vertex(c, glow);
    let rings = [(0.31, 0.6), (0.62, 0.0)];
    let seg = 48;
    let mut prev_ring_start = None;
    for (ri, (f, a)) in rings.iter().enumerate() {
        let start = m.vertices.len() as u32;
        for k in 0..seg {
            let ang = k as f32 / seg as f32 * std::f32::consts::TAU;
            m.colored_vertex(pos2(c.x + ang.cos() * rx * f, c.y + ang.sin() * ry * f), fade(*a));
        }
        for k in 0..seg {
            let a0 = start + k;
            let a1 = start + (k + 1) % seg;
            if ri == 0 {
                m.add_triangle(center, a0, a1);
            } else if let Some(ps) = prev_ring_start {
                let b0: u32 = ps + k;
                let b1 = ps + (k + 1) % seg;
                m.add_triangle(b0, a0, a1);
                m.add_triangle(b0, a1, b1);
            }
        }
        prev_ring_start = Some(start);
    }
    p.add(egui::Shape::mesh(m));
}

/* ---------- Composants ---------- */

pub fn card_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(card())
        .stroke(sk(1.0, border()))
        .rounding(Rounding::same(rd(16.0)))
        .inner_margin(Margin::same(16.0))
}

pub fn panel_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(card())
        .stroke(if pal().retro { sk(3.0, pal().ac1) } else { sk(1.0, border2()) })
        .rounding(Rounding::same(rd(16.0)))
        .inner_margin(Margin::symmetric(32.0, 28.0))
        .shadow(if pal().retro {
            Shadow { offset: vec2(6.0, 6.0), blur: 0.0, spread: 0.0, color: pal().ac2 }
        } else {
            Shadow { offset: vec2(0.0, 10.0), blur: 30.0, spread: 0.0, color: rgba(0, 0, 0, 0.35) }
        })
}

pub fn sub_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(card2())
        .stroke(sk(1.0, border()))
        .rounding(Rounding::same(rd(12.0)))
        .inner_margin(Margin::symmetric(18.0, 16.0))
}

/// Contenu centré, de largeur maximale `w`.
pub fn centered<R>(ui: &mut Ui, w: f32, f: impl FnOnce(&mut Ui) -> R) -> R {
    let avail = ui.available_width();
    let w = w.min(avail);
    let pad = ((avail - w) / 2.0).max(0.0);
    let rect = Rect::from_min_size(ui.cursor().min + vec2(pad, 0.0), vec2(w, f32::INFINITY));
    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.set_width(w);
        f(ui)
    })
    .inner
}

#[derive(Clone, Copy, PartialEq)]
pub enum Bs {
    Normal,
    Primary,
    Grad,
    Danger,
    Ok,
    Ghost,
    On,
}

pub struct Btn<'a> {
    pub text: &'a str,
    pub icon: Option<&'a str>,
    pub style: Bs,
    pub wide: bool,
    pub small: bool,
    pub enabled: bool,
    pub width: Option<f32>,
}

impl<'a> Btn<'a> {
    pub fn new(text: &'a str) -> Self {
        Btn { text, icon: None, style: Bs::Normal, wide: false, small: false, enabled: true, width: None }
    }
    pub fn icon(mut self, i: &'a str) -> Self {
        self.icon = Some(i);
        self
    }
    pub fn style(mut self, s: Bs) -> Self {
        self.style = s;
        self
    }
    pub fn wide(mut self) -> Self {
        self.wide = true;
        self
    }
    pub fn small(mut self) -> Self {
        self.small = true;
        self
    }
    pub fn enabled(mut self, e: bool) -> Self {
        self.enabled = e;
        self
    }
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let fs = if self.small { 13.0 } else { 15.0 };
        let h = if self.small { 36.0 } else { 44.0 };
        let padx = if self.small { 12.0 } else { 16.0 };
        let text_col = match self.style {
            Bs::Primary | Bs::Grad | Bs::Ok | Bs::On => Color32::WHITE,
            Bs::Danger => pal().err,
            Bs::Ghost => pal().muted,
            Bs::Normal => pal().text,
        };
        let label = match self.icon {
            Some(i) if !self.text.is_empty() => format!("{i}  {}", self.text),
            Some(i) => i.to_string(),
            None => self.text.to_string(),
        };
        let galley = ui.painter().layout_no_wrap(label, semi(fs), text_col);
        let w = if self.wide {
            ui.available_width()
        } else {
            self.width.unwrap_or(galley.size().x + padx * 2.0)
        };
        let sense = if self.enabled { Sense::click() } else { Sense::hover() };
        let (rect, resp) = ui.allocate_exact_size(vec2(w, h), sense);
        if ui.is_rect_visible(rect) {
            let hov = self.enabled && resp.hovered();
            let (fill, stroke) = match self.style {
                Bs::Normal => (if hov { pal().btn_hi } else { pal().btn }, Stroke::NONE),
                Bs::Primary => (if hov { rgb(0x0b, 0x95, 0xea) } else { pal().primary }, Stroke::NONE),
                Bs::Grad => (if hov { rgb(0x18, 0x7c, 0xec) } else { rgb(0x10, 0x67, 0xd2) }, Stroke::NONE),
                Bs::Danger => (if hov { rgb(0x4c, 0x11, 0x19) } else { rgb(0x3a, 0x0d, 0x14) }, sk(1.0, rgb(0x7f, 0x1d, 0x2a))),
                Bs::Ok => (if hov { rgb(0x07, 0xa8, 0x77) } else { rgb(0x05, 0x96, 0x69) }, Stroke::NONE),
                Bs::Ghost => (if hov { rgba(255, 255, 255, 0.05) } else { Color32::TRANSPARENT }, Stroke::NONE),
                Bs::On => (pal().primary, sk(2.0, rgba(255, 255, 255, 0.85))),
            };
            let a = if self.enabled { 1.0 } else { 0.4 };
            let p = ui.painter();
            if self.style == Bs::Grad && self.enabled {
                p.rect_filled(rect.translate(vec2(0.0, 6.0)).expand(2.0), 12.0, rgba(14, 165, 233, 0.12));
            }
            let stroke = if pal().retro && self.style != Bs::Ghost {
                // style « borne d'arcade » : bordure épaisse néon et ombre franche décalée
                p.rect_filled(rect.translate(vec2(3.0, 3.0)), 0.0, Color32::BLACK.gamma_multiply(a));
                sk(2.5, if matches!(self.style, Bs::Primary | Bs::Grad | Bs::On) { pal().yellow } else { pal().ac1 })
            } else {
                stroke
            };
            p.rect(rect, rd(10.0), fill.gamma_multiply(a), sk(stroke.width, stroke.color.gamma_multiply(a)));
            let gpos = rect.center() - galley.size() / 2.0;
            p.galley(gpos, galley, text_col.gamma_multiply(a));
            if hov {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
        }
        resp
    }
}

pub fn label(ui: &mut Ui, text: impl Into<String>, font: FontId, color: Color32) -> Response {
    ui.label(egui::RichText::new(text.into()).font(font).color(color))
}

pub fn wrapped(ui: &mut Ui, text: impl Into<String>, font: FontId, color: Color32) -> Response {
    ui.add(egui::Label::new(egui::RichText::new(text.into()).font(font).color(color)).wrap())
}

/// Message centré (info, erreur ou succès).
pub fn msg(ui: &mut Ui, text: &str, kind: MsgKind, size: f32) {
    let c = match kind {
        MsgKind::Info => pal().muted,
        MsgKind::Err => pal().err,
        MsgKind::Ok => pal().green,
    };
    ui.add_space(4.0);
    ui.vertical_centered(|ui| {
        ui.add(egui::Label::new(egui::RichText::new(if text.is_empty() { " " } else { text }).font(reg(size)).color(c)).wrap());
    });
    ui.add_space(4.0);
}

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum MsgKind {
    #[default]
    Info,
    Err,
    Ok,
}

/// Icône carrée colorée (cartes des modes de jeu).
pub fn mode_icon(ui: &mut Ui, icon: &str, color: Color32, bg: Color32, size: f32) -> Rect {
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    let p = ui.painter();
    p.rect(rect, rd(10.0), bg, sk(1.0, color.gamma_multiply(0.55)));
    p.text(rect.center(), Align2::CENTER_CENTER, icon, reg(size * 0.42), color);
    rect
}

pub fn icon_colors(kind: &str) -> (Color32, Color32) {
    match kind {
        "cyan" => (pal().cyan, rgba(8, 145, 178, 0.18)),
        "green" => (rgb(0x6e, 0xe7, 0xb7), rgba(5, 150, 105, 0.2)),
        "yellow" => (rgb(0xfc, 0xd3, 0x4d), rgba(180, 120, 10, 0.22)),
        _ => (rgb(0xc4, 0xb5, 0xfd), rgba(124, 58, 237, 0.22)),
    }
}

/// Barre d'XP.
pub fn xp_bar(ui: &mut Ui, frac: f32, w: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(w, 9.0), Sense::hover());
    let p = ui.painter();
    p.rect(rect, rd(6.0), rgb(0x06, 0x0d, 0x18), sk(1.0, rgba(100, 140, 180, 0.35)));
    let f = frac.clamp(0.0, 1.0);
    if f > 0.0 {
        let fill = Rect::from_min_size(rect.min, vec2(rect.width() * f, rect.height()));
        let mut mesh = Mesh::default();
        let (c0, c1) = (rgb(0x0e, 0xa5, 0xe9), rgb(0x67, 0xe8, 0xf9));
        mesh.colored_vertex(fill.left_top(), c0);
        mesh.colored_vertex(fill.right_top(), c1);
        mesh.colored_vertex(fill.right_bottom(), c1);
        mesh.colored_vertex(fill.left_bottom(), c0);
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
        p.add(egui::Shape::mesh(mesh));
    }
}

/// Bordure en pointillés autour d'un rectangle.
pub fn dashed_rect(ui: &Ui, r: Rect, stroke: Stroke) {
    let pts = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
    let shapes = egui::Shape::dashed_line(&pts, stroke, 6.0, 4.0);
    ui.painter().extend(shapes);
}

/// Image ronde-carrée (avatar).
pub fn paint_image(ui: &Ui, tex: &egui::TextureHandle, rect: Rect, rounding: f32) {
    let img = egui::Image::new(egui::load::SizedTexture::new(tex.id(), rect.size())).rounding(rounding);
    img.paint_at(ui, rect);
}

pub fn modal_backdrop(ctx: &egui::Context, id: &str) {
    let screen = ctx.screen_rect();
    egui::Area::new(egui::Id::new(id).with("bg"))
        .order(egui::Order::Foreground)
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            ui.painter().rect_filled(screen, rd(0.0), rgba(2, 6, 14, 0.78));
            ui.allocate_rect(screen, Sense::click_and_drag());
        });
}

pub fn modal<R>(ctx: &egui::Context, id: &str, width: f32, center: bool, add: impl FnOnce(&mut Ui) -> R) -> R {
    modal_backdrop(ctx, id);
    let w = width.min(ctx.screen_rect().width() - 32.0);
    egui::Area::new(egui::Id::new(id))
        .order(egui::Order::Tooltip)
        .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .show(ctx, |ui| {
            egui::Frame::none()
                .fill(pal().modal)
                .stroke(sk(1.0, border2()))
                .rounding(rd(16.0))
                .inner_margin(Margin::symmetric(20.0, 24.0))
                .shadow(Shadow { offset: vec2(0.0, 24.0), blur: 60.0, spread: 0.0, color: rgba(0, 0, 0, 0.55) })
                .show(ui, |ui| {
                    ui.set_width(w - 40.0);
                    if center {
                        ui.vertical_centered(add).inner
                    } else {
                        add(ui)
                    }
                })
                .inner
        })
        .inner
}

pub fn spinner(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(vec2(46.0, 46.0), Sense::hover());
    let t = ui.input(|i| i.time) as f32;
    let c = rect.center();
    let r = 21.0;
    ui.painter().circle_stroke(c, r, sk(4.0, rgba(56, 189, 248, 0.18)));
    let a0 = t * std::f32::consts::TAU;
    let pts: Vec<Pos2> = (0..=20).map(|i| {
        let a = a0 + i as f32 / 20.0 * std::f32::consts::FRAC_PI_2;
        pos2(c.x + a.cos() * r, c.y + a.sin() * r)
    }).collect();
    ui.painter().add(egui::Shape::line(pts, sk(4.0, pal().sky)));
    ui.ctx().request_repaint();
}

pub fn text_input(ui: &mut Ui, s: &mut String, password: bool, max: usize, hint: &str) -> Response {
    let te = egui::TextEdit::singleline(s)
        .password(password)
        .char_limit(max)
        .font(reg(16.0))
        .margin(vec2(14.0, 11.0))
        .hint_text(hint)
        .desired_width(f32::INFINITY);
    ui.add(te)
}

pub fn field_label(ui: &mut Ui, t: &str) {
    ui.add_space(6.0);
    label(ui, t, reg(13.0), pal().muted);
}
