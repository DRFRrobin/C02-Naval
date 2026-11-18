//! Affichage de tous les écrans (même disposition que la version web).

use eframe::egui::{self, pos2, vec2, Align, Align2, Color32, Key, Layout, Rect, Sense, Ui};
use egui_phosphor::regular as ic;

use crate::accounts::{level_of, valid_photo, xp_for, Stats};
use crate::app::*;
use crate::game::*;
use crate::i18n::{self, Lang};
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
        Look { fill: pal().water, sym: "", sym_col: pal().text, ring: None, last: false, hover: None }
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
        let nn = n() as f32;
        let cell = (w - nn * gap) / (nn + 0.7);
        let lab = 0.7 * cell;
        let h = lab + gap + nn * (cell + gap) - gap;
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
        (r < n() && c < n()).then_some((r, c))
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
        p.rect_stroke(g.rect.expand(5.5), rd(6.0), sk(3.0, c));
    }
    let lf = reg(if small { 10.0 } else { 12.0 });
    let lfb = bold(if small { 10.0 } else { 12.0 });
    for c in 0..n() {
        let cr = g.cell_rect(0, c);
        p.text(pos2(cr.center().x, g.rect.top() + g.lab / 2.0), Align2::CENTER_CENTER, (c + 1).to_string(), lfb.clone(), pal().dim);
    }
    for r in 0..n() {
        let cr = g.cell_rect(r, 0);
        p.text(pos2(g.rect.left() + g.lab / 2.0, cr.center().y), Align2::CENTER_CENTER, (ROWS[r] as char).to_string(), lfb.clone(), pal().dim);
    }
    let _ = lf;
    let sym_size = (g.cell * 0.5).clamp(9.0, if small { 13.0 } else { 18.0 });
    for r in 0..n() {
        for c in 0..n() {
            let k = r * n() + c;
            let l = looks[k];
            let cr = g.cell_rect(r, c);
            let mut fill = l.fill;
            if hovered == Some((r, c)) {
                if let Some(h) = l.hover {
                    fill = h;
                }
            }
            p.rect_filled(cr, rd(4.0), fill);
            if let Some(rc) = l.ring {
                p.rect_stroke(cr.shrink(1.0), rd(4.0), sk(2.0, rc));
            }
            if l.last {
                p.rect_stroke(cr.shrink(1.0), rd(3.0), sk(2.0, pal().yellow));
            }
            if !l.sym.is_empty() {
                p.text(cr.center(), Align2::CENTER_CENTER, l.sym, bold(sym_size), l.sym_col);
            }
        }
    }
    if hovered.is_some_and(|(r, c)| looks[r * n() + c].hover.is_some()) {
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
                    .rounding(rd(18.0))
                    .inner_margin(egui::Margin::symmetric(26.0, 28.0))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.vertical_centered(|ui| {
                            let (c, bg) = icon_colors("cyan");
                            mode_icon(ui, ic::ANCHOR, c, bg, 64.0);
                            label(ui, "C02 Naval", bold(26.0), pal().text);
                            label(ui, format!("v{}", crate::update::current_version()), reg(13.0), pal().muted);
                            ui.add_space(12.0);
                            match &st {
                                UpdState::Checking => {
                                    spinner(ui);
                                    label(ui, self.t("updChecking"), reg(14.0), pal().muted);
                                }
                                UpdState::Downloading { version, got, total } => {
                                    label(ui, self.t("updTitle"), bold(20.0), pal().yellow);
                                    wrapped(ui, self.tf("updText", &[("to", version.clone()), ("from", crate::update::current_version())]), reg(14.0), pal().muted);
                                    ui.add_space(10.0);
                                    let frac = if *total > 0 { *got as f32 / *total as f32 } else { 0.0 };
                                    let w = ui.available_width();
                                    xp_bar(ui, frac, w);
                                    let pct = if *total > 0 { format!("{} %", (frac * 100.0).round()) } else { format!("{:.1} Mo", *got as f32 / 1e6) };
                                    label(ui, self.tf("updDownloading", &[("pct", pct)]), reg(13.0), pal().muted);
                                    ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
                                }
                                UpdState::Restarting => {
                                    spinner(ui);
                                    label(ui, self.t("updRestarting"), reg(14.0), pal().green);
                                }
                                UpdState::Failed { version, .. } => {
                                    label(ui, self.t("updTitle"), bold(20.0), pal().yellow);
                                    label(ui, format!("v{} → v{}", crate::update::current_version(), version), num(14.0), pal().sky);
                                    ui.add_space(6.0);
                                    wrapped(ui, self.t("updFailed"), reg(14.0), pal().err);
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
        ui.painter().rect(rect, rounding, pal().modal, sk(1.0, border2()));
        let tex = photo.filter(|p| valid_photo(p)).and_then(|p| self.photo_tex(ui.ctx(), p));
        if let Some(t) = tex {
            paint_image(ui, &t, rect, rounding);
        } else if bot {
            ui.painter().text(rect.center(), Align2::CENTER_CENTER, ic::ROBOT, reg(size * 0.5), pal().green);
        } else {
            let ch = name.trim().chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or("?".into());
            ui.painter().text(rect.center(), Align2::CENTER_CENTER, ch, bold(size * 0.42), pal().sky);
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
                .rounding(rd(18.0))
                .inner_margin(egui::Margin::symmetric(26.0, 28.0))
                .shadow(egui::Shadow { offset: vec2(0.0, 20.0), blur: 50.0, spread: 0.0, color: rgba(0, 0, 0, 0.45) })
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        let (c, bg) = icon_colors("cyan");
                        let (rect, _) = ui.allocate_exact_size(vec2(64.0, 64.0), Sense::hover());
                        ui.painter().rect(rect, rd(16.0), bg, sk(1.0, c.gamma_multiply(0.55)));
                        ui.painter().text(rect.center(), Align2::CENTER_CENTER, ic::ANCHOR, reg(30.0), c);
                        ui.add_space(4.0);
                        label(ui, "C02 Naval", bold(30.0), pal().text);
                        label(ui, self.t("tagline"), reg(14.0), pal().muted);
                    });
                    ui.add_space(14.0);
                    let ctx = ui.ctx().clone();
                    self.probe_api(&ctx);
                    if self.auth_cloud {
                        // Onglets Connexion / Créer un compte
                        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::hover());
                        ui.painter().rect(rect, rd(12.0), pal().input, sk(1.0, border()));
                        let half = (rect.width() - 12.0) / 2.0;
                        for (i, (key, signup)) in [("tabLogin", false), ("tabReg", true)].into_iter().enumerate() {
                            let r = Rect::from_min_size(rect.min + vec2(4.0 + i as f32 * (half + 4.0), 4.0), vec2(half, 38.0));
                            let resp = ui.interact(r, ui.id().with(key), Sense::click());
                            let on = self.auth_signup == signup;
                            let fill = if on { pal().primary } else if resp.hovered() { rgba(255, 255, 255, 0.04) } else { Color32::TRANSPARENT };
                            ui.painter().rect_filled(r, rd(9.0), fill);
                            ui.painter().text(r.center(), Align2::CENTER_CENTER, self.t(key), semi(14.0), if on { Color32::WHITE } else { pal().muted });
                            if resp.hovered() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                            if resp.clicked() {
                                self.auth_msg = Default::default();
                                self.auth_signup = signup;
                            }
                        }
                        ui.add_space(2.0);
                        self.ui_cloud_pane(ui, &ctx);
                        msg(ui, &self.auth_msg.0.clone(), self.auth_msg.1, 14.0);
                        label(ui, self.t("cloudNote"), reg(12.0), pal().muted);
                        ui.add_space(6.0);
                        if Btn::new(&self.t("goLocal")).style(Bs::Ghost).wide().show(ui).clicked() {
                            self.auth_cloud = false;
                            self.auth_msg = Default::default();
                        }
                    } else {
                        self.ui_local_pane(ui);
                        msg(ui, &self.auth_msg.0.clone(), self.auth_msg.1, 14.0);
                        if Btn::new(&self.t("guest")).style(Bs::Ghost).wide().show(ui).clicked() {
                            self.guest();
                        }
                        if self.api_up != Some(false) && Btn::new(&self.t("goCloud")).style(Bs::Ghost).wide().show(ui).clicked() {
                            self.auth_cloud = true;
                            self.auth_msg = Default::default();
                        }
                    }
                    ui.vertical_centered(|ui| {
                        if !self.auth_cloud {
                            label(ui, self.t("localNote"), reg(12.0), pal().muted);
                        }
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

    /// Choix du thème : « Auto » puis une pastille par thème (couleur d'accent sur fond du thème).
    fn theme_picker(&mut self, ui: &mut Ui) {
        let cur = self.settings.theme.clone();
        let lang = self.lang();
        let mut pick: Option<Option<&'static str>> = None;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
            for k in 0..=THEME_IDS.len() {
                let id = if k == 0 { None } else { Some(THEME_IDS[k - 1]) };
                let on = match id {
                    None => cur.as_deref().map_or(true, |c| !THEME_IDS.contains(&c)),
                    Some(i) => cur.as_deref() == Some(i),
                };
                let name = i18n::theme_name(lang, k);
                let (r, resp) = ui.allocate_exact_size(vec2(if k == 0 { 214.0 } else { 150.0 }, 38.0), Sense::click());
                let p = ui.painter();
                p.rect(r, rd(10.0), pal().btn, sk(if on { 2.0 } else { 1.0 }, if on { pal().yellow } else { border2() }));
                let sw = Rect::from_min_size(r.min + vec2(8.0, 9.0), vec2(20.0, 20.0));
                match id {
                    None => {
                        p.rect_filled(sw, rd(5.0), Colors::from(&palettes::OCEAN).primary);
                        p.rect_filled(Rect::from_min_max(sw.center_top(), sw.max), rd(5.0), Colors::from(&palettes::XMAS).primary);
                    }
                    Some(_) => {
                        let c = Colors::from(&palettes::ALL[k - 1]);
                        p.rect_filled(sw, rd(5.0), c.bg[0]);
                        p.rect_filled(Rect::from_min_max(sw.center(), sw.max), rd(5.0), c.primary);
                    }
                }
                let g = p.layout_no_wrap(name.to_string(), semi(13.0), pal().text);
                p.galley(pos2(sw.max.x + 8.0, r.center().y - g.size().y / 2.0), g, pal().text);
                if resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if resp.clicked() {
                    pick = Some(id);
                }
            }
        });
        if let Some(id) = pick {
            let ctx = ui.ctx().clone();
            self.set_theme(&ctx, id);
        }
    }

    fn lang_combo(&mut self, ui: &mut Ui, id: &str, w: f32) {
        let mut l = self.lang();
        egui::ComboBox::from_id_salt(id)
            .selected_text(egui::RichText::new(l.label()).font(reg(14.0)))
            .width(w)
            .show_ui(ui, |ui| {
                for x in Lang::ALL {
                    ui.selectable_value(&mut l, x, x.label());
                }
            });
        if l != self.lang() {
            self.set_lang(l);
        }
    }

    /// Compte en ligne : pseudo et mot de passe (connexion ou création).
    fn ui_cloud_pane(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        field_label(ui, &self.t("pseudo"));
        text_input(ui, &mut self.c_name, false, 16, "");
        field_label(ui, &self.t("password"));
        let r = text_input(ui, &mut self.c_pass, true, 128, "");
        let enter = r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        ui.add_space(10.0);
        let (key, icon) = if self.auth_signup { ("btnRegister", ic::USER_PLUS) } else { ("btnLogin", ic::SIGN_IN) };
        let busy = self.cloud_busy;
        if (Btn::new(&self.t(key)).icon(icon).style(Bs::Primary).wide().enabled(!busy).show(ui).clicked() || enter) && !busy {
            let signup = self.auth_signup;
            self.cloud_submit(ctx, signup);
        }
    }

    /// Sans compte : un pseudo local (le profil reste sur cet ordinateur).
    fn ui_local_pane(&mut self, ui: &mut Ui) {
        let accs: Vec<_> = self.store.sorted().into_iter().filter(|a| !a.cloud).collect();
        if !accs.is_empty() {
            field_label(ui, &self.t("recentPseudos"));
            egui::ScrollArea::vertical().id_salt("accPick").max_height(200.0).show(ui, |ui| {
                for a in &accs {
                    let resp = egui::Frame::none()
                        .fill(card2())
                        .stroke(sk(1.0, border()))
                        .rounding(rd(12.0))
                        .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                self.mini_av(ui, a.photo.as_deref(), &a.name, 36.0, false);
                                ui.add_space(4.0);
                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = 0.0;
                                    label(ui, &a.name, bold(15.0), pal().text);
                                    let l = level_of(a.xp);
                                    label(ui, format!("{} · {} XP", self.tf("levelN", &[("n", l.to_string())]), a.xp), reg(12.0), pal().muted);
                                });
                            });
                        })
                        .response;
                    let resp = ui.interact(resp.rect, ui.id().with(&a.id), Sense::click());
                    if resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if resp.clicked() {
                        self.do_play(&a.name);
                    }
                }
            });
        }
        field_label(ui, &self.t("pseudo"));
        let r = text_input(ui, &mut self.su_name, false, 16, "");
        let enter = r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        ui.add_space(10.0);
        if Btn::new(&self.t("btnPlay")).icon(ic::PLAY).style(Bs::Primary).wide().show(ui).clicked() || enter {
            let n = self.su_name.clone();
            self.do_play(&n);
        }
    }

    /* ---------- Tableau de bord ---------- */

    fn ui_dash(&mut self, ui: &mut Ui) {
        self.ui_topbar(ui);
        match self.pane {
            Pane::Play => self.ui_play(ui),
            Pane::Stats => self.ui_stats(ui),
            Pane::Rules => self.ui_rules(ui),
            Pane::Settings => self.ui_settings(ui),
        }
    }

    fn ui_topbar(&mut self, ui: &mut Ui) {
        let tiny = ui.available_width() < 520.0;
        // Largeur des 4 boutons de navigation : s'ils ne tiennent pas à côté du joueur, on passe sur 2 lignes.
        let nav_w: f32 = ["navPlay", "navStats", "navRules", "navSettings", "navLogout"]
            .iter()
            .map(|k| ui.painter().layout_no_wrap(format!("{}  {}", ic::GEAR, self.t(k)), semi(15.0), pal().text).size().x + 32.0 + 8.0)
            .sum();
        let inner = ui.available_width() - 36.0;
        let who_max = inner - 64.0 - 16.0 - nav_w - 8.0;
        let narrow = who_max < 170.0;
        let acc = self.me().cloned();
        egui::Frame::none()
            .fill(card())
            .stroke(sk(1.0, border()))
            .rounding(rd(16.0))
            .inner_margin(if narrow { egui::Margin::same(12.0) } else { egui::Margin::symmetric(18.0, 14.0) })
            .shadow(egui::Shadow { offset: vec2(0.0, 10.0), blur: 30.0, spread: 0.0, color: rgba(0, 0, 0, 0.35) })
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                let who = |this: &mut App, ui: &mut Ui, maxw: f32| {
                    // Avatar (clic = changer la photo)
                    let (rect, resp) = ui.allocate_exact_size(vec2(64.0, 64.0), if acc.is_some() { Sense::click() } else { Sense::hover() });
                    let photo = acc.as_ref().and_then(|a| a.photo.clone()).filter(|p| valid_photo(p));
                    ui.painter().rect_filled(rect, rd(14.0), rgba(0, 0, 0, 0.2));
                    match photo.and_then(|p| this.photo_tex(ui.ctx(), &p)) {
                        Some(t) => {
                            paint_image(ui, &t, rect.shrink(2.0), 12.0);
                            ui.painter().rect_stroke(rect, rd(14.0), sk(2.0, border2()));
                        }
                        None => {
                            if acc.is_some() {
                                dashed_rect(ui, rect.shrink(1.0), sk(2.0, pal().sky));
                            } else {
                                ui.painter().rect_stroke(rect, rd(14.0), sk(2.0, pal().sky));
                            }
                            let i = if acc.is_some() { ic::CAMERA } else { ic::USER };
                            ui.painter().text(rect.center(), Align2::CENTER_CENTER, i, reg(20.0), pal().dim);
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
                            label(ui, name, bold(18.0), pal().text);
                            if let Some(a) = &acc {
                                let l = level_of(a.xp);
                                let txt = this.tf("badge", &[("n", l.to_string())]);
                                let g = ui.painter().layout_no_wrap(txt, bold(12.0), pal().badge);
                                let (r, _) = ui.allocate_exact_size(g.size() + vec2(14.0, 4.0), Sense::hover());
                                ui.painter().rect(r, rd(6.0), rgba(14, 116, 144, 0.3), sk(1.0, rgba(34, 211, 238, 0.45)));
                                ui.painter().galley(r.center() - g.size() / 2.0, g, pal().text);
                            }
                        });
                        if let Some(a) = &acc {
                            let l = level_of(a.xp);
                            let (lo, hi) = (xp_for(l), xp_for(l + 1));
                            ui.add_space(5.0);
                            xp_bar(ui, (a.xp - lo) as f32 / (hi - lo) as f32, w);
                            ui.horizontal(|ui| {
                                label(ui, "XP:", reg(11.0), pal().muted);
                                label(ui, format!("{} / {}", a.xp - lo, hi - lo), bold(11.0), pal().sky);
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    label(ui, format!("{} · {}", this.tf("levelN", &[("n", l.to_string())]), this.rank_of(l)), reg(11.0), pal().muted);
                                });
                            });
                        }
                    });
                };
                let nav = |this: &mut App, ui: &mut Ui, w: Option<f32>| {
                    let items = [
                        (Some(Pane::Play), ic::CROSSHAIR, "navPlay"),
                        (Some(Pane::Stats), ic::CHART_PIE, "navStats"),
                        (Some(Pane::Rules), ic::RULER, "navRules"),
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
                        let w = (aw - 4.0 * 8.0) / 5.0;
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
                                (Some(Pane::Rules), ic::RULER, "navRules"),
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
                            .rounding(rd(16.0))
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
                                            label(ui, self.t(tk), bold(17.0), pal().text);
                                            wrapped(ui, self.t(dk), reg(14.0), pal().muted);
                                        });
                                    });
                                } else {
                                    ui.set_min_height(150.0);
                                    mode_icon(ui, icon, c, bg, 48.0);
                                    ui.add_space(18.0);
                                    label(ui, self.t(tk), bold(20.0), pal().text);
                                    wrapped(ui, self.t(dk), reg(14.0), pal().muted);
                                }
                                });
                            })
                            .response;
                        let resp = ui.interact(resp.rect, ui.id().with(tk), Sense::click());
                        if resp.hovered() {
                            ui.painter().rect_stroke(resp.rect, rd(16.0), sk(1.5, rgba(56, 189, 248, 0.6)));
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
            // résumé des règles et lien pour les modifier
            let cfg = self.settings.rules.clone();
            let sum = if cfg.is_std() { self.t("rSumStd") } else { i18n::rules_custom(self.lang(), cfg.size, cfg.fleet.len()) };
            ui.vertical_centered(|ui| {
                ui.horizontal(|ui| {
                    let edit = self.t("rEdit");
                    let w = ui.painter().layout_no_wrap(format!("{sum} — {edit}"), reg(13.5), pal().muted).size().x;
                    ui.add_space(((ui.available_width() - w) / 2.0).max(0.0));
                    label(ui, format!("{sum} —"), reg(13.5), pal().muted);
                    let l = ui.add(egui::Label::new(egui::RichText::new(edit).font(reg(13.5)).color(pal().sky).underline()).sense(Sense::click()));
                    if l.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if l.clicked() {
                        self.pane = Pane::Rules;
                    }
                });
            });
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
            label(ui, icon, reg(22.0), pal().cyan);
            label(ui, self.t(key), bold(if ui.available_width() < 600.0 { 21.0 } else { 26.0 }), pal().text);
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
                    p.rect(rect, rd(12.0), card2(), sk(1.0, border()));
                    let cx = rect.center().x;
                    p.text(pos2(cx, rect.top() + 30.0), Align2::CENTER_CENTER, *icon, reg(20.0), *ic_col);
                    p.text(pos2(cx, rect.top() + 62.0), Align2::CENTER_CENTER, val, num(20.0), *val_col);
                    let g = p.layout(lab.clone(), reg(13.0), pal().muted, w - 12.0);
                    p.galley(pos2(cx - g.size().x / 2.0, rect.top() + 82.0), g, pal().muted);
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
                    (ic::CLOCK, pal().sky, fmt_time(s.time_played), self.t("stTime"), pal().text),
                    (ic::GAME_CONTROLLER, pal().purple, s.games.to_string(), self.t("stGames"), pal().text),
                    (ic::TROPHY, pal().green, s.wins.to_string(), self.t("stWins"), pal().green),
                    (ic::SKULL, pal().red, s.losses.to_string(), self.t("stLosses"), pal().red),
                    (ic::PERCENT, pal().sky, pct(s.wins, s.games), self.t("stWinPct"), pal().sky),
                    (ic::PERCENT, pal().yellow, pct(s.losses, s.games), self.t("stLossPct"), pal().yellow),
                    (ic::TREND_UP, pal().green, rate(s.wins, s.games), self.t("stWinRate"), pal().green),
                    (ic::TREND_DOWN, pal().red, rate(s.losses, s.games), self.t("stLossRate"), pal().red),
                ];
                self.tile_grid(ui, &career);
                ui.add_space(18.0);
                label(ui, self.t("combatTitle").to_uppercase(), bold(15.0), pal().muted);
                ui.add_space(8.0);
                let fast = s.fastest_win.map_or("—".to_string(), |n| self.tf("shotsN", &[("n", n.to_string())]));
                let combat = [
                    (ic::CROSSHAIR, pal().sky, s.shots.to_string(), self.t("stShots"), pal().text),
                    (ic::TARGET, pal().green, pct(s.hits, s.shots), self.t("stAcc"), pal().green),
                    (ic::BOAT, pal().red, s.sunk.to_string(), self.t("stSunk"), pal().text),
                    (ic::MAGIC_WAND, pal().purple, s.upgrades.to_string(), self.t("stUpg"), pal().text),
                    (ic::FIRE, pal().yellow, s.streak.to_string(), self.t("stStreak"), pal().yellow),
                    (ic::MEDAL, pal().yellow, s.best_streak.to_string(), self.t("stBest"), pal().yellow),
                    (ic::LIGHTNING, pal().sky, fast, self.t("stFast"), pal().text),
                    (ic::PLUGS, pal().red, s.interrupted.to_string(), self.t("stInt"), pal().text),
                ];
                self.tile_grid(ui, &combat);
                if acc.is_none() {
                    msg(ui, &self.t("guestStats"), MsgKind::Info, 14.0);
                }
            });
        });
    }

    /// Pastille « − valeur + » ; renvoie -1, 0 ou +1.
    fn stepper(&self, ui: &mut Ui, val: &str, minus: bool, plus: bool) -> i32 {
        let mut d = 0;
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            if Btn::new("+").small().width(34.0).enabled(plus).show(ui).clicked() {
                d = 1;
            }
            let (r, _) = ui.allocate_exact_size(vec2(86.0, 34.0), Sense::hover());
            ui.painter().text(r.center(), Align2::CENTER_CENTER, val, bold(14.0), pal().text);
            if Btn::new("−").small().width(34.0).enabled(minus).show(ui).clicked() {
                d = -1;
            }
        });
        d
    }

    fn ui_rules(&mut self, ui: &mut Ui) {
        ui.add_space(24.0);
        let lang = self.lang();
        let mut r = self.settings.rules.clone();
        let before = r.clone();
        centered(ui, 770.0, |ui| {
            panel_frame().show(ui, |ui| {
                ui.set_width(ui.available_width());
                self.panel_title(ui, ic::RULER, "rulesTitle");
                wrapped(ui, self.t("rulesIntro"), reg(13.5), pal().muted);
                ui.add_space(8.0);
                // Taille de la grille
                sub_frame().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        label(ui, ic::TABLE, reg(20.0), pal().sky);
                        ui.add_space(6.0);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            label(ui, self.t("rGridT"), bold(16.0), pal().text);
                            wrapped(ui, self.t("rGridD"), reg(13.0), pal().muted);
                        });
                        let d = self.stepper(ui, &format!("{0} × {0}", r.size), r.size > MIN_SIZE, r.size < MAX_SIZE);
                        if d != 0 {
                            r.size = (r.size as i32 + d) as usize;
                            r.fleet.sort_unstable_by(|a, b| b.cmp(a));
                            while r.fleet.len() > 1 && r.total_cells() > max_cells(r.size) {
                                r.fleet.remove(0); // flotte trop grande : on retire les plus gros bateaux
                            }
                        }
                    });
                });
                ui.add_space(8.0);
                // Flotte
                sub_frame().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        label(ui, ic::BOAT, reg(18.0), pal().yellow);
                        label(ui, self.t("rFleetT"), bold(16.0), pal().text);
                    });
                    wrapped(ui, self.t("rFleetD"), reg(13.0), pal().muted);
                    ui.add_space(4.0);
                    let tot = r.total_cells();
                    let cap = max_cells(r.size);
                    for len in (1..=MAX_LEN).rev() {
                        let cnt = r.fleet.iter().filter(|&&l| l == len).count();
                        ui.horizontal(|ui| {
                            let name = i18n::ship_name(lang, &[len], 0);
                            ui.allocate_ui(vec2(150.0, 34.0), |ui| {
                                ui.set_width(150.0);
                                label(ui, name, semi(13.5), pal().text);
                            });
                            let (br, _) = ui.allocate_exact_size(vec2(len as f32 * 13.0, 12.0), Sense::hover());
                            for b in 0..len {
                                let c = Rect::from_min_size(br.min + vec2(b as f32 * 13.0, 0.0), vec2(11.0, 11.0));
                                ui.painter().rect_filled(c, rd(2.0), pal().ship);
                            }
                            let d = self.stepper(ui, &cnt.to_string(), cnt > 0 && r.fleet.len() > 1, r.fleet.len() < MAX_SHIPS && tot + len <= cap);
                            if d > 0 {
                                r.fleet.push(len);
                            } else if d < 0 {
                                if let Some(i) = r.fleet.iter().position(|&l| l == len) {
                                    r.fleet.remove(i);
                                }
                            }
                        });
                    }
                    let pct = (tot as f32 / (r.size * r.size) as f32 * 100.0).round() as usize;
                    label(ui, i18n::fleet_total(lang, r.fleet.len(), tot, pct), reg(13.0), pal().sky);
                });
                ui.add_space(8.0);
                // Améliorations
                sub_frame().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        label(ui, ic::MAGIC_WAND, reg(18.0), pal().purple);
                        label(ui, self.t("rUpT"), bold(16.0), pal().text);
                    });
                    wrapped(ui, self.t("rUpD"), reg(13.0), pal().muted);
                    let items = [
                        (ic::WIFI_HIGH, "upRadarN", "upRadarD", 0),
                        (ic::PLUS, "upCrossN", "upCrossD", 1),
                        (ic::SHIELD, "upShieldN", "upShieldD", 2),
                    ];
                    for (icon, nk, dk, which) in items {
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            label(ui, icon, reg(18.0), pal().yellow);
                            ui.add_space(6.0);
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 2.0;
                                label(ui, self.t(nk), bold(14.5), pal().text);
                                wrapped(ui, self.t(dk), reg(12.5), pal().muted);
                            });
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let on = match which {
                                    0 => r.radar,
                                    1 => r.cross,
                                    _ => r.shield,
                                };
                                if self.on_off(ui, on) {
                                    match which {
                                        0 => r.radar = !on,
                                        1 => r.cross = !on,
                                        _ => r.shield = !on,
                                    }
                                }
                            });
                        });
                    }
                });
                ui.add_space(8.0);
                // Chronomètre
                sub_frame().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        label(ui, ic::TIMER, reg(20.0), pal().cyan);
                        ui.add_space(6.0);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            label(ui, self.t("rTimerT"), bold(16.0), pal().text);
                            wrapped(ui, self.t("rTimerD"), reg(13.0), pal().muted);
                        });
                        let idx = TIMER_CHOICES.iter().position(|&t| t == r.timer).unwrap_or(0);
                        let txt = if r.timer == 0 { self.t("timerOff") } else { self.tf("timerN", &[("n", r.timer.to_string())]) };
                        let d = self.stepper(ui, &txt, idx > 0, idx + 1 < TIMER_CHOICES.len());
                        if d != 0 {
                            r.timer = TIMER_CHOICES[(idx as i32 + d) as usize];
                        }
                    });
                });
                ui.add_space(8.0);
                                sub_frame().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        label(ui, ic::ROBOT, reg(20.0), pal().green);
                        ui.add_space(6.0);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            label(ui, self.t("rBotT"), bold(16.0), pal().text);
                            wrapped(ui, self.t("rBotD"), reg(13.0), pal().muted);
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let on = self.settings.bot;
                            let (r, resp) = ui.allocate_exact_size(vec2(58.0, 36.0), Sense::click());
                            ui.painter().rect_filled(r, rd(9.0), if on { pal().ok } else { rgb(0x33, 0x41, 0x55) });
                            let g = ui.painter().layout_no_wrap(if on { "ON" } else { "OFF" }.into(), bold(13.0), if on { Color32::WHITE } else { rgb(0xcb, 0xd5, 0xe1) });
                            ui.painter().galley(r.center() - g.size() / 2.0, g, pal().text);
                            if resp.hovered() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                            if resp.clicked() {
                                self.set_bot_enabled(!on);
                            }
                        });
                    });
                    ui.add_space(6.0);
                    label(ui, self.t("rSkillT"), semi(14.0), pal().text);
                    let mut sk_sel = self.settings.skill.min(2);
                    let names = [self.t("skill0"), self.t("skill1"), self.t("skill2")];
                    ui.add_enabled_ui(self.settings.bot, |ui| {
                        egui::ComboBox::from_id_salt("skillSel")
                            .selected_text(egui::RichText::new(names[sk_sel as usize].clone()).font(reg(14.0)))
                            .width(ui.available_width() - 8.0)
                            .show_ui(ui, |ui| {
                                for (i, n) in names.iter().enumerate() {
                                    ui.selectable_value(&mut sk_sel, i as u8, n.as_str());
                                }
                            });
                    });
                    if sk_sel != self.settings.skill {
                        self.set_skill(sk_sel);
                    }
                });

                ui.add_space(12.0);
                if Btn::new(&self.t("rReset")).icon(ic::ARROW_COUNTER_CLOCKWISE).show(ui).clicked() {
                    self.play("click");
                    r = RulesCfg::default();
                }
            });
        });
        if r != before {
            self.set_rules(r);
        }
    }

    /// Bouton ON / OFF ; renvoie `true` s'il vient d'être cliqué.
    fn on_off(&self, ui: &mut Ui, on: bool) -> bool {
        let (r, resp) = ui.allocate_exact_size(vec2(58.0, 36.0), Sense::click());
        ui.painter().rect_filled(r, rd(9.0), if on { pal().ok } else { rgb(0x33, 0x41, 0x55) });
        let g = ui.painter().layout_no_wrap(self.t(if on { "rOn" } else { "rOff" }), bold(13.0), if on { Color32::WHITE } else { rgb(0xcb, 0xd5, 0xe1) });
        ui.painter().galley(r.center() - g.size() / 2.0, g, pal().text);
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        resp.clicked()
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
                        label(ui, ic::SPEAKER_HIGH, reg(20.0), pal().sky);
                        ui.add_space(6.0);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            label(ui, self.t("sndT"), bold(16.0), pal().text);
                            wrapped(ui, self.t("sndD"), reg(13.0), pal().muted);
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let on = self.settings.sound;
                            let g = ui.painter().layout_no_wrap(if on { "ON" } else { "OFF" }.into(), bold(13.0), if on { Color32::WHITE } else { rgb(0xcb, 0xd5, 0xe1) });
                            let (r, resp) = ui.allocate_exact_size(vec2(58.0, 36.0), Sense::click());
                            ui.painter().rect_filled(r, rd(9.0), if on { pal().ok } else { rgb(0x33, 0x41, 0x55) });
                            ui.painter().galley(r.center() - g.size() / 2.0, g, pal().text);
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
                    label(ui, self.t("langT"), bold(16.0), pal().text);
                    label(ui, self.t("langD"), reg(13.0), pal().muted);
                    ui.add_space(4.0);
                    let w = ui.available_width();
                    self.lang_combo(ui, "langSel", w - 8.0);
                });
                ui.add_space(8.0);
                sub_frame().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    label(ui, self.t("advT"), bold(16.0), pal().text);
                    wrapped(ui, self.t("advD"), reg(13.0), pal().muted);
                    ui.add_space(6.0);
                    let (h1, h2, h3) = (self.t("turnUrlPh"), self.t("turnUserPh"), self.t("turnCredPh"));
                    let mut s = self.settings.clone();
                    let mut changed = false;
                    changed |= theme::text_input(ui, &mut s.turn_url, false, 200, &h1).lost_focus();
                    changed |= theme::text_input(ui, &mut s.turn_user, false, 100, &h2).lost_focus();
                    changed |= theme::text_input(ui, &mut s.turn_cred, true, 200, &h3).lost_focus();
                    self.settings.turn_url = s.turn_url;
                    self.settings.turn_user = s.turn_user;
                    self.settings.turn_cred = s.turn_cred;
                    if changed {
                        self.save_turn();
                    }
                });
                ui.add_space(8.0);
                sub_frame().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    label(ui, self.t("themeT"), bold(16.0), pal().text);
                    wrapped(ui, self.t("themeD"), reg(13.0), pal().muted);
                    ui.add_space(6.0);
                    self.theme_picker(ui);
                });
                if let Some(a) = acc {
                    ui.add_space(8.0);
                    sub_frame().show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            label(ui, ic::IMAGE, reg(16.0), pal().yellow);
                            label(ui, self.t("photoT"), bold(16.0), pal().text);
                        });
                        wrapped(ui, self.t("photoD"), reg(13.0), pal().muted);
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
                        ui.add_space(8.0);
                        label(ui, self.t("emojiPick"), semi(14.0), pal().text);
                        wrapped(ui, self.t("emojiNote"), reg(12.5), pal().muted);
                        ui.add_space(4.0);
                        let mut chosen: Option<&str> = None;
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                            for (e, _) in crate::emoji::EMOJIS.iter() {
                                let on = a.photo.as_deref() == Some(format!("emoji:{e}").as_str());
                                let (r, resp) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::click());
                                ui.painter().rect(r, rd(8.0), pal().btn, sk(if on { 2.0 } else { 1.0 }, if on { pal().yellow } else { border2() }));
                                if let Some(t) = self.photo_tex(ui.ctx(), &format!("emoji:{e}")) {
                                    paint_image(ui, &t, r.shrink(5.0), 0.0);
                                }
                                if resp.hovered() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                }
                                if resp.clicked() {
                                    chosen = Some(e);
                                }
                            }
                        });
                        if let Some(e) = chosen {
                            self.set_emoji(e);
                        }
                    });
                    ui.add_space(8.0);
                    if a.cloud {
                        // compte en ligne : le pseudo est unique et fixe ; mot de passe modifiable
                        sub_frame().show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                label(ui, ic::CLOUD, reg(16.0), pal().sky);
                                label(ui, self.t("cloudAcct"), bold(16.0), pal().text);
                            });
                            ui.add_space(6.0);
                            label(ui, self.t("pwT"), semi(14.5), pal().text);
                            wrapped(ui, self.t("pwD"), reg(13.0), pal().muted);
                            ui.add_space(4.0);
                            let (h1, h2) = (self.t("pwOldPh"), self.t("pwNewPh"));
                            text_input(ui, &mut self.pw_old, true, 128, &h1);
                            let r = text_input(ui, &mut self.pw_new, true, 128, &h2);
                            let enter = r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                            ui.add_space(8.0);
                            if Btn::new(&self.t("pwBtn")).style(Bs::Grad).wide().show(ui).clicked() || enter {
                                let ctx = ui.ctx().clone();
                                self.change_password(&ctx);
                            }
                            msg(ui, &self.pw_msg.0.clone(), self.pw_msg.1, 14.0);
                        });
                    } else {
                        sub_frame().show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                label(ui, ic::USER_GEAR, reg(16.0), pal().yellow);
                                wrapped(ui, self.t("editT"), bold(16.0), pal().text);
                            });
                            field_label(ui, &self.t("newPseudo"));
                            let r = text_input(ui, &mut self.edit_name, false, 16, "");
                            if r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                                self.save_profile();
                            }
                            ui.add_space(10.0);
                            if Btn::new(&self.t("save")).style(Bs::Grad).wide().show(ui).clicked() {
                                self.save_profile();
                            }
                            msg(ui, &self.set_msg.0.clone(), self.set_msg.1, 14.0);
                        });
                    }
                    ui.add_space(8.0);
                    sub_frame().show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let aw = ui.available_width();
                        ui.horizontal(|ui| {
                            label(ui, ic::TRASH, reg(20.0), pal().red);
                            ui.add_space(6.0);
                            let w = (aw - 110.0).max(100.0);
                            ui.vertical(|ui| {
                                ui.set_width(w);
                                ui.spacing_mut().item_spacing.y = 2.0;
                                label(ui, self.t("deleteT"), bold(16.0), pal().text);
                                wrapped(ui, self.t("deleteD"), reg(13.0), pal().muted);
                            });
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if Btn::new(&self.t("deleteBtn")).style(Bs::Danger).small().show(ui).clicked() {
                                    self.confirm = Some(Confirm::DeleteAccount);
                                }
                            });
                        });
                        if a.cloud {
                            ui.add_space(4.0);
                            let ph = self.t("delPrompt");
                            wrapped(ui, ph, reg(12.5), pal().muted);
                            text_input(ui, &mut self.pw_del, true, 128, "");
                        }
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
                .rounding(rd(18.0))
                .inner_margin(egui::Margin::symmetric(24.0, 28.0))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.vertical_centered(|ui| {
                        let (c, bg) = icon_colors(if host { "purple" } else { "cyan" });
                        mode_icon(ui, if host { ic::BROADCAST } else { ic::SHUFFLE }, c, bg, 48.0);
                        ui.add_space(4.0);
                        label(ui, self.t(if host { "lobbyHostT" } else { "lobbyRandomT" }), bold(24.0), pal().text);
                        wrapped(ui, self.t(if host { "lobbyHostD" } else { "lobbyRandomD" }), reg(15.0), pal().muted);
                        if host {
                            ui.add_space(10.0);
                            let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 76.0), Sense::hover());
                            ui.painter().rect(r, rd(12.0), pal().input, sk(1.0, border2()));
                            let code = if self.s.code.is_empty() { "······".to_string() } else { self.s.code.clone() };
                            let n = code.chars().count() as f32;
                            let step = 38.0 * 1.0;
                            let x0 = r.center().x - step * (n - 1.0) / 2.0;
                            for (i, ch) in code.chars().enumerate() {
                                ui.painter().text(pos2(x0 + i as f32 * step, r.center().y), Align2::CENTER_CENTER, ch.to_string(), num(36.0), pal().cyan);
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
                            label(ui, self.tf("countdown", &[("s", left.to_string())]), num(15.0), pal().yellow);
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
            .rounding(rd(14.0))
            .inner_margin(egui::Margin::symmetric(14.0, 10.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let who = |this: &mut App, ui: &mut Ui, p: &Opp| {
                        this.mini_av(ui, p.photo.as_deref(), &p.name, 30.0, p.bot);
                        label(ui, &p.name, bold(14.0), pal().text);
                        if p.level > 0 {
                            label(ui, this.tf("levelShort", &[("n", p.level.to_string())]), bold(11.0), pal().badge);
                        }
                    };
                    who(self, ui, &me);
                    if let Some(o) = &opp {
                        ui.add_space(4.0);
                        label(ui, self.t("vs").to_uppercase(), semi(12.0), pal().dim);
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
        let mut looks = vec![Look::default(); n() * n()];
        for k in 0..n() * n() {
            looks[k].hover = None;
            if occ[k] >= 0 {
                looks[k].fill = pal().ship;
            }
        }
        if let Some((cells, valid)) = &pv {
            for &(r, c) in cells {
                let l = &mut looks[r * n() + c];
                if *valid {
                    l.fill = pal().ok;
                    l.ring = Some(rgb(0xa7, 0xf3, 0xd0));
                } else {
                    l.fill = pal().hit.gamma_multiply(0.85);
                }
            }
        }
        let (resp, g) = draw_grid(ui, 460.0, &looks, false, None, Sense::click_and_drag());
        if self.s.me_ready {
            return;
        }
        if resp.hover_pos().and_then(|p| g.cell_at(p)).is_some() {
            let grab = resp.hover_pos().and_then(|p| g.cell_at(p)).is_some_and(|(r, c)| occ[r * n() + c] >= 0);
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
        label(ui, self.t("placeTitle"), bold(20.0), pal().text);
        wrapped(ui, self.t("placeHelp"), reg(13.5), pal().muted);
        let rules = self.s.rules;
        let mut info = i18n::rules_info(self.lang(), n(), lens().len(), rules.any_upgrade());
        if rules.timer > 0 {
            info += &format!(
                " {} {}",
                self.tf("placeLimit", &[("s", (PLACE_LIMIT.as_secs()).to_string())]),
                self.tf("turnLimit", &[("s", rules.timer.to_string())])
            );
        }
        wrapped(ui, info, reg(12.5), pal().sky);
        ui.add_space(6.0);
        // Liste des bateaux (boutons qui se répartissent sur la largeur).
        let w = ui.available_width();
        let gap = 6.0;
        let per_row = (((w + gap) / (120.0 + gap)).floor() as usize).max(1);
        let mut clicked = None;
        for row in (0..self.s.ships.len()).collect::<Vec<_>>().chunks(per_row) {
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
                    p.rect(r, rd(10.0), card2().gamma_multiply(a), sk(if sel { 2.0 } else { 1.0 }, if sel { pal().yellow } else { border() }));
                    p.text(r.left_top() + vec2(10.0, 8.0), Align2::LEFT_TOP, self.ship_name(i), semi(12.5), pal().text.gamma_multiply(a));
                    for b in 0..s.len {
                        let br = Rect::from_min_size(r.left_top() + vec2(10.0 + b as f32 * 12.0, 30.0), vec2(10.0, 10.0));
                        p.rect_filled(br, rd(2.0), if placed { pal().ok } else { pal().ship }.gamma_multiply(a));
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
        let (text, kind) = if self.s.reconnecting {
            (self.t("reconnecting"), MsgKind::Info)
        } else if self.s.me_ready {
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
            (self.t("stLost"), pal().btn, pal().muted)
        } else if self.s.reconnecting {
            (self.t("stReconnecting"), pal().btn, pal().yellow)
        } else if self.s.phase == Phase::Over {
            (self.t("stOver"), pal().btn, pal().muted)
        } else if self.s.waiting {
            (self.t("stFiring"), rgb(0xb4, 0x53, 0x09), Color32::WHITE)
        } else if self.s.my_turn {
            match self.turn_left() {
                Some(left) => (format!("{} · {left} s", self.t("stMyTurn")), if left <= 5 { pal().red } else { pal().ok }, Color32::WHITE),
                None => (self.t("stMyTurn"), pal().ok, Color32::WHITE),
            }
        } else {
            (self.t("stOppTurn"), pal().btn, pal().muted)
        };
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 44.0), Sense::hover());
        ui.painter().rect_filled(r, rd(12.0), fill);
        ui.painter().text(r.center(), Align2::CENTER_CENTER, text, bold(17.5), col);
        if let (Some(left), t) = (self.turn_left(), self.s.rules.timer) {
            // barre du temps restant sous le bandeau
            let frac = (self.s.turn_start.map_or(0.0, |s| s.elapsed().as_secs_f32()) / t as f32).clamp(0.0, 1.0);
            let bar = Rect::from_min_size(pos2(r.left(), r.bottom() - 4.0), vec2(r.width() * (1.0 - frac), 4.0));
            ui.painter().rect_filled(bar, 0.0, if left <= 5 { Color32::WHITE } else { pal().yellow });
        }
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
            let title_w = ui.painter().layout_no_wrap(title.clone(), semi(13.0), pal().muted).size().x;
            let tags_w: f32 = tags.iter().map(|(t, _)| ui.painter().layout_no_wrap(t.clone(), reg(11.0), pal().text).size().x + 18.0).sum();
            let one_line = title_w + 12.0 + tags_w <= ui.available_width();
            let draw_tags = |ui: &mut Ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    for (t, down) in tags.iter().rev() {
                        let g = ui.painter().layout_no_wrap(t.clone(), reg(11.0), if *down { pal().text.gamma_multiply(0.45) } else { pal().text });
                        let (r, _) = ui.allocate_exact_size(g.size() + vec2(14.0, 4.0), Sense::hover());
                        ui.painter().rect_filled(r, rd(6.0), pal().btn.gamma_multiply(if *down { 0.45 } else { 1.0 }));
                        let tp = r.center() - g.size() / 2.0;
                        let gw = g.size().x;
                        ui.painter().galley(tp, g, pal().text);
                        if *down {
                            ui.painter().hline(tp.x..=tp.x + gw, r.center().y, sk(1.0, pal().text.gamma_multiply(0.45)));
                        }
                    }
                });
            };
            if one_line {
                ui.horizontal(|ui| {
                    label(ui, &title, semi(13.0), pal().muted);
                    draw_tags(ui);
                });
            } else {
                label(ui, &title, semi(13.0), pal().muted);
                ui.horizontal(|ui| draw_tags(ui));
            }
        });
        ui.add_space(4.0);
        let mut looks = vec![Look::default(); n() * n()];
        let aimed: Vec<usize> = match self.s.aim {
            Some((r, c)) if live && matches!(self.s.mode, Mode::Radar | Mode::Cross) => self.aim_cells(self.s.mode, r, c).into_iter().map(|(r, c)| r * n() + c).collect(),
            _ => vec![],
        };
        let missed = self.opp_ship_cells();
        for k in 0..n() * n() {
            let v = self.s.enemy[k];
            let tg = self.targetable(k);
            let l = &mut looks[k];
            match v {
                None => {}
                Some(Mark::Miss) => {
                    l.sym = "•";
                    l.sym_col = pal().miss;
                }
                Some(Mark::Hit) => {
                    l.fill = pal().hit;
                    l.sym = "✕";
                    l.sym_col = Color32::WHITE;
                }
                Some(Mark::Sunk) => {
                    l.fill = pal().sunk;
                    l.sym = "✕";
                    l.sym_col = rgb(0xfe, 0xca, 0xca);
                }
                Some(Mark::Blocked) => {
                    l.fill = pal().blk;
                    l.sym = SHIELD_SYM;
                    l.sym_col = pal().yellow;
                }
                Some(Mark::ScanOcc) => {
                    l.fill = pal().scan;
                    l.sym = "?";
                    l.sym_col = pal().yellow;
                }
                Some(Mark::ScanEmpty) => {
                    l.sym = "·";
                    l.sym_col = pal().miss;
                }
            }
            if live && tg {
                l.fill = if v == Some(Mark::ScanOcc) { pal().scan_hi } else { pal().water_hi };
                l.hover = Some(pal().water_hov);
            }
            if aimed.contains(&k) {
                l.fill = pal().water_hov;
                l.ring = Some(pal().yellow);
            }
            if missed.contains(&k) && v != Some(Mark::Sunk) {
                l.ring = Some(pal().ship); // bateau adverse jamais trouvé, dévoilé en fin de partie
                if v.is_none() || v == Some(Mark::ScanEmpty) {
                    l.fill = pal().ship.gamma_multiply(0.55);
                }
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
            label(ui, self.t("myFleet"), semi(13.0), pal().muted);
        });
        ui.add_space(2.0);
        let occ = self.occupancy();
        let mut looks = vec![Look::default(); n() * n()];
        for k in 0..n() * n() {
            let i = occ[k];
            let v = self.s.incoming[k];
            let l = &mut looks[k];
            let sunk = i >= 0 && self.is_sunk(i as usize);
            if i >= 0 {
                l.fill = if sunk { pal().sunk } else { pal().ship };
                if sunk {
                    l.sym_col = rgb(0xfe, 0xca, 0xca);
                }
                if self.s.ships[i as usize].shield == Shield::Active {
                    l.ring = Some(pal().yellow);
                }
                if pick {
                    l.hover = Some(rgb(0x94, 0xa8, 0xc0));
                }
            }
            match v {
                Some(Mark::Hit) => {
                    l.sym = "✕";
                    if !sunk {
                        l.fill = pal().hit;
                        l.sym_col = Color32::WHITE;
                    }
                }
                Some(Mark::Miss) => {
                    l.sym = "•";
                    l.sym_col = pal().miss;
                }
                Some(Mark::Blocked) => {
                    l.fill = pal().blk;
                    l.sym = SHIELD_SYM;
                    l.sym_col = pal().yellow;
                }
                _ => {}
            }
            l.last = self.s.last_in.contains(&k);
        }
        let (resp, g) = draw_grid(ui, gw, &looks, true, pick.then_some(pal().yellow), Sense::click());
        if resp.clicked() {
            if let Some((r, c)) = resp.interact_pointer_pos().and_then(|p| g.cell_at(p)) {
                self.on_my_tap(r, c);
            }
        }
        ui.add_space(10.0);
        // Modes de tir et améliorations
        let live = self.live();
        let rules = self.s.rules;
        let modes: Vec<(Mode, &str, &str, u32)> = [
            (Mode::Shot, ic::CROSSHAIR, "modeShot", 0, true),
            (Mode::Radar, ic::WIFI_HIGH, "modeRadar", self.s.inv.radar, rules.radar),
            (Mode::Cross, ic::PLUS, "modeCross", self.s.inv.cross, rules.cross),
            (Mode::Shield, ic::SHIELD, "modeShield", self.s.inv.shield, rules.shield),
        ]
        .into_iter()
        .filter(|m| m.4)
        .map(|m| (m.0, m.1, m.2, m.3))
        .collect();
        let aw = ui.available_width();
        ui.horizontal(|ui| {
            let bw = (aw - 8.0 * (modes.len() as f32 - 1.0)) / modes.len() as f32;
            for (m, icon, key, n) in modes {
                let enabled = live && (m == Mode::Shot || n > 0);
                let on = live && self.s.mode == m;
                let (r, resp) = ui.allocate_exact_size(vec2(bw, 56.0), if enabled { Sense::click() } else { Sense::hover() });
                let a = if enabled { 1.0 } else { 0.35 };
                let p = ui.painter();
                let fill = if enabled && resp.hovered() { pal().btn_hi } else { pal().btn };
                p.rect(r, rd(10.0), fill.gamma_multiply(a), if on { sk(2.0, pal().yellow) } else { egui::Stroke::NONE });
                p.text(r.center() - vec2(0.0, 9.0), Align2::CENTER_CENTER, icon, reg(17.0), pal().text.gamma_multiply(a));
                p.text(r.center() + vec2(0.0, 12.0), Align2::CENTER_CENTER, self.t(key), semi(12.0), pal().text.gamma_multiply(a));
                if m != Mode::Shot && n > 0 {
                    p.text(r.right_top() + vec2(-8.0, 5.0), Align2::RIGHT_TOP, n.to_string(), num(11.0), pal().yellow.gamma_multiply(a));
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
            (t.clone(), pal().err)
        } else if self.s.phase == Phase::Game && self.s.opp_away {
            (self.t("oppAway"), pal().muted)
        } else if live && self.s.aim.is_some() && matches!(self.s.mode, Mode::Radar | Mode::Cross) {
            let (r, c) = self.s.aim.unwrap_or((0, 0));
            (self.tf("hintConfirm", &[("cell", cell_name(r, c))]), pal().muted)
        } else if self.s.phase == Phase::Over && self.opp_ship_cells().iter().any(|&k| self.s.enemy[k] != Some(Mark::Sunk)) {
            (self.t("missedLegend"), pal().muted)
        } else if live {
            let k = match self.s.mode {
                Mode::Shot => "hintShot",
                Mode::Radar => "hintRadar",
                Mode::Cross => "hintCross",
                Mode::Shield => "hintShield",
            };
            (self.t(k), pal().muted)
        } else {
            (String::new(), pal().muted)
        };
        ui.add_space(6.0);
        ui.vertical_centered(|ui| {
            wrapped(ui, if hint.is_empty() { " ".into() } else { hint }, reg(13.0), col);
            let (log, opp) = self.s.log.clone();
            wrapped(ui, if log.is_empty() { " ".into() } else { log }, semi(14.0), if opp { pal().muted } else { pal().text });
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
                label(ui, ic::WARNING, reg(34.0), pal().yellow);
                ui.add_space(4.0);
                wrapped(ui, text, semi(15.0), pal().text);
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
                        Confirm::DeleteAccount => {
                            if self.me().is_some_and(|a| a.cloud) {
                                let pw = self.pw_del.clone();
                                self.delete_cloud(ctx, pw);
                            } else {
                                self.delete_account();
                            }
                        }
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
            label(ui, title, bold(22.0), pal().text);
            wrapped(ui, desc, reg(14.0), pal().muted);
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
        let rules = self.s.rules;
        let opts: Vec<_> = [
            (Mode::Radar, ic::WIFI_HIGH, self.t("upRadarN"), self.t("upRadarD"), rules.radar),
            (Mode::Cross, ic::PLUS, self.t("upCrossN"), self.t("upCrossD"), rules.cross),
            (Mode::Shield, ic::SHIELD, self.t("upShieldN"), self.t("upShieldD"), rules.shield),
        ]
        .into_iter()
        .filter(|o| o.4)
        .map(|o| (o.0, o.1, o.2, o.3))
        .collect();
        let mut picked = None;
        modal(ctx, "choice", 380.0, false, |ui| {
            label(ui, title, bold(22.0), pal().text);
            label(ui, sub, reg(14.0), pal().muted);
            ui.add_space(6.0);
            for (m, icon, name, desc) in opts {
                let resp = egui::Frame::none()
                    .fill(card2())
                    .stroke(sk(1.0, border()))
                    .rounding(rd(12.0))
                    .inner_margin(egui::Margin::symmetric(14.0, 12.0))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            label(ui, icon, reg(15.0), pal().yellow);
                            label(ui, &name, bold(15.0), pal().text);
                        });
                        wrapped(ui, &desc, reg(13.0), pal().muted);
                    })
                    .response;
                let resp = ui.interact(resp.rect, ui.id().with(name.as_str()), Sense::click());
                if resp.hovered() {
                    ui.painter().rect_stroke(resp.rect, rd(12.0), sk(1.0, pal().sky));
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
        let (xp, bot_note, guest, parts) = (e.xp, e.bot_note, e.guest, e.parts);
        let win = self.s.win;
        let lost = self.s.lost;
        let rank_line = xp.filter(|x| x.2).map(|(_, lvl, _, _, _)| format!("⬆ {}", self.tf("levelUp", &[("n", lvl.to_string()), ("rank", self.rank_of(lvl).to_string())])));
        let (ng, sg, bx, xg) = (self.t("newGame"), self.t("seeGrids"), self.t("botXp"), self.t("xpGuest"));
        let lvl_lbl = xp.map(|(_, lvl, _, _, _)| self.tf("levelN", &[("n", lvl.to_string())]));
        let name = self.s.opp.as_ref().map_or(String::new(), |o| o.name.clone());
        // contrôle de la grille adverse
        let (verify_txt, verify_bad) = match self.s.verify {
            Verify::Pending if self.s.record_pending => (self.t("verifyWait"), false),
            Verify::Ok => (self.t("verifyOk"), false),
            Verify::Cheat => (self.tf(if win { "verifyCheatWin" } else { "verifyCheat" }, &[("name", name.clone())]), true),
            Verify::Missing if !win => (self.tf("verifyMissing", &[("name", name.clone())]), true),
            _ => (String::new(), false),
        };
        let pending_xp = self.s.record_pending;
        // revanche
        let can_rematch = self.s.connected && !lost;
        let rematch_note = if self.s.opp_gone {
            self.t("rematchGone")
        } else if self.s.rematch_me {
            self.tf("rematchWait", &[("name", name.clone())])
        } else if self.s.rematch_opp {
            self.tf("rematchOpp", &[("name", name.clone())])
        } else {
            String::new()
        };
        let rematch_on = !self.s.opp_gone && !self.s.rematch_me;
        let rematch_lbl = self.t("rematch");
        let share_lbl = self.t("share");
        let (share_txt, share_kind) = self.share_msg.clone();
        let rows: Vec<(String, u64)> = if xp.is_some() && !pending_xp {
            vec![
                (self.t(if win { "xpBase1" } else { "xpBase0" }), parts.base),
                (self.t("xpHits"), parts.hits),
                (self.t("xpSunk"), parts.sunk),
                (self.t("xpAcc"), parts.acc),
                (self.t("xpTime"), parts.time),
            ]
        } else {
            vec![]
        };
        let mut new_game = false;
        let mut see = false;
        let mut rematch = false;
        let mut share = false;
        modal(ctx, "end", 380.0, true, |ui| {
            label(ui, icon, reg(46.0), pal().text);
            label(ui, title, bold(22.0), pal().text);
            wrapped(ui, text, reg(14.0), pal().muted);
            if !verify_txt.is_empty() {
                wrapped(ui, verify_txt, semi(12.5), if verify_bad { pal().err } else if pending_xp { pal().muted } else { pal().green });
            }
            if let Some((gain, _, _, inl, size)) = xp {
                if !pending_xp {
                    ui.add_space(4.0);
                    for (k, v) in &rows {
                        ui.horizontal(|ui| {
                            label(ui, k, reg(12.5), if *v > 0 { pal().text } else { pal().dim });
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                label(ui, if *v > 0 { format!("+{v}") } else { "—".into() }, bold(12.5), if *v > 0 { pal().green } else { pal().dim });
                            });
                        });
                    }
                    label(ui, format!("+{gain} XP"), num(24.0), pal().yellow);
                    if let Some(r) = &rank_line {
                        label(ui, r, bold(15.0), pal().green);
                    }
                    if bot_note {
                        label(ui, &bx, reg(12.0), pal().muted);
                    }
                    ui.add_space(4.0);
                    let w = ui.available_width();
                    xp_bar(ui, inl as f32 / size as f32, w);
                    ui.horizontal(|ui| {
                        label(ui, "XP:", reg(11.0), pal().muted);
                        label(ui, format!("{inl} / {size}"), bold(11.0), pal().sky);
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            label(ui, lvl_lbl.clone().unwrap_or_default(), reg(11.0), pal().muted);
                        });
                    });
                }
            } else if guest && !pending_xp {
                label(ui, &xg, reg(12.0), pal().muted);
            }
            ui.add_space(10.0);
            if can_rematch {
                if Btn::new(&rematch_lbl).icon(ic::ARROWS_CLOCKWISE).style(Bs::Primary).wide().enabled(rematch_on).show(ui).clicked() {
                    rematch = true;
                }
                if !rematch_note.is_empty() {
                    wrapped(ui, rematch_note, reg(12.5), pal().muted);
                }
                ui.add_space(2.0);
            }
            if Btn::new(&ng).style(if can_rematch { Bs::Normal } else { Bs::Primary }).wide().show(ui).clicked() {
                new_game = true;
            }
            ui.add_space(2.0);
            if !lost && Btn::new(&share_lbl).icon(ic::SHARE_NETWORK).style(Bs::Ghost).wide().show(ui).clicked() {
                share = true;
            }
            if !share_txt.is_empty() {
                wrapped(ui, share_txt, reg(12.5), if share_kind == MsgKind::Err { pal().err } else { pal().green });
            }
            if Btn::new(&sg).style(Bs::Ghost).wide().show(ui).clicked() {
                see = true;
            }
        });
        if new_game {
            self.reset_to_menu();
        } else if rematch {
            self.ask_rematch();
        } else if share {
            self.share_msg = Default::default();
            self.s.end_show = false; // on photographie les grilles, sans la fenêtre
            self.share_req = 1;
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
