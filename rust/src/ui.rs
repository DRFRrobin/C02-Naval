//! Affichage de tous les écrans (même disposition que la version web).

use eframe::egui::{self, pos2, vec2, Align, Align2, Color32, Key, Layout, Rect, Sense, Ui};
use egui_phosphor::regular as ic;

use crate::accounts::{level_of, valid_photo, xp_for, Stats};
use crate::app::*;
use crate::game::*;
use crate::i18n::Lang;
use crate::theme::{self, *};

/* =========================================================
   Grilles
   ========================================================= */

#[derive(Clone, Copy)]
pub struct Look {
    fill: Color32,
    sym: &'static str,
    sym_col: Color32,
    ring: Option<Color32>,
    last: bool,
    hover: Option<Color32>,
}
impl Default for Look {
    fn default() -> Self {
        Look { fill: WATER, sym: "", sym_col: TEXT, ring: None, last: false, hover: None }
    }
}

struct Geom {
    rect: Rect,
    lab: f32,
    cell: f32,
    gap: f32,
}
impl Geom {
    fn new(min: egui::Pos2, w: f32) -> Geom {
        let gap = 2.0;
        let cell = (w - 10.0 * gap) / 10.7;
        let lab = 0.7 * cell;
        let h = lab + gap + 10.0 * (cell + gap) - gap;
        Geom { rect: Rect::from_min_size(min, vec2(w, h)), lab, cell, gap }
    }
    fn cell_rect(&self, r: usize, c: usize) -> Rect {
        let x = self.rect.left() + self.lab + self.gap + c as f32 * (self.cell + self.gap);
        let y = self.rect.top() + self.lab + self.gap + r as f32 * (self.cell + self.gap);
        Rect::from_min_size(pos2(x, y), vec2(self.cell, self.cell))
    }
    fn cell_at(&self, p: egui::Pos2) -> Option<(usize, usize)> {
        let x = p.x - (self.rect.left() + self.lab + self.gap);
        let y = p.y - (self.rect.top() + self.lab + self.gap);
        if x < 0.0 || y < 0.0 {
            return None;
        }
        let c = (x / (self.cell + self.gap)) as usize;
        let r = (y / (self.cell + self.gap)) as usize;
        (r < N && c < N).then_some((r, c))
    }
}

/// Dessine une grille 10 × 10 avec ses repères (1–10 en haut, A–J à gauche).
fn draw_grid(ui: &mut Ui, width: f32, looks: &[Look], small: bool, outline: Option<Color32>, sense: Sense) -> (egui::Response, Geom) {
    let w = width.min(ui.available_width());
    let x0 = ui.cursor().left() + (ui.available_width() - w) / 2.0;
    let g = Geom::new(pos2(x0, ui.cursor().top()), w);
    let resp = ui.allocate_rect(Rect::from_min_size(pos2(ui.min_rect().left(), g.rect.top()), vec2(ui.available_width(), g.rect.height())), Sense::hover());
    let resp = ui.interact(g.rect, resp.id.with("grid"), sense).union(resp);
    let hovered = resp.hover_pos().and_then(|p| g.cell_at(p));
    let p = ui.painter();
    if let Some(c) = outline {
        p.rect_stroke(g.rect.expand(5.5), 6.0, sk(3.0, c));
    }
    let lf = reg(if small { 10.0 } else { 12.0 });
    let lfb = bold(if small { 10.0 } else { 12.0 });
    for c in 0..N {
        let cr = g.cell_rect(0, c);
        p.text(pos2(cr.center().x, g.rect.top() + g.lab / 2.0), Align2::CENTER_CENTER, (c + 1).to_string(), lfb.clone(), DIM);
    }
    for r in 0..N {
        let cr = g.cell_rect(r, 0);
        p.text(pos2(g.rect.left() + g.lab / 2.0, cr.center().y), Align2::CENTER_CENTER, (ROWS[r] as char).to_string(), lfb.clone(), DIM);
    }
    let _ = lf;
    let sym_size = (g.cell * 0.5).clamp(9.0, if small { 13.0 } else { 18.0 });
    for r in 0..N {
        for c in 0..N {
            let k = r * N + c;
            let l = looks[k];
            let cr = g.cell_rect(r, c);
            let mut fill = l.fill;
            if hovered == Some((r, c)) {
                if let Some(h) = l.hover {
                    fill = h;
                }
            }
            p.rect_filled(cr, 4.0, fill);
            if let Some(rc) = l.ring {
                p.rect_stroke(cr.shrink(1.0), 4.0, sk(2.0, rc));
            }
            if l.last {
                p.rect_stroke(cr.shrink(1.0), 3.0, sk(2.0, YELLOW));
            }
            if !l.sym.is_empty() {
                p.text(cr.center(), Align2::CENTER_CENTER, l.sym, bold(sym_size), l.sym_col);
            }
        }
    }
    if hovered.is_some_and(|(r, c)| looks[r * N + c].hover.is_some()) {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    (resp, g)
}

const SHIELD_SYM: &str = ic::SHIELD;

/* =========================================================
   Application
   ========================================================= */

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        theme::paint_background(ctx);
        // Mise à jour obligatoire : tant qu'elle n'est pas faite, le jeu reste bloqué.
        let upd = self.upd.lock().unwrap().clone();
        if upd != crate::update::UpdState::UpToDate {
            self.ui_update(ctx, upd);
            return;
        }
        self.tick(ctx);
        egui::CentralPanel::default().frame(egui::Frame::none()).show(ctx, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                let pad = if ui.available_width() < 640.0 { 12.0 } else { 16.0 };
                ui.add_space(pad);
                ui.horizontal(|ui| {
                    ui.add_space(pad);
                    let w = ui.available_width() - pad;
                    ui.vertical(|ui| {
                        ui.set_width(w);
                        centered(ui, 1000.0 - 32.0, |ui| match self.view {
                            View::Auth => self.ui_auth(ui),
                            View::Dash => self.ui_dash(ui),
                            View::Lobby => self.ui_lobby(ui),
                            View::Place => self.ui_place(ui),
                            View::Game => self.ui_game(ui),
                        });
                    });
                });
                ui.add_space(48.0);
            });
        });
        self.ui_modals(ctx);
        self.shortcuts(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Fermer la fenêtre en pleine partie = partie interrompue (comme quitter la page).
        if self.s.phase == Phase::Game {
            self.quit_game();
        }
        self.net.destroy_all();
    }
}

impl App {
    fn shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.wants_keyboard_input() {
            return;
        }
        let (r, enter, del, esc) = ctx.input(|i| {
            (i.key_pressed(Key::R), i.key_pressed(Key::Enter), i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace), i.key_pressed(Key::Escape))
        });
        if esc {
            if self.confirm.is_some() {
                self.confirm = None;
            } else if self.show_join {
                self.cancel_join();
            }
        }
        // Raccourcis pendant le placement : R pivote, Entrée pose, Suppr retire.
        if self.s.phase == Phase::Place && self.confirm.is_none() {
            if r {
                self.rotate_ship();
            } else if enter {
                self.confirm_placement();
            } else if del {
                self.remove_ship();
            }
        }
    }

    fn ui_update(&mut self, ctx: &egui::Context, st: crate::update::UpdState) {
        use crate::update::UpdState;
        egui::CentralPanel::default().frame(egui::Frame::none()).show(ctx, |ui| {
            let h = ui.available_height();
            ui.add_space((h * 0.22).max(16.0));
            centered(ui, 440.0, |ui| {
                egui::Frame::none()
                    .fill(card())
                    .stroke(sk(1.0, border2()))
                    .rounding(18.0)
                    .inner_margin(egui::Margin::symmetric(26.0, 28.0))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.vertical_centered(|ui| {
                            let (c, bg) = icon_colors("cyan");
                            mode_icon(ui, ic::ANCHOR, c, bg, 64.0);
                            label(ui, "C02 Naval", bold(26.0), TEXT);
                            label(ui, format!("v{}", crate::update::current_version()), reg(13.0), MUTED);
                            ui.add_space(12.0);
                            match &st {
                                UpdState::Checking => {
                                    spinner(ui);
                                    label(ui, self.t("updChecking"), reg(14.0), MUTED);
                                }
                                UpdState::Downloading { version, got, total } => {
                                    label(ui, self.t("updTitle"), bold(20.0), YELLOW);
                                    wrapped(ui, self.tf("updText", &[("to", version.clone()), ("from", crate::update::current_version())]), reg(14.0), MUTED);
                                    ui.add_space(10.0);
                                    let frac = if *total > 0 { *got as f32 / *total as f32 } else { 0.0 };
                                    let w = ui.available_width();
                                    xp_bar(ui, frac, w);
                                    let pct = if *total > 0 { format!("{} %", (frac * 100.0).round()) } else { format!("{:.1} Mo", *got as f32 / 1e6) };
                                    label(ui, self.tf("updDownloading", &[("pct", pct)]), reg(13.0), MUTED);
                                    ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
                                }
                                UpdState::Restarting => {
                                    spinner(ui);
                                    label(ui, self.t("updRestarting"), reg(14.0), GREEN);
                                }
                                UpdState::Failed { version, .. } => {
                                    label(ui, self.t("updTitle"), bold(20.0), YELLOW);
                                    label(ui, format!("v{} → v{}", crate::update::current_version(), version), num(14.0), SKY);
                                    ui.add_space(6.0);
                                    wrapped(ui, self.t("updFailed"), reg(14.0), ERR);
                                    ui.add_space(12.0);
                                    if Btn::new(&self.t("updOpen")).icon(ic::LINK).style(Bs::Primary).wide().show(ui).clicked() {
                                        let _ = std::process::Command::new("explorer").arg(crate::update::RELEASES_URL).spawn();
                                    }
                                    ui.add_space(4.0);
                                    if Btn::new(&self.t("updQuit")).style(Bs::Ghost).wide().show(ui).clicked() {
                                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                                    }
                                }
                                UpdState::UpToDate => {}
                            }
                        });
                    });
            });
        });
    }

    /// Petit avatar : photo, initiale ou robot.
    fn mini_av(&mut self, ui: &mut Ui, photo: Option<&str>, name: &str, size: f32, bot: bool) {
        let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
        let rounding = if size < 34.0 { 8.0 } else { 10.0 };
        ui.painter().rect(rect, rounding, rgb(0x0b, 0x1a, 0x2e), sk(1.0, border2()));
        let tex = photo.filter(|p| valid_photo(p)).and_then(|p| self.photo_tex(ui.ctx(), p));
        if let Some(t) = tex {
            paint_image(ui, &t, rect, rounding);
        } else if bot {
            ui.painter().text(rect.center(), Align2::CENTER_CENTER, ic::ROBOT, reg(size * 0.5), GREEN);
        } else {
            let ch = name.trim().chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or("?".into());
            ui.painter().text(rect.center(), Align2::CENTER_CENTER, ch, bold(size * 0.42), SKY);
        }
    }

    /* ---------- Connexion ---------- */

    fn ui_auth(&mut self, ui: &mut Ui) {
        let h = ui.ctx().screen_rect().height();
        ui.add_space(h * 0.07);
        centered(ui, 440.0, |ui| {
            egui::Frame::none()
                .fill(card())
                .stroke(sk(1.0, border2()))
                .rounding(18.0)
                .inner_margin(egui::Margin::symmetric(26.0, 28.0))
                .shadow(egui::Shadow { offset: vec2(0.0, 20.0), blur: 50.0, spread: 0.0, color: rgba(0, 0, 0, 0.45) })
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        let (c, bg) = icon_colors("cyan");
                        let (rect, _) = ui.allocate_exact_size(vec2(64.0, 64.0), Sense::hover());
                        ui.painter().rect(rect, 16.0, bg, sk(1.0, c.gamma_multiply(0.55)));
                        ui.painter().text(rect.center(), Align2::CENTER_CENTER, ic::ANCHOR, reg(30.0), c);
                        ui.add_space(4.0);
                        label(ui, "C02 Naval", bold(30.0), TEXT);
                        label(ui, self.t("tagline"), reg(14.0), MUTED);
                    });
                    ui.add_space(14.0);
                    // Onglets Connexion / Créer un compte
                    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::hover());
                    ui.painter().rect(rect, 12.0, INPUT_BG, sk(1.0, border()));
                    let half = (rect.width() - 12.0) / 2.0;
                    for (i, (key, signup)) in [("tabLogin", false), ("tabSignup", true)].into_iter().enumerate() {
                        let r = Rect::from_min_size(rect.min + vec2(4.0 + i as f32 * (half + 4.0), 4.0), vec2(half, 38.0));
                        let resp = ui.interact(r, ui.id().with(key), Sense::click());
                        let on = self.auth_signup == signup;
                        let fill = if on { PRIMARY } else if resp.hovered() { rgba(255, 255, 255, 0.04) } else { Color32::TRANSPARENT };
                        ui.painter().rect_filled(r, 9.0, fill);
                        ui.painter().text(r.center(), Align2::CENTER_CENTER, self.t(key), semi(14.0), if on { Color32::WHITE } else { MUTED });
                        if resp.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        if resp.clicked() {
                            self.auth_msg = Default::default();
                            self.open_auth(Some(signup));
                        }
                    }
                    ui.add_space(2.0);
                    if !self.auth_signup {
                        self.ui_login_pane(ui);
                    } else {
                        self.ui_signup_pane(ui);
                    }
                    msg(ui, &self.auth_msg.0.clone(), self.auth_msg.1, 14.0);
                    if Btn::new(&self.t("guest")).style(Bs::Ghost).wide().show(ui).clicked() {
                        self.guest();
                    }
                    ui.vertical_centered(|ui| {
                        label(ui, self.t("localNote"), reg(12.0), MUTED);
                        ui.add_space(6.0);
                        let aw = ui.available_width();
                        ui.horizontal(|ui| {
                            ui.add_space(((aw - 158.0) / 2.0).max(0.0));
                            self.lang_combo(ui, "langMini", 130.0);
                        });
                    });
                });
        });
    }

    fn lang_combo(&mut self, ui: &mut Ui, id: &str, w: f32) {
        let mut l = self.lang();
        let name = |l: Lang| if l == Lang::Fr { "Français" } else { "English" };
        egui::ComboBox::from_id_salt(id)
            .selected_text(egui::RichText::new(name(l)).font(reg(14.0)))
            .width(w)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut l, Lang::Fr, "Français");
                ui.selectable_value(&mut l, Lang::En, "English");
            });
        if l != self.lang() {
            self.set_lang(l);
        }
    }

    fn ui_login_pane(&mut self, ui: &mut Ui) {
        field_label(ui, &self.t("account"));
        let accs = self.store.sorted();
        if accs.is_empty() {
            msg(ui, &self.t("noAccounts"), MsgKind::Info, 14.0);
        }
        egui::ScrollArea::vertical().id_salt("accPick").max_height(230.0).show(ui, |ui| {
            for a in &accs {
                let on = self.picked.as_deref() == Some(a.id.as_str());
                let resp = egui::Frame::none()
                    .fill(card2())
                    .stroke(sk(if on { 2.0 } else { 1.0 }, if on { SKY } else { border() }))
                    .rounding(12.0)
                    .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            self.mini_av(ui, a.photo.as_deref(), &a.name, 36.0, false);
                            ui.add_space(4.0);
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 0.0;
                                label(ui, &a.name, bold(15.0), TEXT);
                                let l = level_of(a.xp);
                                label(ui, format!("{} · {} XP", self.tf("levelN", &[("n", l.to_string())]), a.xp), reg(12.0), MUTED);
                            });
                        });
                    })
                    .response;
                let resp = ui.interact(resp.rect, ui.id().with(&a.id), Sense::click());
                if resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if resp.clicked() {
                    self.picked = Some(a.id.clone());
                    self.focus_pass = true;
                }
            }
        });
        field_label(ui, &self.t("password"));
        let r = text_input(ui, &mut self.login_pass, true, 128, "");
        if self.focus_pass {
            r.request_focus();
            self.focus_pass = false;
        }
        if r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
            self.do_login();
        }
        ui.add_space(10.0);
        if Btn::new(&self.t("btnLogin")).icon(ic::SIGN_IN).style(Bs::Primary).wide().enabled(!accs.is_empty()).show(ui).clicked() {
            self.do_login();
        }
    }

    fn ui_signup_pane(&mut self, ui: &mut Ui) {
        field_label(ui, &self.t("pseudo"));
        text_input(ui, &mut self.su_name, false, 16, "");
        field_label(ui, &self.t("password"));
        text_input(ui, &mut self.su_pass, true, 128, "");
        field_label(ui, &self.t("passwordConfirm"));
        let r = text_input(ui, &mut self.su_pass2, true, 128, "");
        if r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
            self.do_signup();
        }
        ui.add_space(10.0);
        if Btn::new(&self.t("btnSignup")).icon(ic::USER_PLUS).style(Bs::Primary).wide().show(ui).clicked() {
            self.do_signup();
        }
    }

    /* ---------- Tableau de bord ---------- */

    fn ui_dash(&mut self, ui: &mut Ui) {
        self.ui_topbar(ui);
        match self.pane {
            Pane::Play => self.ui_play(ui),
            Pane::Stats => self.ui_stats(ui),
            Pane::Settings => self.ui_settings(ui),
        }
    }

    fn ui_topbar(&mut self, ui: &mut Ui) {
        let tiny = ui.available_width() < 520.0;
        // Largeur des 4 boutons de navigation : s'ils ne tiennent pas à côté du joueur, on passe sur 2 lignes.
        let nav_w: f32 = ["navPlay", "navStats", "navSettings", "navLogout"]
            .iter()
            .map(|k| ui.painter().layout_no_wrap(format!("{}  {}", ic::GEAR, self.t(k)), semi(15.0), TEXT).size().x + 32.0 + 8.0)
            .sum();
        let inner = ui.available_width() - 36.0;
        let who_max = inner - 64.0 - 16.0 - nav_w - 8.0;
        let narrow = who_max < 170.0;
        let acc = self.me().cloned();
        egui::Frame::none()
            .fill(card())
            .stroke(sk(1.0, border()))
            .rounding(16.0)
            .inner_margin(if narrow { egui::Margin::same(12.0) } else { egui::Margin::symmetric(18.0, 14.0) })
            .shadow(egui::Shadow { offset: vec2(0.0, 10.0), blur: 30.0, spread: 0.0, color: rgba(0, 0, 0, 0.35) })
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                let who = |this: &mut App, ui: &mut Ui, maxw: f32| {
                    // Avatar (clic = changer la photo)
                    let (rect, resp) = ui.allocate_exact_size(vec2(64.0, 64.0), if acc.is_some() { Sense::click() } else { Sense::hover() });
                    let photo = acc.as_ref().and_then(|a| a.photo.clone()).filter(|p| valid_photo(p));
                    ui.painter().rect_filled(rect, 14.0, rgba(0, 0, 0, 0.2));
                    match photo.and_then(|p| this.photo_tex(ui.ctx(), &p)) {
                        Some(t) => {
                            paint_image(ui, &t, rect.shrink(2.0), 12.0);
                            ui.painter().rect_stroke(rect, 14.0, sk(2.0, border2()));
                        }
                        None => {
                            if acc.is_some() {
                                dashed_rect(ui, rect.shrink(1.0), sk(2.0, SKY));
                            } else {
                                ui.painter().rect_stroke(rect, 14.0, sk(2.0, SKY));
                            }
                            let i = if acc.is_some() { ic::CAMERA } else { ic::USER };
                            ui.painter().text(rect.center(), Align2::CENTER_CENTER, i, reg(20.0), DIM);
                        }
                    }
                    if acc.is_some() {
                        let resp = resp.on_hover_text(this.t("photoChange"));
                        if resp.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        if resp.clicked() {
                            this.pick_photo();
                        }
                    }
                    ui.add_space(8.0);
                    let w = maxw.min(330.0).max(140.0);
                    ui.vertical(|ui| {
                        ui.set_width(w);
                        ui.spacing_mut().item_spacing.y = 2.0;
                        ui.horizontal(|ui| {
                            let name = acc.as_ref().map_or(this.t("guestName"), |a| a.name.clone());
                            label(ui, name, bold(18.0), TEXT);
                            if let Some(a) = &acc {
                                let l = level_of(a.xp);
                                let txt = this.tf("badge", &[("n", l.to_string())]);
                                let g = ui.painter().layout_no_wrap(txt, bold(12.0), rgb(0x67, 0xe8, 0xf9));
                                let (r, _) = ui.allocate_exact_size(g.size() + vec2(14.0, 4.0), Sense::hover());
                                ui.painter().rect(r, 6.0, rgba(14, 116, 144, 0.3), sk(1.0, rgba(34, 211, 238, 0.45)));
                                ui.painter().galley(r.center() - g.size() / 2.0, g, TEXT);
                            }
                        });
                        if let Some(a) = &acc {
                            let l = level_of(a.xp);
                            let (lo, hi) = (xp_for(l), xp_for(l + 1));
                            ui.add_space(5.0);
                            xp_bar(ui, (a.xp - lo) as f32 / (hi - lo) as f32, w);
                            ui.horizontal(|ui| {
                                label(ui, "XP:", reg(11.0), MUTED);
                                label(ui, format!("{} / {}", a.xp - lo, hi - lo), bold(11.0), SKY);
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    label(ui, format!("{} · {}", this.tf("levelN", &[("n", l.to_string())]), this.rank_of(l)), reg(11.0), MUTED);
                                });
                            });
                        }
                    });
                };
                let nav = |this: &mut App, ui: &mut Ui, w: Option<f32>| {
                    let items = [
                        (Some(Pane::Play), ic::CROSSHAIR, "navPlay"),
                        (Some(Pane::Stats), ic::CHART_PIE, "navStats"),
                        (Some(Pane::Settings), ic::GEAR, "navSettings"),
                        (None, ic::POWER, "navLogout"),
                    ];
                    for (p, icon, key) in items {
                        let text = if tiny { String::new() } else { this.t(key) };
                        let style = match p {
                            None => Bs::Danger,
                            Some(p) if p == this.pane => Bs::On,
                            _ => Bs::Normal,
                        };
                        let mut b = Btn::new(&text).icon(icon).style(style);
                        if let Some(w) = w {
                            b = b.width(w);
                        }
                        if b.show(ui).clicked() {
                            match p {
                                Some(p) => this.set_pane(p),
                                None => this.logout(),
                            }
                        }
                    }
                };
                if narrow {
                    let aw = ui.available_width();
                    ui.horizontal(|ui| {
                        let m = aw - 80.0;
                        who(self, ui, m)
                    });
                    ui.add_space(6.0);
                    let aw = ui.available_width();
                    ui.horizontal(|ui| {
                        let w = (aw - 3.0 * 8.0) / 4.0;
                        nav(self, ui, Some(w));
                    });
                } else {
                    ui.horizontal(|ui| {
                        who(self, ui, who_max);
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            // ordre inversé (de droite à gauche)
                            let mut tmp: Vec<(Option<Pane>, &str, &str)> = vec![
                                (Some(Pane::Play), ic::CROSSHAIR, "navPlay"),
                                (Some(Pane::Stats), ic::CHART_PIE, "navStats"),
                                (Some(Pane::Settings), ic::GEAR, "navSettings"),
                                (None, ic::POWER, "navLogout"),
                            ];
                            tmp.reverse();
                            for (p, icon, key) in tmp {
                                let text = self.t(key);
                                let style = match p {
                                    None => Bs::Danger,
                                    Some(p) if p == self.pane => Bs::On,
                                    _ => Bs::Normal,
                                };
                                if Btn::new(&text).icon(icon).style(style).show(ui).clicked() {
                                    match p {
                                        Some(p) => self.set_pane(p),
                                        None => self.logout(),
                                    }
                                }
                            }
                        });
                    });
                }
            });
    }

    fn ui_play(&mut self, ui: &mut Ui) {
        let h = ui.ctx().screen_rect().height();
        let one_col = ui.available_width() < 640.0;
        ui.add_space(if one_col { 20.0 } else { (h * 0.13).clamp(24.0, 150.0) });
        let cards = [
            ("cyan", ic::SHUFFLE, "mRandomT", "mRandomD"),
            ("green", ic::ROBOT, "mBotT", "mBotD"),
            ("yellow", ic::SIGN_IN, "mJoinT", "mJoinD"),
            ("purple", ic::BROADCAST, "mHostT", "mHostD"),
        ];
        let mut clicked = None;
        centered(ui, 770.0, |ui| {
            let gap = if one_col { 12.0 } else { 24.0 };
            let cols = if one_col { 1 } else { 2 };
            let w = (ui.available_width() - gap * (cols as f32 - 1.0)) / cols as f32;
            for row in cards.chunks(cols) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for (i, (col, icon, tk, dk)) in row.iter().enumerate() {
                        let resp = egui::Frame::none()
                            .fill(card())
                            .stroke(sk(1.0, border2()))
                            .rounding(16.0)
                            .inner_margin(egui::Margin::same(if one_col { 16.0 } else { 24.0 }))
                            .show(ui, |ui| {
                                ui.set_width(w - if one_col { 32.0 } else { 48.0 });
                                ui.vertical(|ui| {
                                let (c, bg) = icon_colors(col);
                                if one_col {
                                    ui.horizontal(|ui| {
                                        mode_icon(ui, icon, c, bg, 48.0);
                                        ui.add_space(6.0);
                                        ui.vertical(|ui| {
                                            label(ui, self.t(tk), bold(17.0), TEXT);
                                            wrapped(ui, self.t(dk), reg(14.0), MUTED);
                                        });
                                    });
                                } else {
                                    ui.set_min_height(150.0);
                                    mode_icon(ui, icon, c, bg, 48.0);
                                    ui.add_space(18.0);
                                    label(ui, self.t(tk), bold(20.0), TEXT);
                                    wrapped(ui, self.t(dk), reg(14.0), MUTED);
                                }
                                });
                            })
                            .response;
                        let resp = ui.interact(resp.rect, ui.id().with(tk), Sense::click());
                        if resp.hovered() {
                            ui.painter().rect_stroke(resp.rect, 16.0, sk(1.5, rgba(56, 189, 248, 0.6)));
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        if resp.clicked() {
                            clicked = Some(*tk);
                        }
                        let _ = i;
                    }
                });
                ui.add_space(gap - 8.0);
            }
            msg(ui, &self.dash_msg.0.clone(), self.dash_msg.1, 14.0);
        });
        if let Some(k) = clicked {
            self.play("click");
            self.dash_msg = Default::default();
            match k {
                "mRandomT" => self.find_random(),
                "mBotT" => self.start_bot(),
                "mJoinT" => self.open_join(""),
                _ => self.host_game(),
            }
        }
    }

    fn panel_title(&self, ui: &mut Ui, icon: &str, key: &str) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            label(ui, icon, reg(22.0), CYAN);
            label(ui, self.t(key), bold(if ui.available_width() < 600.0 { 21.0 } else { 26.0 }), TEXT);
        });
        ui.add_space(12.0);
        let r = ui.available_rect_before_wrap();
        ui.painter().hline(r.x_range(), r.top(), sk(1.0, border()));
        ui.add_space(20.0);
    }

    fn tile_grid(&self, ui: &mut Ui, tiles: &[(&str, Color32, String, String, Color32)]) {
        let cols = if ui.available_width() < 560.0 { 2 } else { 4 };
        let gap = if cols == 2 { 10.0 } else { 16.0 };
        let w = (ui.available_width() - gap * (cols as f32 - 1.0)) / cols as f32;
        for row in tiles.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for (icon, ic_col, val, lab, val_col) in row {
                    let (rect, _) = ui.allocate_exact_size(vec2(w, 112.0), Sense::hover());
                    let p = ui.painter();
                    p.rect(rect, 12.0, card2(), sk(1.0, border()));
                    let cx = rect.center().x;
                    p.text(pos2(cx, rect.top() + 30.0), Align2::CENTER_CENTER, *icon, reg(20.0), *ic_col);
                    p.text(pos2(cx, rect.top() + 62.0), Align2::CENTER_CENTER, val, num(20.0), *val_col);
                    let g = p.layout(lab.clone(), reg(13.0), MUTED, w - 12.0);
                    p.galley(pos2(cx - g.size().x / 2.0, rect.top() + 82.0), g, MUTED);
                }
            });
            ui.add_space(gap - 8.0);
        }
    }

    fn ui_stats(&mut self, ui: &mut Ui) {
        ui.add_space(24.0);
        let acc = self.me().cloned();
        let s: Stats = acc.as_ref().map(|a| a.stats.clone()).unwrap_or_default();
        let pct = |x: u32, y: u32| format!("{}%", if y > 0 { (x as f64 / y as f64 * 100.0).round() as u32 } else { 0 });
        let rate = |x: u32, y: u32| format!("{:.2}", if y > 0 { x as f64 / y as f64 } else { 0.0 });
        centered(ui, 770.0, |ui| {
            panel_frame().show(ui, |ui| {
                ui.set_width(ui.available_width());
                self.panel_title(ui, ic::CHART_LINE, "statsTitle");
                let career = [
                    (ic::CLOCK, SKY, fmt_time(s.time_played), self.t("stTime"), TEXT),
                    (ic::GAME_CONTROLLER, PURPLE, s.games.to_string(), self.t("stGames"), TEXT),
                    (ic::TROPHY, GREEN, s.wins.to_string(), self.t("stWins"), GREEN),
                    (ic::SKULL, RED, s.losses.to_string(), self.t("stLosses"), RED),
                    (ic::PERCENT, SKY, pct(s.wins, s.games), self.t("stWinPct"), SKY),
                    (ic::PERCENT, YELLOW, pct(s.losses, s.games), self.t("stLossPct"), YELLOW),
                    (ic::TREND_UP, GREEN, rate(s.wins, s.games), self.t("stWinRate"), GREEN),
                    (ic::TREND_DOWN, RED, rate(s.losses, s.games), self.t("stLossRate"), RED),
                ];
                self.tile_grid(ui, &career);
                ui.add_space(18.0);
                label(ui, self.t("combatTitle").to_uppercase(), bold(15.0), MUTED);
                ui.add_space(8.0);
                let fast = s.fastest_win.map_or("—".to_string(), |n| self.tf("shotsN", &[("n", n.to_string())]));
                let combat = [
                    (ic::CROSSHAIR, SKY, s.shots.to_string(), self.t("stShots"), TEXT),
                    (ic::TARGET, GREEN, pct(s.hits, s.shots), self.t("stAcc"), GREEN),
                    (ic::BOAT, RED, s.sunk.to_string(), self.t("stSunk"), TEXT),
                    (ic::MAGIC_WAND, PURPLE, s.upgrades.to_string(), self.t("stUpg"), TEXT),
                    (ic::FIRE, YELLOW, s.streak.to_string(), self.t("stStreak"), YELLOW),
                    (ic::MEDAL, YELLOW, s.best_streak.to_string(), self.t("stBest"), YELLOW),
                    (ic::LIGHTNING, SKY, fast, self.t("stFast"), TEXT),
                    (ic::PLUGS, RED, s.interrupted.to_string(), self.t("stInt"), TEXT),
                ];
                self.tile_grid(ui, &combat);
                if acc.is_none() {
                    msg(ui, &self.t("guestStats"), MsgKind::Info, 14.0);
                }
            });
        });
    }

    fn ui_settings(&mut self, ui: &mut Ui) {
        ui.add_space(24.0);
        let acc = self.me().cloned();
        centered(ui, 770.0, |ui| {
            panel_frame().show(ui, |ui| {
                ui.set_width(ui.available_width());
                self.panel_title(ui, ic::SLIDERS, "setTitle");
                // Sons
                sub_frame().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        label(ui, ic::SPEAKER_HIGH, reg(20.0), SKY);
                        ui.add_space(6.0);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            label(ui, self.t("sndT"), bold(16.0), TEXT);
                            wrapped(ui, self.t("sndD"), reg(13.0), MUTED);
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let on = self.settings.sound;
                            let g = ui.painter().layout_no_wrap(if on { "ON" } else { "OFF" }.into(), bold(13.0), if on { Color32::WHITE } else { rgb(0xcb, 0xd5, 0xe1) });
                            let (r, resp) = ui.allocate_exact_size(vec2(58.0, 36.0), Sense::click());
                            ui.painter().rect_filled(r, 9.0, if on { rgb(0x05, 0x96, 0x69) } else { rgb(0x33, 0x41, 0x55) });
                            ui.painter().galley(r.center() - g.size() / 2.0, g, TEXT);
                            if resp.hovered() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                            if resp.clicked() {
                                self.toggle_sound();
                            }
                        });
                    });
                });
                ui.add_space(8.0);
                // Langue
                sub_frame().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    label(ui, self.t("langT"), bold(16.0), TEXT);
                    label(ui, self.t("langD"), reg(13.0), MUTED);
                    ui.add_space(4.0);
                    let w = ui.available_width();
                    self.lang_combo(ui, "langSel", w - 8.0);
                });
                if let Some(a) = acc {
                    ui.add_space(8.0);
                    sub_frame().show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            label(ui, ic::IMAGE, reg(16.0), YELLOW);
                            label(ui, self.t("photoT"), bold(16.0), TEXT);
                        });
                        wrapped(ui, self.t("photoD"), reg(13.0), MUTED);
                        ui.add_space(6.0);
                        let aw = ui.available_width();
                        ui.horizontal(|ui| {
                            let w = (aw - 10.0) / 2.0;
                            ui.spacing_mut().item_spacing.x = 10.0;
                            if Btn::new(&self.t("photoChange")).icon(ic::CAMERA).width(w).show(ui).clicked() {
                                self.pick_photo();
                            }
                            let has = a.photo.as_deref().is_some_and(valid_photo);
                            if Btn::new(&self.t("photoRemove")).icon(ic::X).width(w).enabled(has).show(ui).clicked() {
                                self.remove_photo();
                            }
                        });
                    });
                    ui.add_space(8.0);
                    sub_frame().show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            label(ui, ic::USER_GEAR, reg(16.0), YELLOW);
                            wrapped(ui, self.t("editT"), bold(16.0), TEXT);
                        });
                        field_label(ui, &self.t("newPseudo"));
                        text_input(ui, &mut self.edit_name, false, 16, "");
                        field_label(ui, &self.t("newPass"));
                        let r = text_input(ui, &mut self.edit_pass, true, 128, "••••••••");
                        if r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                            self.save_profile();
                        }
                        ui.add_space(10.0);
                        if Btn::new(&self.t("save")).style(Bs::Grad).wide().show(ui).clicked() {
                            self.save_profile();
                        }
                        msg(ui, &self.set_msg.0.clone(), self.set_msg.1, 14.0);
                    });
                    ui.add_space(8.0);
                    sub_frame().show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let aw = ui.available_width();
                        ui.horizontal(|ui| {
                            label(ui, ic::TRASH, reg(20.0), RED);
                            ui.add_space(6.0);
                            let w = (aw - 110.0).max(100.0);
                            ui.vertical(|ui| {
                                ui.set_width(w);
                                ui.spacing_mut().item_spacing.y = 2.0;
                                label(ui, self.t("deleteT"), bold(16.0), TEXT);
                                wrapped(ui, self.t("deleteD"), reg(13.0), MUTED);
                            });
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if Btn::new(&self.t("deleteBtn")).style(Bs::Danger).small().show(ui).clicked() {
                                    self.confirm = Some(Confirm::DeleteAccount);
                                }
                            });
                        });
                    });
                }
            });
        });
    }

    /* ---------- Salon ---------- */

    fn ui_lobby(&mut self, ui: &mut Ui) {
        let h = ui.ctx().screen_rect().height();
        ui.add_space(h * 0.09);
        let host = self.s.kind == Some(Kind::Host);
        let mut cancel = false;
        centered(ui, 460.0, |ui| {
            egui::Frame::none()
                .fill(card())
                .stroke(sk(1.0, border2()))
                .rounding(18.0)
                .inner_margin(egui::Margin::symmetric(24.0, 28.0))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.vertical_centered(|ui| {
                        let (c, bg) = icon_colors(if host { "purple" } else { "cyan" });
                        mode_icon(ui, if host { ic::BROADCAST } else { ic::SHUFFLE }, c, bg, 48.0);
                        ui.add_space(4.0);
                        label(ui, self.t(if host { "lobbyHostT" } else { "lobbyRandomT" }), bold(24.0), TEXT);
                        wrapped(ui, self.t(if host { "lobbyHostD" } else { "lobbyRandomD" }), reg(15.0), MUTED);
                        if host {
                            ui.add_space(10.0);
                            let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 76.0), Sense::hover());
                            ui.painter().rect(r, 12.0, INPUT_BG, sk(1.0, border2()));
                            let code = if self.s.code.is_empty() { "······".to_string() } else { self.s.code.clone() };
                            let n = code.chars().count() as f32;
                            let step = 38.0 * 1.0;
                            let x0 = r.center().x - step * (n - 1.0) / 2.0;
                            for (i, ch) in code.chars().enumerate() {
                                ui.painter().text(pos2(x0 + i as f32 * step, r.center().y), Align2::CENTER_CENTER, ch.to_string(), num(36.0), CYAN);
                            }
                            ui.add_space(8.0);
                            let aw = ui.available_width();
                            ui.horizontal(|ui| {
                                let w = (aw - 10.0) / 2.0;
                                ui.spacing_mut().item_spacing.x = 10.0;
                                if Btn::new(&self.t("copyCode")).icon(ic::COPY).width(w).show(ui).clicked() {
                                    let c = self.s.code.clone();
                                    self.copy(c);
                                }
                                if Btn::new(&self.t("copyLink")).icon(ic::LINK).width(w).show(ui).clicked() {
                                    let l = format!("{WEB_URL}?code={}", self.s.code);
                                    self.copy(l);
                                }
                            });
                        }
                        ui.add_space(12.0);
                        spinner(ui);
                        msg(ui, &self.lobby_msg.0.clone(), self.lobby_msg.1, 14.0);
                        if let Some(left) = self.countdown_left() {
                            label(ui, self.tf("countdown", &[("s", left.to_string())]), num(15.0), YELLOW);
                        }
                        ui.add_space(6.0);
                        if Btn::new(&self.t("cancel")).style(Bs::Ghost).wide().show(ui).clicked() {
                            cancel = true;
                        }
                    });
                });
        });
        if cancel {
            self.reset_to_menu();
        }
    }

    /* ---------- En-tête de partie ---------- */

    fn ui_ghead(&mut self, ui: &mut Ui) {
        let me = self.my_identity();
        let opp = self.s.opp.clone();
        egui::Frame::none()
            .fill(card())
            .stroke(sk(1.0, border()))
            .rounding(14.0)
            .inner_margin(egui::Margin::symmetric(14.0, 10.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let who = |this: &mut App, ui: &mut Ui, p: &Opp| {
                        this.mini_av(ui, p.photo.as_deref(), &p.name, 30.0, p.bot);
                        label(ui, &p.name, bold(14.0), TEXT);
                        if p.level > 0 {
                            label(ui, this.tf("levelShort", &[("n", p.level.to_string())]), bold(11.0), rgb(0x67, 0xe8, 0xf9));
                        }
                    };
                    who(self, ui, &me);
                    if let Some(o) = &opp {
                        ui.add_space(4.0);
                        label(ui, self.t("vs").to_uppercase(), semi(12.0), DIM);
                        ui.add_space(4.0);
                        who(self, ui, o);
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if Btn::new(&self.t("quit")).icon(ic::DOOR_OPEN).style(Bs::Ghost).small().show(ui).clicked() {
                            if self.s.phase == Phase::Over {
                                self.reset_to_menu();
                            } else {
                                self.confirm = Some(Confirm::Quit);
                            }
                        }
                    });
                });
            });
        ui.add_space(6.0);
    }

    /* ---------- Placement ---------- */

    fn ui_place(&mut self, ui: &mut Ui) {
        self.ui_ghead(ui);
        let wide = ui.available_width() >= 880.0;
        if wide {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 16.0;
                let side = 330.0;
                let gw = ui.available_width() - side - 16.0;
                ui.vertical(|ui| {
                    ui.set_width(gw);
                    ui.spacing_mut().item_spacing.x = 8.0;
                    card_frame().show(ui, |ui| {
                        ui.set_width(gw - 32.0);
                        self.place_grid(ui);
                    });
                });
                ui.vertical(|ui| {
                    ui.set_width(side);
                    ui.spacing_mut().item_spacing.x = 8.0;
                    card_frame().show(ui, |ui| {
                        ui.set_width(side - 32.0);
                        self.place_side(ui);
                    });
                });
            });
        } else {
            card_frame().show(ui, |ui| {
                ui.set_width(ui.available_width());
                self.place_grid(ui);
            });
            ui.add_space(8.0);
            card_frame().show(ui, |ui| {
                ui.set_width(ui.available_width());
                self.place_side(ui);
            });
        }
    }

    fn place_grid(&mut self, ui: &mut Ui) {
        let occ = self.occupancy();
        let pv = self.preview_info();
        let mut looks = [Look::default(); N * N];
        for k in 0..N * N {
            looks[k].hover = None;
            if occ[k] >= 0 {
                looks[k].fill = SHIP;
            }
        }
        if let Some((cells, valid)) = &pv {
            for &(r, c) in cells {
                let l = &mut looks[r * N + c];
                if *valid {
                    l.fill = OK;
                    l.ring = Some(rgb(0xa7, 0xf3, 0xd0));
                } else {
                    l.fill = HIT.gamma_multiply(0.85);
                }
            }
        }
        let (resp, g) = draw_grid(ui, 460.0, &looks, false, None, Sense::click_and_drag());
        if self.s.me_ready {
            return;
        }
        if resp.hover_pos().and_then(|p| g.cell_at(p)).is_some() {
            let grab = resp.hover_pos().and_then(|p| g.cell_at(p)).is_some_and(|(r, c)| occ[r * N + c] >= 0);
            ui.ctx().set_cursor_icon(if grab { egui::CursorIcon::Grab } else { egui::CursorIcon::PointingHand });
        }
        let (pressed, released, down, pos, secondary) = ui.input(|i| {
            (i.pointer.primary_pressed(), i.pointer.primary_released(), i.pointer.primary_down(), i.pointer.interact_pos(), i.pointer.secondary_clicked())
        });
        if pressed && resp.hovered() {
            if let Some((r, c)) = pos.and_then(|p| g.cell_at(p)) {
                self.place_down(r, c);
            }
        } else if down && self.s.drag.is_some() {
            if let Some((r, c)) = pos.and_then(|p| g.cell_at(p)) {
                self.place_move(r, c);
            }
        }
        if released && self.s.drag.is_some() {
            self.place_up();
        }
        // Clic droit : pivoter (en plus de la touche R).
        if secondary && resp.hovered() {
            self.rotate_ship();
        }
    }

    fn place_side(&mut self, ui: &mut Ui) {
        label(ui, self.t("placeTitle"), bold(20.0), TEXT);
        wrapped(ui, self.t("placeHelp"), reg(13.5), MUTED);
        ui.add_space(6.0);
        // Liste des bateaux (boutons qui se répartissent sur la largeur).
        let w = ui.available_width();
        let gap = 6.0;
        let per_row = (((w + gap) / (120.0 + gap)).floor() as usize).max(1);
        let mut clicked = None;
        for row in (0..5).collect::<Vec<_>>().chunks(per_row) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                let bw = (w - gap * (row.len() as f32 - 1.0)) / row.len() as f32;
                for &i in row {
                    let s = &self.s.ships[i];
                    let placed = s.cells.is_some() || self.s.lifted.as_ref().is_some_and(|l| l.idx == i);
                    let sel = i as i32 == self.s.sel && !self.s.me_ready;
                    let sense = if self.s.me_ready { Sense::hover() } else { Sense::click() };
                    let (r, resp) = ui.allocate_exact_size(vec2(bw, 50.0), sense);
                    let a = if placed && !sel { 0.55 } else { 1.0 };
                    let a = if self.s.me_ready { a * 0.6 } else { a };
                    let p = ui.painter();
                    p.rect(r, 10.0, card2().gamma_multiply(a), sk(if sel { 2.0 } else { 1.0 }, if sel { YELLOW } else { border() }));
                    p.text(r.left_top() + vec2(10.0, 8.0), Align2::LEFT_TOP, self.ship_name(i), semi(12.5), TEXT.gamma_multiply(a));
                    for b in 0..s.len {
                        let br = Rect::from_min_size(r.left_top() + vec2(10.0 + b as f32 * 12.0, 30.0), vec2(10.0, 10.0));
                        p.rect_filled(br, 2.0, if placed { OK } else { SHIP }.gamma_multiply(a));
                    }
                    if resp.hovered() && !self.s.me_ready {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if resp.clicked() {
                        clicked = Some(i);
                    }
                }
            });
        }
        if let Some(i) = clicked {
            self.select_ship(i);
        }
        ui.add_space(4.0);
        let rd = self.s.me_ready;
        let horiz = self.s.preview.map_or(self.s.horiz, |p| p.h);
        let aw = ui.available_width();
        ui.horizontal(|ui| {
            let bw = (aw - 16.0) / 3.0;
            if Btn::new(&self.t(if horiz { "rotH" } else { "rotV" })).icon(ic::ARROWS_CLOCKWISE).width(bw).small().enabled(!rd).show(ui).clicked() {
                self.rotate_ship();
            }
            if Btn::new(&self.t("random")).icon(ic::DICE_FIVE).width(bw).small().enabled(!rd).show(ui).clicked() {
                self.random_fleet();
            }
            if Btn::new(&self.t("clear")).icon(ic::BROOM).width(bw).small().enabled(!rd).show(ui).clicked() {
                self.clear_fleet();
            }
        });
        let pv_ok = self.preview_info().is_some_and(|p| p.1);
        let lifted = self.s.lifted.is_some();
        let aw = ui.available_width();
        ui.horizontal(|ui| {
            let bw = (aw - 10.0) / 2.0;
            ui.spacing_mut().item_spacing.x = 10.0;
            if Btn::new(&self.t("confirm")).icon(ic::CHECK).style(Bs::Ok).width(bw).enabled(!rd && pv_ok).show(ui).clicked() {
                self.confirm_placement();
            }
            if Btn::new(&self.t("remove")).icon(ic::X).width(bw).enabled(!rd && lifted).show(ui).clicked() {
                self.remove_ship();
            }
        });
        // Message d'aide
        let (text, kind) = if self.s.me_ready {
            (if self.s.opp_ready { String::new() } else { self.t("waitOppReady") }, MsgKind::Info)
        } else if self.s.opp_away {
            (self.t("oppAway"), MsgKind::Info)
        } else if self.s.opp_ready && self.place_msg.0.is_empty() {
            (self.t("oppReady"), MsgKind::Info)
        } else {
            self.place_msg.clone()
        };
        msg(ui, &text, kind, 14.0);
        let complete = self.next_unplaced() == -1 && !lifted;
        if Btn::new(&self.t("ready")).icon(ic::FLAG_CHECKERED).style(Bs::Primary).wide().enabled(complete && !rd).show(ui).clicked() {
            self.set_ready();
        }
    }

    /* ---------- Partie ---------- */

    fn ui_game(&mut self, ui: &mut Ui) {
        self.ui_ghead(ui);
        // Bandeau d'état
        let (text, fill, col) = if self.s.lost {
            (self.t("stLost"), BTN, MUTED)
        } else if self.s.phase == Phase::Over {
            (self.t("stOver"), BTN, MUTED)
        } else if self.s.waiting {
            (self.t("stFiring"), rgb(0xb4, 0x53, 0x09), Color32::WHITE)
        } else if self.s.my_turn {
            (self.t("stMyTurn"), rgb(0x05, 0x96, 0x69), Color32::WHITE)
        } else {
            (self.t("stOppTurn"), BTN, MUTED)
        };
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 44.0), Sense::hover());
        ui.painter().rect_filled(r, 12.0, fill);
        ui.painter().text(r.center(), Align2::CENTER_CENTER, text, bold(17.5), col);
        ui.add_space(8.0);

        let wide = ui.available_width() >= 880.0;
        if wide {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 16.0;
                let total = ui.available_width() - 16.0;
                let w1 = total * 1.3 / 2.3;
                let w2 = total - w1;
                ui.vertical(|ui| {
                    ui.set_width(w1);
                    ui.spacing_mut().item_spacing.x = 8.0;
                    card_frame().show(ui, |ui| {
                        ui.set_width(w1 - 32.0);
                        self.enemy_card(ui);
                    });
                });
                ui.vertical(|ui| {
                    ui.set_width(w2);
                    ui.spacing_mut().item_spacing.x = 8.0;
                    card_frame().show(ui, |ui| {
                        ui.set_width(w2 - 32.0);
                        self.my_card(ui, 340.0);
                    });
                });
            });
        } else {
            card_frame().show(ui, |ui| {
                ui.set_width(ui.available_width());
                self.enemy_card(ui);
            });
            ui.add_space(8.0);
            card_frame().show(ui, |ui| {
                ui.set_width(ui.available_width());
                let w = (ui.available_width() * 0.8).min(270.0);
                self.my_card(ui, w);
            });
        }
    }

    fn live(&self) -> bool {
        self.s.phase == Phase::Game && self.s.my_turn && !self.s.waiting
    }

    fn enemy_card(&mut self, ui: &mut Ui) {
        let live = self.live() && self.s.mode != Mode::Shield;
        let gw = 460.0f32.min(ui.available_width());
        centered(ui, gw, |ui| {
            // Libellé à gauche, flotte adverse à droite (sur deux lignes si la place manque).
            let name = self.s.opp.as_ref().map_or("?".to_string(), |o| o.name.clone());
            let title = self.tf("enemyGrid", &[("name", name)]);
            let tags: Vec<(String, bool)> = (0..5).map(|i| (self.ship_name(i), self.s.enemy_sunk.contains(&i))).collect();
            let title_w = ui.painter().layout_no_wrap(title.clone(), semi(13.0), MUTED).size().x;
            let tags_w: f32 = tags.iter().map(|(t, _)| ui.painter().layout_no_wrap(t.clone(), reg(11.0), TEXT).size().x + 18.0).sum();
            let one_line = title_w + 12.0 + tags_w <= ui.available_width();
            let draw_tags = |ui: &mut Ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    for (t, down) in tags.iter().rev() {
                        let g = ui.painter().layout_no_wrap(t.clone(), reg(11.0), if *down { TEXT.gamma_multiply(0.45) } else { TEXT });
                        let (r, _) = ui.allocate_exact_size(g.size() + vec2(14.0, 4.0), Sense::hover());
                        ui.painter().rect_filled(r, 6.0, BTN.gamma_multiply(if *down { 0.45 } else { 1.0 }));
                        let tp = r.center() - g.size() / 2.0;
                        let gw = g.size().x;
                        ui.painter().galley(tp, g, TEXT);
                        if *down {
                            ui.painter().hline(tp.x..=tp.x + gw, r.center().y, sk(1.0, TEXT.gamma_multiply(0.45)));
                        }
                    }
                });
            };
            if one_line {
                ui.horizontal(|ui| {
                    label(ui, &title, semi(13.0), MUTED);
                    draw_tags(ui);
                });
            } else {
                label(ui, &title, semi(13.0), MUTED);
                ui.horizontal(|ui| draw_tags(ui));
            }
        });
        ui.add_space(4.0);
        let mut looks = [Look::default(); N * N];
        for k in 0..N * N {
            let v = self.s.enemy[k];
            let tg = self.targetable(k);
            let l = &mut looks[k];
            match v {
                None => {}
                Some(Mark::Miss) => {
                    l.sym = "•";
                    l.sym_col = MISS;
                }
                Some(Mark::Hit) => {
                    l.fill = HIT;
                    l.sym = "✕";
                    l.sym_col = Color32::WHITE;
                }
                Some(Mark::Sunk) => {
                    l.fill = SUNK;
                    l.sym = "✕";
                    l.sym_col = rgb(0xfe, 0xca, 0xca);
                }
                Some(Mark::Blocked) => {
                    l.fill = rgb(0x5a, 0x4a, 0x1c);
                    l.sym = SHIELD_SYM;
                    l.sym_col = YELLOW;
                }
                Some(Mark::ScanOcc) => {
                    l.fill = rgb(0x2c, 0x4f, 0x78);
                    l.sym = "?";
                    l.sym_col = YELLOW;
                }
                Some(Mark::ScanEmpty) => {
                    l.sym = "·";
                    l.sym_col = MISS;
                }
            }
            if live && tg {
                l.fill = if v == Some(Mark::ScanOcc) { rgb(0x3c, 0x6a, 0x9e) } else { WATER_HI };
                l.hover = Some(WATER_HOVER);
            }
        }
        let (resp, g) = draw_grid(ui, 460.0, &looks, false, None, Sense::click());
        if resp.clicked() {
            if let Some((r, c)) = resp.interact_pointer_pos().and_then(|p| g.cell_at(p)) {
                self.on_enemy_tap(r, c);
            }
        }
    }

    fn my_card(&mut self, ui: &mut Ui, gw: f32) {
        let pick = self.live() && self.s.mode == Mode::Shield;
        let gw = gw.min(ui.available_width());
        centered(ui, gw, |ui| {
            label(ui, self.t("myFleet"), semi(13.0), MUTED);
        });
        ui.add_space(2.0);
        let occ = self.occupancy();
        let mut looks = [Look::default(); N * N];
        for k in 0..N * N {
            let i = occ[k];
            let v = self.s.incoming[k];
            let l = &mut looks[k];
            let sunk = i >= 0 && self.is_sunk(i as usize);
            if i >= 0 {
                l.fill = if sunk { SUNK } else { SHIP };
                if sunk {
                    l.sym_col = rgb(0xfe, 0xca, 0xca);
                }
                if self.s.ships[i as usize].shield == Shield::Active {
                    l.ring = Some(YELLOW);
                }
                if pick {
                    l.hover = Some(rgb(0x94, 0xa8, 0xc0));
                }
            }
            match v {
                Some(Mark::Hit) => {
                    l.sym = "✕";
                    if !sunk {
                        l.fill = HIT;
                        l.sym_col = Color32::WHITE;
                    }
                }
                Some(Mark::Miss) => {
                    l.sym = "•";
                    l.sym_col = MISS;
                }
                Some(Mark::Blocked) => {
                    l.fill = rgb(0x5a, 0x4a, 0x1c);
                    l.sym = SHIELD_SYM;
                    l.sym_col = YELLOW;
                }
                _ => {}
            }
            l.last = self.s.last_in.contains(&k);
        }
        let (resp, g) = draw_grid(ui, gw, &looks, true, pick.then_some(YELLOW), Sense::click());
        if resp.clicked() {
            if let Some((r, c)) = resp.interact_pointer_pos().and_then(|p| g.cell_at(p)) {
                self.on_my_tap(r, c);
            }
        }
        ui.add_space(10.0);
        // Modes de tir et améliorations
        let live = self.live();
        let modes = [
            (Mode::Shot, ic::CROSSHAIR, "modeShot", 0),
            (Mode::Radar, ic::WIFI_HIGH, "modeRadar", self.s.inv.radar),
            (Mode::Cross, ic::PLUS, "modeCross", self.s.inv.cross),
            (Mode::Shield, ic::SHIELD, "modeShield", self.s.inv.shield),
        ];
        let aw = ui.available_width();
        ui.horizontal(|ui| {
            let bw = (aw - 24.0) / 4.0;
            for (m, icon, key, n) in modes {
                let enabled = live && (m == Mode::Shot || n > 0);
                let on = live && self.s.mode == m;
                let (r, resp) = ui.allocate_exact_size(vec2(bw, 56.0), if enabled { Sense::click() } else { Sense::hover() });
                let a = if enabled { 1.0 } else { 0.35 };
                let p = ui.painter();
                let fill = if enabled && resp.hovered() { BTN_HI } else { BTN };
                p.rect(r, 10.0, fill.gamma_multiply(a), if on { sk(2.0, YELLOW) } else { egui::Stroke::NONE });
                p.text(r.center() - vec2(0.0, 9.0), Align2::CENTER_CENTER, icon, reg(17.0), TEXT.gamma_multiply(a));
                p.text(r.center() + vec2(0.0, 12.0), Align2::CENTER_CENTER, self.t(key), semi(12.0), TEXT.gamma_multiply(a));
                if m != Mode::Shot && n > 0 {
                    p.text(r.right_top() + vec2(-8.0, 5.0), Align2::RIGHT_TOP, n.to_string(), num(11.0), YELLOW.gamma_multiply(a));
                }
                if enabled && resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if resp.clicked() {
                    self.set_mode(m);
                }
            }
        });
        // Aide et journal
        let (hint, col) = if let Some((t, _)) = &self.flash {
            (t.clone(), ERR)
        } else if self.s.phase == Phase::Game && self.s.opp_away {
            (self.t("oppAway"), MUTED)
        } else if live {
            let k = match self.s.mode {
                Mode::Shot => "hintShot",
                Mode::Radar => "hintRadar",
                Mode::Cross => "hintCross",
                Mode::Shield => "hintShield",
            };
            (self.t(k), MUTED)
        } else {
            (String::new(), MUTED)
        };
        ui.add_space(6.0);
        ui.vertical_centered(|ui| {
            wrapped(ui, if hint.is_empty() { " ".into() } else { hint }, reg(13.0), col);
            let (log, opp) = self.s.log.clone();
            wrapped(ui, if log.is_empty() { " ".into() } else { log }, semi(14.0), if opp { MUTED } else { TEXT });
        });
    }

    /* ---------- Fenêtres ---------- */

    fn ui_modals(&mut self, ctx: &egui::Context) {
        if self.show_join {
            self.ui_join_modal(ctx);
        }
        if self.s.show_choice && self.s.phase == Phase::Game && self.s.choices > 0 {
            self.ui_choice_modal(ctx);
        }
        if self.s.end_show && self.s.end.is_some() {
            self.ui_end_modal(ctx);
        }
        if let Some(c) = self.confirm {
            let text = match c {
                Confirm::DeleteAccount => {
                    let name = self.me().map(|a| a.name.clone()).unwrap_or_default();
                    self.tf("deleteConfirm", &[("name", name)])
                }
                Confirm::Quit => self.t("quitConfirm"),
            };
            let (yes, no) = (self.t("yes"), self.t("no"));
            let mut answer = None;
            modal(ctx, "confirm", 400.0, true, |ui| {
                label(ui, ic::WARNING, reg(34.0), YELLOW);
                ui.add_space(4.0);
                wrapped(ui, text, semi(15.0), TEXT);
                ui.add_space(14.0);
                let aw = ui.available_width();
                ui.horizontal(|ui| {
                    let w = (aw - 10.0) / 2.0;
                    ui.spacing_mut().item_spacing.x = 10.0;
                    if Btn::new(&no).width(w).show(ui).clicked() {
                        answer = Some(false);
                    }
                    if Btn::new(&yes).style(Bs::Danger).width(w).show(ui).clicked() {
                        answer = Some(true);
                    }
                });
            });
            match answer {
                Some(true) => {
                    self.confirm = None;
                    match c {
                        Confirm::DeleteAccount => self.delete_account(),
                        Confirm::Quit => self.quit_game(),
                    }
                }
                Some(false) => self.confirm = None,
                None => {}
            }
        }
    }

    fn ui_join_modal(&mut self, ctx: &egui::Context) {
        let mut join = false;
        let mut cancel = false;
        let busy = self.s.join_busy;
        let (title, desc, jl, cl) = (self.t("mJoinT"), self.t("mJoinD"), self.t("btnJoin"), self.t("cancel"));
        let jm = self.join_msg.clone();
        let focus = std::mem::take(&mut self.focus_code);
        modal(ctx, "join", 380.0, false, |ui| {
            let (c, bg) = icon_colors("yellow");
            mode_icon(ui, ic::SIGN_IN, c, bg, 48.0);
            ui.add_space(6.0);
            label(ui, title, bold(22.0), TEXT);
            wrapped(ui, desc, reg(14.0), MUTED);
            ui.add_space(8.0);
            let te = egui::TextEdit::singleline(&mut self.code_input)
                .char_limit(6)
                .font(num(22.0))
                .horizontal_align(Align::Center)
                .margin(vec2(14.0, 12.0))
                .hint_text("CODE")
                .desired_width(f32::INFINITY);
            let r = ui.add(te);
            if focus {
                r.request_focus();
            }
            if r.changed() {
                self.code_input = self.code_input.to_uppercase();
                self.join_msg = Default::default();
            }
            if r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                join = true;
            }
            msg(ui, &jm.0, jm.1, 14.0);
            if Btn::new(&jl).style(Bs::Primary).wide().enabled(!busy).show(ui).clicked() {
                join = true;
            }
            ui.add_space(4.0);
            if Btn::new(&cl).style(Bs::Ghost).wide().show(ui).clicked() {
                cancel = true;
            }
        });
        if join && !busy {
            self.join_game();
        }
        if cancel {
            self.cancel_join();
        }
    }

    fn ui_choice_modal(&mut self, ctx: &egui::Context) {
        let title = self.t("choiceTitle");
        let sub = if self.s.choices > 1 { self.tf("choiceSubN", &[("n", self.s.choices.to_string())]) } else { self.t("choiceSub") };
        let opts = [
            (Mode::Radar, ic::WIFI_HIGH, self.t("upRadarN"), self.t("upRadarD")),
            (Mode::Cross, ic::PLUS, self.t("upCrossN"), self.t("upCrossD")),
            (Mode::Shield, ic::SHIELD, self.t("upShieldN"), self.t("upShieldD")),
        ];
        let mut picked = None;
        modal(ctx, "choice", 380.0, false, |ui| {
            label(ui, title, bold(22.0), TEXT);
            label(ui, sub, reg(14.0), MUTED);
            ui.add_space(6.0);
            for (m, icon, name, desc) in opts {
                let resp = egui::Frame::none()
                    .fill(card2())
                    .stroke(sk(1.0, border()))
                    .rounding(12.0)
                    .inner_margin(egui::Margin::symmetric(14.0, 12.0))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            label(ui, icon, reg(15.0), YELLOW);
                            label(ui, &name, bold(15.0), TEXT);
                        });
                        wrapped(ui, &desc, reg(13.0), MUTED);
                    })
                    .response;
                let resp = ui.interact(resp.rect, ui.id().with(name.as_str()), Sense::click());
                if resp.hovered() {
                    ui.painter().rect_stroke(resp.rect, 12.0, sk(1.0, SKY));
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if resp.clicked() {
                    picked = Some(m);
                }
                ui.add_space(2.0);
            }
        });
        if let Some(m) = picked {
            self.pick_upgrade(m);
        }
    }

    fn ui_end_modal(&mut self, ctx: &egui::Context) {
        let Some(e) = self.s.end.as_ref() else { return };
        let (icon, title, text) = (e.icon, e.title.clone(), e.text.clone());
        let (xp, bot_note, guest) = (e.xp, e.bot_note, e.guest);
        let rank_line = xp.filter(|x| x.2).map(|(_, lvl, _, _, _)| format!("⬆ {}", self.tf("levelUp", &[("n", lvl.to_string()), ("rank", self.rank_of(lvl).to_string())])));
        let (ng, sg, bx, xg) = (self.t("newGame"), self.t("seeGrids"), self.t("botXp"), self.t("xpGuest"));
        let lvl_lbl = xp.map(|(_, lvl, _, _, _)| self.tf("levelN", &[("n", lvl.to_string())]));
        let mut new_game = false;
        let mut see = false;
        modal(ctx, "end", 380.0, true, |ui| {
            label(ui, icon, reg(46.0), TEXT);
            label(ui, title, bold(22.0), TEXT);
            wrapped(ui, text, reg(14.0), MUTED);
            if let Some((gain, _, _, inl, size)) = xp {
                ui.add_space(4.0);
                label(ui, format!("+{gain} XP"), num(24.0), YELLOW);
                if let Some(r) = &rank_line {
                    label(ui, r, bold(15.0), GREEN);
                }
                if bot_note {
                    label(ui, &bx, reg(12.0), MUTED);
                }
                ui.add_space(4.0);
                let w = ui.available_width();
                xp_bar(ui, inl as f32 / size as f32, w);
                ui.horizontal(|ui| {
                    label(ui, "XP:", reg(11.0), MUTED);
                    label(ui, format!("{inl} / {size}"), bold(11.0), SKY);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        label(ui, lvl_lbl.clone().unwrap_or_default(), reg(11.0), MUTED);
                    });
                });
            } else if guest {
                label(ui, &xg, reg(12.0), MUTED);
            }
            ui.add_space(10.0);
            if Btn::new(&ng).style(Bs::Primary).wide().show(ui).clicked() {
                new_game = true;
            }
            ui.add_space(2.0);
            if Btn::new(&sg).style(Bs::Ghost).wide().show(ui).clicked() {
                see = true;
            }
        });
        if new_game {
            self.reset_to_menu();
        } else if see {
            self.s.end_show = false;
        }
    }
}

pub fn fmt_time(sec: u64) -> String {
    if sec < 60 {
        format!("{sec}s")
    } else if sec < 3600 {
        format!("{}m {:02}s", sec / 60, sec % 60)
    } else {
        format!("{}h {:02}m", sec / 3600, sec % 3600 / 60)
    }
}
