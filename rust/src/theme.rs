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

pub fn card() -> Color32 {
    rgba(12, 24, 42, 0.88)
}
pub fn card2() -> Color32 {
    rgba(6, 13, 25, 0.72)
}
pub fn border() -> Color32 {
    rgba(64, 140, 210, 0.22)
}
pub fn border2() -> Color32 {
    rgba(64, 140, 210, 0.36)
}
pub const TEXT: Color32 = rgb(0xe8, 0xf1, 0xfb);
pub const MUTED: Color32 = rgb(0x8e, 0xa3, 0xbb);
pub const DIM: Color32 = rgb(0x5d, 0x71, 0x89);
pub const PRIMARY: Color32 = rgb(0x0a, 0x84, 0xd0);
pub const SKY: Color32 = rgb(0x38, 0xbd, 0xf8);
pub const CYAN: Color32 = rgb(0x22, 0xd3, 0xee);
pub const GREEN: Color32 = rgb(0x34, 0xd3, 0x99);
pub const YELLOW: Color32 = rgb(0xfb, 0xbf, 0x24);
pub const PURPLE: Color32 = rgb(0xa7, 0x8b, 0xfa);
pub const RED: Color32 = rgb(0xf8, 0x71, 0x71);
pub const ERR: Color32 = rgb(0xfc, 0xa5, 0xa5);
pub const BTN: Color32 = rgb(0x1a, 0x25, 0x36);
pub const BTN_HI: Color32 = rgb(0x23, 0x32, 0x47);
pub const WATER: Color32 = rgb(0x0f, 0x2b, 0x49);
pub const WATER_HI: Color32 = rgb(0x17, 0x49, 0x7a);
pub const WATER_HOVER: Color32 = rgb(0x1f, 0x5f, 0x9c);
pub const SHIP: Color32 = rgb(0x7b, 0x90, 0xa8);
pub const HIT: Color32 = rgb(0xef, 0x44, 0x44);
pub const SUNK: Color32 = rgb(0x7f, 0x1d, 0x1d);
pub const MISS: Color32 = rgb(0xcf, 0xe0, 0xf0);
pub const OK: Color32 = rgb(0x10, 0xb9, 0x81);
pub const INPUT_BG: Color32 = rgb(0x03, 0x07, 0x0f);
pub const MODAL: Color32 = rgb(0x0c, 0x1a, 0x2e);

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
        s.visuals = egui::Visuals::dark();
        s.visuals.override_text_color = Some(TEXT);
        s.visuals.panel_fill = Color32::TRANSPARENT;
        s.visuals.window_fill = MODAL;
        s.visuals.extreme_bg_color = INPUT_BG;
        s.visuals.selection.bg_fill = PRIMARY;
        s.visuals.selection.stroke = sk(1.0, SKY);
        s.visuals.text_cursor.stroke = sk(2.0, SKY);
        s.visuals.widgets.inactive.bg_stroke = sk(1.0, border2());
        s.visuals.widgets.inactive.rounding = Rounding::same(10.0);
        s.visuals.widgets.hovered.bg_stroke = sk(1.0, SKY);
        s.visuals.widgets.hovered.rounding = Rounding::same(10.0);
        s.visuals.widgets.active.bg_stroke = sk(1.0, SKY);
        s.visuals.widgets.active.rounding = Rounding::same(10.0);
        s.visuals.widgets.inactive.weak_bg_fill = INPUT_BG;
        s.visuals.widgets.hovered.weak_bg_fill = INPUT_BG;
        s.visuals.widgets.active.weak_bg_fill = INPUT_BG;
        s.visuals.widgets.open.weak_bg_fill = INPUT_BG;
        s.visuals.widgets.inactive.bg_fill = INPUT_BG;
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
    let stops = [(0.0, rgb(0x08, 0x1a, 0x31)), (0.55, rgb(0x05, 0x0d, 0x1b)), (1.0, rgb(0x03, 0x08, 0x10))];
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
    m.colored_vertex(c, rgba(22, 62, 110, 0.85));
    let rings = [(0.31, 0.6), (0.62, 0.0)];
    let seg = 48;
    let mut prev_ring_start = None;
    for (ri, (f, a)) in rings.iter().enumerate() {
        let start = m.vertices.len() as u32;
        for k in 0..seg {
            let ang = k as f32 / seg as f32 * std::f32::consts::TAU;
            m.colored_vertex(pos2(c.x + ang.cos() * rx * f, c.y + ang.sin() * ry * f), rgba(22, 62, 110, 0.85 * a));
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
        .rounding(Rounding::same(16.0))
        .inner_margin(Margin::same(16.0))
}

pub fn panel_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(card())
        .stroke(sk(1.0, border2()))
        .rounding(Rounding::same(16.0))
        .inner_margin(Margin::symmetric(32.0, 28.0))
        .shadow(Shadow { offset: vec2(0.0, 10.0), blur: 30.0, spread: 0.0, color: rgba(0, 0, 0, 0.35) })
}

pub fn sub_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(card2())
        .stroke(sk(1.0, border()))
        .rounding(Rounding::same(12.0))
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
            Bs::Danger => ERR,
            Bs::Ghost => MUTED,
            Bs::Normal => TEXT,
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
                Bs::Normal => (if hov { BTN_HI } else { BTN }, Stroke::NONE),
                Bs::Primary => (if hov { rgb(0x0b, 0x95, 0xea) } else { PRIMARY }, Stroke::NONE),
                Bs::Grad => (if hov { rgb(0x18, 0x7c, 0xec) } else { rgb(0x10, 0x67, 0xd2) }, Stroke::NONE),
                Bs::Danger => (if hov { rgb(0x4c, 0x11, 0x19) } else { rgb(0x3a, 0x0d, 0x14) }, sk(1.0, rgb(0x7f, 0x1d, 0x2a))),
                Bs::Ok => (if hov { rgb(0x07, 0xa8, 0x77) } else { rgb(0x05, 0x96, 0x69) }, Stroke::NONE),
                Bs::Ghost => (if hov { rgba(255, 255, 255, 0.05) } else { Color32::TRANSPARENT }, Stroke::NONE),
                Bs::On => (PRIMARY, sk(2.0, rgba(255, 255, 255, 0.85))),
            };
            let a = if self.enabled { 1.0 } else { 0.4 };
            let p = ui.painter();
            if self.style == Bs::Grad && self.enabled {
                p.rect_filled(rect.translate(vec2(0.0, 6.0)).expand(2.0), 12.0, rgba(14, 165, 233, 0.12));
            }
            p.rect(rect, 10.0, fill.gamma_multiply(a), sk(stroke.width, stroke.color.gamma_multiply(a)));
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
        MsgKind::Info => MUTED,
        MsgKind::Err => ERR,
        MsgKind::Ok => GREEN,
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
    p.rect(rect, 10.0, bg, sk(1.0, color.gamma_multiply(0.55)));
    p.text(rect.center(), Align2::CENTER_CENTER, icon, reg(size * 0.42), color);
    rect
}

pub fn icon_colors(kind: &str) -> (Color32, Color32) {
    match kind {
        "cyan" => (CYAN, rgba(8, 145, 178, 0.18)),
        "green" => (rgb(0x6e, 0xe7, 0xb7), rgba(5, 150, 105, 0.2)),
        "yellow" => (rgb(0xfc, 0xd3, 0x4d), rgba(180, 120, 10, 0.22)),
        _ => (rgb(0xc4, 0xb5, 0xfd), rgba(124, 58, 237, 0.22)),
    }
}

/// Barre d'XP.
pub fn xp_bar(ui: &mut Ui, frac: f32, w: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(w, 9.0), Sense::hover());
    let p = ui.painter();
    p.rect(rect, 6.0, rgb(0x06, 0x0d, 0x18), sk(1.0, rgba(100, 140, 180, 0.35)));
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
            ui.painter().rect_filled(screen, 0.0, rgba(2, 6, 14, 0.78));
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
                .fill(MODAL)
                .stroke(sk(1.0, border2()))
                .rounding(16.0)
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
    ui.painter().add(egui::Shape::line(pts, sk(4.0, SKY)));
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
    label(ui, t, reg(13.0), MUTED);
}
