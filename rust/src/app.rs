//! État de l'application et logique du jeu (portage fidèle de la version web).

use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

use eframe::egui;
use rand::Rng;
use serde_json::{json, Value as Json};

use crate::accounts::{self, level_of, valid_photo, Account, Settings, Store};
use crate::game::{self, *};
use crate::i18n::{self, fill, tr, Lang};
use crate::net::{ConnId, Net, NetEvent, PeerErr, PeerId};
use crate::sound::Sound;
use crate::theme::MsgKind;

pub const PREFIX: &str = "c02-naval-v1-";
pub const MM_PREFIX: &str = "c02-naval-mm-v1-";
pub const MM_SLOTS: usize = 6;
pub const MM_WAIT: Duration = Duration::from_secs(45);
pub const HEARTBEAT: Duration = Duration::from_secs(2);
pub const TIMEOUT: Duration = Duration::from_secs(20);
pub const AWAY_TIMEOUT: Duration = Duration::from_secs(120);
pub const JOIN_TIMEOUT: Duration = Duration::from_secs(15);
pub const WEB_URL: &str = "https://drfrrobin.github.io/C02-Naval/";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum View {
    Auth,
    Dash,
    Lobby,
    Place,
    Game,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pane {
    Play,
    Stats,
    Settings,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    Menu,
    Lobby,
    Place,
    Game,
    Over,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Host,
    Join,
    Random,
    Bot,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Shot,
    Radar,
    Cross,
    Shield,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reject {
    HostWaiting,
    Join,
    Slot(usize),
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PeerRole {
    Host,
    Joiner,
    SlotOwner(usize),
    SlotJoiner(usize),
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Confirm {
    DeleteAccount,
    Quit,
}

#[derive(Debug)]
enum Timer {
    BotEmit(Json),
    BotMove,
    TurnSound,
    OpenChoice,
    EndSound(bool),
    EndShow,
    StartBot,
    JoinTimeout,
    SlotTimeout(u64),
    TrySlot(usize),
    Reconnect(PeerId),
    BusyClose(ConnId),
}

#[derive(Clone, Debug)]
pub struct Opp {
    pub name: String,
    pub photo: Option<String>,
    pub level: u64,
    pub bot: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Preview {
    pub r: i32,
    pub c: i32,
    pub h: bool,
}

#[derive(Clone, Debug)]
pub struct Lifted {
    pub idx: usize,
    pub cells: Vec<Cell>,
}

#[derive(Clone, Debug)]
pub struct Drag {
    pub r: usize,
    pub c: usize,
    pub moved: bool,
    pub ship: bool,
    pub was_selected: bool,
    pub off: (i32, i32),
}

#[derive(Default, Clone, Copy, Debug)]
pub struct Inv {
    pub radar: u32,
    pub cross: u32,
    pub shield: u32,
}

#[derive(Default, Clone, Copy, Debug)]
pub struct MatchStats {
    pub shots: u32,
    pub hits: u32,
    pub sunk: u32,
    pub upgrades: u32,
    pub start: Option<Instant>,
}

pub struct EndInfo {
    pub icon: &'static str,
    pub title: String,
    pub text: String,
    pub xp: Option<(u64, u64, bool, u64, u64)>, // (gain, niveau, monté ?, xp dans le niveau, taille du niveau)
    pub bot_note: bool,
    pub guest: bool,
}

/// État d'une partie (remis à zéro à chaque « Nouvelle partie »).
pub struct GameState {
    pub kind: Option<Kind>,
    pub peer: Option<PeerId>,
    pub role: Option<PeerRole>,
    pub conn: Option<ConnId>,
    pub reject: Option<Reject>,
    pub reject_conns: Vec<ConnId>,
    pub connected: bool,
    pub is_host: bool,
    pub code: String,
    pub phase: Phase,
    pub lost: bool,
    pub vs_bot: bool,
    pub bot: Option<Bot>,
    pub ships: Vec<Ship>,
    pub sel: i32,
    pub horiz: bool,
    pub preview: Option<Preview>,
    pub lifted: Option<Lifted>,
    pub drag: Option<Drag>,
    pub me_ready: bool,
    pub opp_ready: bool,
    pub my_turn: bool,
    pub waiting: bool,
    pub mode: Mode,
    pub enemy: Vec<Option<Mark>>,
    pub incoming: Vec<Option<Mark>>,
    pub enemy_sunk: Vec<usize>,
    pub inv: Inv,
    pub choices: u32,
    pub last_in: Vec<usize>,
    pub last_seen: Instant,
    pub next_ping: Instant,
    pub pending_act: Option<(ActKind, Cell)>,
    pub opp_away: bool,
    pub opp: Option<Opp>,
    pub m: MatchStats,
    pub mm_deadline: Option<Instant>,
    pub mm_stopped: bool,
    pub slot_gen: u64,
    pub slot_done: bool,
    pub join_failed: bool,
    pub join_busy: bool,
    pub show_choice: bool,
    pub end: Option<EndInfo>,
    pub end_show: bool,
    pub log: (String, bool),
}

impl GameState {
    fn new() -> GameState {
        let now = Instant::now();
        GameState {
            kind: None,
            peer: None,
            role: None,
            conn: None,
            reject: None,
            reject_conns: vec![],
            connected: false,
            is_host: false,
            code: String::new(),
            phase: Phase::Menu,
            lost: false,
            vs_bot: false,
            bot: None,
            ships: Ship::fleet(),
            sel: 0,
            horiz: true,
            preview: None,
            lifted: None,
            drag: None,
            me_ready: false,
            opp_ready: false,
            my_turn: false,
            waiting: false,
            mode: Mode::Shot,
            enemy: vec![None; N * N],
            incoming: vec![None; N * N],
            enemy_sunk: vec![],
            inv: Inv::default(),
            choices: 0,
            last_in: vec![],
            last_seen: now,
            next_ping: now,
            pending_act: None,
            opp_away: false,
            opp: None,
            m: MatchStats::default(),
            mm_deadline: None,
            mm_stopped: false,
            slot_gen: 0,
            slot_done: false,
            join_failed: false,
            join_busy: false,
            show_choice: false,
            end: None,
            end_show: false,
            log: (String::new(), false),
        }
    }
}

pub struct App {
    pub settings: Settings,
    pub store: Store,
    pub cur_id: Option<String>,
    pub view: View,
    pub pane: Pane,
    pub auth_signup: bool,
    pub picked: Option<String>,
    // champs de saisie
    pub login_pass: String,
    pub su_name: String,
    pub su_pass: String,
    pub su_pass2: String,
    pub edit_name: String,
    pub edit_pass: String,
    pub code_input: String,
    // messages
    pub auth_msg: (String, MsgKind),
    pub dash_msg: (String, MsgKind),
    pub set_msg: (String, MsgKind),
    pub lobby_msg: (String, MsgKind),
    pub join_msg: (String, MsgKind),
    pub place_msg: (String, MsgKind),
    pub flash: Option<(String, Instant)>,
    pub show_join: bool,
    pub confirm: Option<Confirm>,
    pub pending_code: Option<String>,
    pub s: GameState,
    pub net: Net,
    ev_rx: Receiver<NetEvent>,
    timers: Vec<(Instant, Timer)>,
    pub snd: Sound,
    pub textures: HashMap<String, egui::TextureHandle>,
    was_minimized: bool,
    awake: bool,
    pub focus_pass: bool,
    pub focus_code: bool,
}

pub fn gen_code() -> String {
    let mut r = rand::thread_rng();
    (0..6).map(|_| ALPHA[r.gen_range(0..ALPHA.len())] as char).collect()
}

impl App {
    pub fn new(cc: &eframe::CreationContext) -> App {
        crate::theme::setup_fonts(&cc.egui_ctx);
        crate::theme::setup_style(&cc.egui_ctx);
        let (tx, rx) = channel();
        let net = Net::new(tx, cc.egui_ctx.clone());
        let settings = Settings::load();
        let mut store = Store::default();
        store.load();
        // Code d'invitation passé en argument (ex. : c02-naval.exe ABC234).
        let pending_code = std::env::args()
            .skip(1)
            .map(|a| a.rsplit("code=").next().unwrap_or("").trim().to_uppercase())
            .find(|a| a.len() == 6 && a.bytes().all(|b| ALPHA.contains(&b)));
        let mut app = App {
            settings,
            store,
            cur_id: None,
            view: View::Auth,
            pane: Pane::Play,
            auth_signup: false,
            picked: None,
            login_pass: String::new(),
            su_name: String::new(),
            su_pass: String::new(),
            su_pass2: String::new(),
            edit_name: String::new(),
            edit_pass: String::new(),
            code_input: String::new(),
            auth_msg: Default::default(),
            dash_msg: Default::default(),
            set_msg: Default::default(),
            lobby_msg: Default::default(),
            join_msg: Default::default(),
            place_msg: Default::default(),
            flash: None,
            show_join: false,
            confirm: None,
            pending_code,
            s: GameState::new(),
            net,
            ev_rx: rx,
            timers: vec![],
            snd: Sound::new(),
            textures: HashMap::new(),
            was_minimized: false,
            awake: false,
            focus_pass: false,
            focus_code: false,
        };
        // Session précédente.
        match app.settings.current.clone() {
            Some(id) if id == "guest" => app.open_dash(Pane::Play),
            Some(id) if app.store.accounts.contains_key(&id) => {
                app.cur_id = Some(id);
                app.open_dash(Pane::Play);
            }
            _ => app.open_auth(None),
        }
        app
    }

    /* ---------- Outils ---------- */

    pub fn lang(&self) -> Lang {
        self.settings.lang
    }
    pub fn t(&self, key: &str) -> String {
        tr(self.settings.lang, key).to_string()
    }
    pub fn tf(&self, key: &str, p: &[(&str, String)]) -> String {
        fill(tr(self.settings.lang, key), p)
    }
    pub fn ship_name(&self, i: usize) -> String {
        self.t(&format!("ship{i}"))
    }
    pub fn rank_of(&self, l: u64) -> &'static str {
        let r = i18n::ranks(self.settings.lang);
        r[(l as usize - 1).min(r.len() - 1)]
    }
    pub fn play(&mut self, name: &str) {
        let on = self.settings.sound;
        self.snd.play(name, on);
    }
    fn after(&mut self, d: Duration, t: Timer) {
        self.timers.push((Instant::now() + d, t));
    }
    fn after_ms(&mut self, ms: u64, t: Timer) {
        self.after(Duration::from_millis(ms), t);
    }
    fn cancel(&mut self, f: impl Fn(&Timer) -> bool) {
        self.timers.retain(|(_, t)| !f(t));
    }
    pub fn me(&self) -> Option<&Account> {
        self.cur_id.as_ref().and_then(|id| self.store.accounts.get(id))
    }
    pub fn my_identity(&self) -> Opp {
        match self.me() {
            Some(a) => Opp {
                name: a.name.clone(),
                photo: a.photo.clone().filter(|p| valid_photo(p)),
                level: level_of(a.xp),
                bot: false,
            },
            None => Opp { name: self.t("guestName"), photo: None, level: 0, bot: false },
        }
    }
    pub fn occupancy(&self) -> [i32; N * N] {
        occ_of(&self.s.ships)
    }
    pub fn is_sunk(&self, i: usize) -> bool {
        sunk_of(&self.s.ships[i], &self.s.incoming)
    }
    pub fn targetable(&self, k: usize) -> bool {
        matches!(self.s.enemy[k], None | Some(Mark::ScanOcc) | Some(Mark::ScanEmpty) | Some(Mark::Blocked))
    }
    fn set_session(&mut self, id: Option<String>) {
        self.cur_id = id.clone();
        self.settings.current = Some(id.unwrap_or_else(|| "guest".into()));
        self.settings.save();
    }
    fn clear_session(&mut self) {
        self.cur_id = None;
        self.settings.current = None;
        self.settings.save();
    }
    pub fn set_lang(&mut self, l: Lang) {
        self.settings.lang = l;
        self.settings.save();
    }
    pub fn toggle_sound(&mut self) {
        self.settings.sound = !self.settings.sound;
        self.settings.save();
        self.play("click");
    }

    /* ---------- Connexion ---------- */

    pub fn open_auth(&mut self, signup: Option<bool>) {
        self.store.load();
        self.auth_signup = signup.unwrap_or(self.store.accounts.is_empty());
        let accs = self.store.sorted();
        if self.picked.as_ref().map_or(true, |p| !self.store.accounts.contains_key(p)) {
            self.picked = accs.first().map(|a| a.id.clone());
        }
        self.view = View::Auth;
    }

    pub fn do_login(&mut self) {
        self.store.load();
        let Some(mut a) = self.picked.as_ref().and_then(|p| self.store.accounts.get(p)).cloned() else {
            self.auth_msg = (self.t("errPickAcc"), MsgKind::Err);
            return;
        };
        if let Some(h) = &a.pass {
            if !accounts::check_pass(&self.login_pass, h) {
                self.play("blocked");
                self.auth_msg = (self.t("errPassWrong"), MsgKind::Err);
                return;
            }
        }
        a.last_login = accounts::now_iso();
        self.store.save(&a);
        self.login_pass.clear();
        self.auth_msg = Default::default();
        self.set_session(Some(a.id.clone()));
        self.open_dash(Pane::Play);
        self.dash_msg = if a.pass.is_some() {
            (self.tf("welcome", &[("name", a.name.clone())]), MsgKind::Ok)
        } else {
            (self.t("noPassHint"), MsgKind::Err)
        };
    }

    pub fn do_signup(&mut self) {
        self.store.load();
        let name = self.su_name.split_whitespace().collect::<Vec<_>>().join(" ");
        let n = name.chars().count();
        if !(2..=16).contains(&n) {
            self.auth_msg = (self.t("errPseudoLen"), MsgKind::Err);
            return;
        }
        if self.store.name_taken(&name, None) {
            self.auth_msg = (self.t("errPseudoTaken"), MsgKind::Err);
            return;
        }
        if self.su_pass.chars().count() < 4 {
            self.auth_msg = (self.t("errPassLen"), MsgKind::Err);
            return;
        }
        if self.su_pass != self.su_pass2 {
            self.auth_msg = (self.t("errPassMatch"), MsgKind::Err);
            return;
        }
        let now = accounts::now_iso();
        let acc = Account {
            id: accounts::new_id(),
            name: name.clone(),
            pass: Some(accounts::hash_pass(&self.su_pass, None, 120_000)),
            photo: None,
            xp: 0,
            created: now.clone(),
            last_login: now,
            stats: Default::default(),
        };
        self.store.save(&acc);
        self.su_name.clear();
        self.su_pass.clear();
        self.su_pass2.clear();
        self.auth_msg = Default::default();
        self.set_session(Some(acc.id.clone()));
        self.open_dash(Pane::Play);
        self.dash_msg = if self.store.storage_ok {
            (self.tf("welcome", &[("name", name)]), MsgKind::Ok)
        } else {
            (self.t("errStorage"), MsgKind::Err)
        };
    }

    pub fn guest(&mut self) {
        self.set_session(None);
        self.open_dash(Pane::Play);
    }

    pub fn logout(&mut self) {
        self.clear_session();
        self.auth_msg = Default::default();
        self.open_auth(Some(false));
    }

    /* ---------- Tableau de bord ---------- */

    pub fn open_dash(&mut self, p: Pane) {
        self.s.phase = Phase::Menu;
        self.pane = p;
        self.store.load();
        if self.cur_id.is_some() && self.me().is_none() {
            // compte supprimé ailleurs
            self.clear_session();
            self.open_auth(None);
            return;
        }
        if let Some(a) = self.me() {
            self.edit_name = a.name.clone();
        }
        self.view = View::Dash;
        if let Some(c) = self.pending_code.take() {
            self.open_join(&c);
        }
    }

    pub fn set_pane(&mut self, p: Pane) {
        self.pane = p;
        self.set_msg = Default::default();
        self.store.load();
        if let Some(a) = self.me() {
            self.edit_name = a.name.clone();
        }
        self.play("click");
    }

    pub fn save_profile(&mut self) {
        self.store.load();
        let Some(mut a) = self.me().cloned() else { return };
        let name = self.edit_name.split_whitespace().collect::<Vec<_>>().join(" ");
        let n = name.chars().count();
        if !(2..=16).contains(&n) {
            self.set_msg = (self.t("errPseudoLen"), MsgKind::Err);
            return;
        }
        if self.store.name_taken(&name, Some(&a.id)) {
            self.set_msg = (self.t("errPseudoTaken"), MsgKind::Err);
            return;
        }
        let pw = self.edit_pass.clone();
        if !pw.is_empty() && pw.chars().count() < 4 {
            self.set_msg = (self.t("errPassLen"), MsgKind::Err);
            return;
        }
        if name == a.name && pw.is_empty() {
            self.set_msg = (self.t("nothingChanged"), MsgKind::Info);
            return;
        }
        if !pw.is_empty() {
            a.pass = Some(accounts::hash_pass(&pw, None, 120_000));
        }
        a.name = name;
        self.store.save(&a);
        self.edit_pass.clear();
        self.set_msg = (self.t("saved"), MsgKind::Ok);
    }

    pub fn delete_account(&mut self) {
        let Some(id) = self.cur_id.clone() else { return };
        self.store.delete(&id);
        self.clear_session();
        self.open_auth(None);
    }

    pub fn pick_photo(&mut self) {
        if self.me().is_none() {
            return;
        }
        let file = rfd::FileDialog::new()
            .set_title(self.t("photoPick"))
            .add_filter(self.t("images"), &["jpg", "jpeg", "png", "webp", "gif", "bmp"])
            .pick_file();
        let Some(path) = file else { return };
        match accounts::make_photo(&path) {
            Some(url) => {
                self.store.load();
                if let Some(mut a) = self.me().cloned() {
                    a.photo = Some(url);
                    self.store.save(&a);
                }
            }
            None => self.dash_msg = (self.t("errPhoto"), MsgKind::Err),
        }
    }

    pub fn remove_photo(&mut self) {
        self.store.load();
        if let Some(mut a) = self.me().cloned() {
            a.photo = None;
            self.store.save(&a);
        }
    }

    /// Texture d'une photo (mise en cache).
    pub fn photo_tex(&mut self, ctx: &egui::Context, url: &str) -> Option<egui::TextureHandle> {
        if let Some(t) = self.textures.get(url) {
            return Some(t.clone());
        }
        let (w, h, rgba) = accounts::decode_photo(url)?;
        let tex = ctx.load_texture(
            format!("photo{}", self.textures.len()),
            egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba),
            egui::TextureOptions::LINEAR,
        );
        if self.textures.len() > 64 {
            self.textures.clear();
        }
        self.textures.insert(url.to_string(), tex.clone());
        Some(tex)
    }

    /* ---------- Enregistrement des parties ---------- */

    fn record_game(&mut self, win: bool) -> Option<(u64, u64, bool)> {
        self.store.load();
        let mut a = self.me().cloned()?;
        let m = self.s.m;
        let acc = if m.shots > 0 { m.hits as f64 / m.shots as f64 } else { 0.0 };
        let mut gain = (if win { 100 } else { 30 }) as f64 + m.hits as f64 * 5.0 + m.sunk as f64 * 20.0
            + if win { (acc * 50.0).round() } else { 0.0 };
        if self.s.vs_bot {
            gain = (gain / 2.0).round();
        }
        let gain = gain as u64;
        let before = level_of(a.xp);
        a.xp += gain;
        let s = &mut a.stats;
        s.games += 1;
        s.shots += m.shots;
        s.hits += m.hits;
        s.sunk += m.sunk;
        s.upgrades += m.upgrades;
        s.time_played += m.start.map_or(0, |t| t.elapsed().as_secs_f64().round() as u64);
        if win {
            s.wins += 1;
            s.streak += 1;
            s.best_streak = s.best_streak.max(s.streak);
            if s.fastest_win.map_or(true, |f| m.shots < f) {
                s.fastest_win = Some(m.shots);
            }
        } else {
            s.losses += 1;
            s.streak = 0;
        }
        let lvl = level_of(a.xp);
        self.store.save(&a);
        Some((gain, lvl, lvl > before))
    }

    fn record_interrupted(&mut self) {
        if self.s.phase != Phase::Game {
            return;
        }
        self.store.load();
        let Some(mut a) = self.me().cloned() else { return };
        a.stats.interrupted += 1;
        a.stats.time_played += self.s.m.start.map_or(0, |t| t.elapsed().as_secs_f64().round() as u64);
        self.store.save(&a);
    }

    /* =========================================================
       Réseau
       ========================================================= */

    fn net_error_text(&self, e: &PeerErr) -> String {
        match e {
            PeerErr::Network => self.t("errNet"),
            PeerErr::PeerUnavailable => self.t("errNoGame"),
            PeerErr::UnavailableId => self.tf("errNetOther", &[("type", "unavailable-id".into())]),
            PeerErr::Other(t) => self.tf("errNetOther", &[("type", t.clone())]),
        }
    }

    fn setup_lobby(&mut self, host: bool) {
        self.lobby_msg = (self.t(if host { "creating" } else { "searching" }), MsgKind::Info);
        self.view = View::Lobby;
    }

    // Salon privé
    pub fn host_game(&mut self) {
        self.s.kind = Some(Kind::Host);
        self.s.is_host = true;
        self.s.code = gen_code();
        self.s.phase = Phase::Lobby;
        self.setup_lobby(true);
        let pid = self.net.create_peer(Some(format!("{PREFIX}{}", self.s.code)));
        self.s.peer = Some(pid);
        self.s.role = Some(PeerRole::Host);
    }

    // Rejoindre avec un code
    pub fn open_join(&mut self, code: &str) {
        self.code_input = code.to_uppercase().chars().take(6).collect();
        self.join_msg = Default::default();
        self.s.join_busy = false;
        self.show_join = true;
        self.focus_code = true;
    }

    pub fn join_game(&mut self) {
        if self.s.peer.is_some() {
            return;
        }
        let code: String = self.code_input.to_uppercase().split_whitespace().collect();
        if code.chars().count() != 6 {
            self.join_msg = (self.t("errCodeLen"), MsgKind::Err);
            return;
        }
        if code.bytes().any(|b| !ALPHA.contains(&b)) {
            self.join_msg = (self.t("errCodeChars"), MsgKind::Err);
            return;
        }
        self.join_msg = (self.t("connecting"), MsgKind::Info);
        self.s.join_busy = true;
        self.s.kind = Some(Kind::Join);
        self.s.is_host = false;
        self.s.join_failed = false;
        let pid = self.net.create_peer(None);
        self.s.peer = Some(pid);
        self.s.role = Some(PeerRole::Joiner);
        self.s.code = code;
    }

    pub fn cancel_join(&mut self) {
        if !self.s.connected {
            if let Some(p) = self.s.peer.take() {
                self.net.destroy(p);
            }
            self.s.conn = None;
            self.cancel(|t| matches!(t, Timer::JoinTimeout));
        }
        self.show_join = false;
    }

    fn join_fail(&mut self, text: String) {
        self.cancel(|t| matches!(t, Timer::JoinTimeout));
        if self.s.connected || self.s.join_failed {
            return;
        }
        self.s.join_failed = true;
        if let Some(p) = self.s.peer.take() {
            self.net.destroy(p);
        }
        self.s.conn = None;
        self.s.join_busy = false;
        self.join_msg = (text, MsgKind::Err);
    }

    // Adversaire aléatoire : quelques « créneaux » publics. On essaie d'en occuper un (on attend),
    // et s'il est déjà pris, quelqu'un y attend : on le rejoint. Bot au bout de 45 s.
    pub fn find_random(&mut self) {
        self.s.kind = Some(Kind::Random);
        self.s.phase = Phase::Lobby;
        self.s.mm_stopped = false;
        self.setup_lobby(false);
        self.s.mm_deadline = Some(Instant::now() + MM_WAIT);
        self.try_slot(0);
    }

    fn stop_search(&mut self) {
        self.s.mm_stopped = true;
        self.s.mm_deadline = None;
        if let Some(c) = self.s.conn.take() {
            self.net.close_conn(c);
        }
        if let Some(p) = self.s.peer.take() {
            self.net.destroy(p);
        }
    }

    fn try_slot(&mut self, k: usize) {
        if self.s.connected || self.s.mm_stopped {
            return;
        }
        if k >= MM_SLOTS {
            self.after_ms(1500, Timer::TrySlot(0));
            return;
        }
        let pid = self.net.create_peer(Some(format!("{MM_PREFIX}{k}")));
        self.s.peer = Some(pid);
        self.s.role = Some(PeerRole::SlotOwner(k));
    }

    fn join_slot(&mut self, k: usize) {
        if self.s.connected || self.s.mm_stopped {
            return;
        }
        let pid = self.net.create_peer(None);
        self.s.peer = Some(pid);
        self.s.role = Some(PeerRole::SlotJoiner(k));
        self.s.is_host = false;
        self.s.slot_done = false;
        self.s.slot_gen += 1;
    }

    fn slot_next(&mut self, k: usize) {
        if self.s.slot_done || self.s.connected || self.s.mm_stopped {
            return;
        }
        self.s.slot_done = true;
        self.s.slot_gen += 1;
        self.cancel(|t| matches!(t, Timer::SlotTimeout(_)));
        if let Some(c) = self.s.conn.take() {
            self.net.close_conn(c);
        }
        if let Some(p) = self.s.peer.take() {
            self.net.destroy(p);
        }
        self.try_slot(k + 1);
    }

    fn keep_peer_alive(&mut self, pid: PeerId) {
        // Lien perdu avec le service de mise en relation (veille, réseau) : on se réenregistre
        // pour que le code reste valable tant que l'adversaire n'est pas arrivé.
        if self.s.connected {
            return;
        }
        self.lobby_msg = (self.t("reconnecting"), MsgKind::Info);
        self.after_ms(1000, Timer::Reconnect(pid));
    }

    fn accept_conn(&mut self, conn: ConnId) {
        if self.s.conn.is_some() || self.s.connected {
            // déjà un adversaire : on refuse poliment
            self.s.reject_conns.push(conn);
            return;
        }
        self.setup_conn(conn, Reject::HostWaiting);
    }

    fn setup_conn(&mut self, conn: ConnId, on_fail: Reject) {
        self.s.conn = Some(conn);
        self.s.reject = Some(on_fail);
    }

    fn on_reject(&mut self, busy: bool) {
        match self.s.reject {
            Some(Reject::HostWaiting) => {
                self.s.conn = None;
                if !self.s.connected {
                    self.lobby_msg = (self.t("waitingOpp"), MsgKind::Info);
                }
            }
            Some(Reject::Join) => {
                let t = self.t(if busy { "errFull" } else { "errJoinTimeout" });
                self.join_fail(t);
            }
            Some(Reject::Slot(k)) => self.slot_next(k),
            None => {}
        }
    }

    fn handle_net(&mut self, e: NetEvent) {
        match e {
            NetEvent::PeerOpen(pid) => {
                if self.s.peer != Some(pid) {
                    return;
                }
                match self.s.role {
                    Some(PeerRole::Host) => self.lobby_msg = (self.t("waitingOpp"), MsgKind::Info),
                    Some(PeerRole::Joiner) => {
                        let c = self.net.connect(pid, &format!("{PREFIX}{}", self.s.code));
                        self.setup_conn(c, Reject::Join);
                        self.after(JOIN_TIMEOUT, Timer::JoinTimeout);
                    }
                    Some(PeerRole::SlotOwner(_)) => {
                        if self.s.mm_stopped {
                            self.net.destroy(pid);
                            return;
                        }
                        self.s.is_host = true;
                        self.lobby_msg = (self.t("waitingOpp"), MsgKind::Info);
                    }
                    Some(PeerRole::SlotJoiner(k)) => {
                        if self.s.mm_stopped {
                            self.net.destroy(pid);
                            return;
                        }
                        let c = self.net.connect(pid, &format!("{MM_PREFIX}{k}"));
                        self.setup_conn(c, Reject::Slot(k));
                        let g = self.s.slot_gen;
                        self.after_ms(8000, Timer::SlotTimeout(g));
                    }
                    None => {}
                }
            }
            NetEvent::PeerError(pid, err) => {
                if self.s.peer != Some(pid) {
                    return;
                }
                match self.s.role {
                    Some(PeerRole::Host) => {
                        if err == PeerErr::UnavailableId {
                            // code déjà pris : on en tire un autre
                            self.net.destroy(pid);
                            self.host_game();
                            return;
                        }
                        if !self.s.connected {
                            self.lobby_msg = (self.net_error_text(&err), MsgKind::Err);
                        }
                    }
                    Some(PeerRole::Joiner) => {
                        if !self.s.connected {
                            let t = self.net_error_text(&err);
                            self.join_fail(t);
                        }
                    }
                    Some(PeerRole::SlotOwner(k)) => {
                        if self.s.connected || self.s.mm_stopped {
                            return;
                        }
                        if err == PeerErr::UnavailableId {
                            self.net.destroy(pid);
                            self.join_slot(k);
                            return;
                        }
                        self.lobby_msg = (self.net_error_text(&err), MsgKind::Err);
                    }
                    Some(PeerRole::SlotJoiner(k)) => {
                        if !self.s.connected {
                            self.slot_next(k);
                        }
                    }
                    None => {}
                }
            }
            NetEvent::PeerDisconnected(pid) => {
                if self.s.peer == Some(pid) && matches!(self.s.role, Some(PeerRole::Host | PeerRole::SlotOwner(_))) {
                    self.keep_peer_alive(pid);
                }
            }
            NetEvent::Incoming(pid, conn) => {
                self.net.adopt(pid, conn);
                if self.s.peer != Some(pid) || self.s.mm_stopped && self.s.kind == Some(Kind::Random) {
                    self.net.close_conn(conn);
                    return;
                }
                self.accept_conn(conn);
            }
            NetEvent::ConnOpen(conn) => {
                if self.s.reject_conns.contains(&conn) {
                    self.net.send(conn, json!({"t":"busy"}));
                    self.after_ms(300, Timer::BusyClose(conn));
                    return;
                }
                if self.s.conn == Some(conn) {
                    let me = self.my_identity();
                    self.send(json!({"t":"hello","name":me.name,"photo":me.photo,"level":me.level}));
                }
            }
            NetEvent::ConnData(conn, m) => {
                if self.s.conn == Some(conn) {
                    self.on_data(m);
                }
            }
            NetEvent::ConnClosed(conn) => {
                self.s.reject_conns.retain(|c| *c != conn);
                if self.s.conn != Some(conn) {
                    return;
                }
                if self.s.connected {
                    self.lost();
                } else {
                    self.on_reject(false);
                }
            }
        }
    }

    fn on_connected(&mut self) {
        self.s.connected = true;
        self.cancel(|t| matches!(t, Timer::JoinTimeout | Timer::SlotTimeout(_) | Timer::TrySlot(_)));
        self.s.mm_deadline = None;
        self.show_join = false;
        self.s.last_seen = Instant::now();
        self.s.next_ping = Instant::now() + HEARTBEAT;
        // Créneau aléatoire : on le libère pour le prochain joueur (la liaison directe reste ouverte).
        if self.s.kind == Some(Kind::Random) {
            if let Some(p) = self.s.peer {
                self.net.disconnect(p);
            }
        }
        self.enter_placement();
    }

    pub fn send(&mut self, m: Json) {
        if self.s.vs_bot {
            self.bot_receive(&m);
            return;
        }
        if let Some(c) = self.s.conn {
            self.net.send(c, m);
        }
    }

    fn lost(&mut self) {
        if !self.s.connected || self.s.lost || self.s.phase == Phase::Over {
            return;
        }
        self.record_interrupted();
        self.s.lost = true;
        self.s.phase = Phase::Over;
        self.s.show_choice = false;
        self.s.end = Some(EndInfo {
            icon: "📴",
            title: self.t("lostTitle"),
            text: self.t("lostText"),
            xp: None,
            bot_note: false,
            guest: false,
        });
        self.s.end_show = true;
    }

    pub fn quit_game(&mut self) {
        self.record_interrupted();
        self.s.phase = Phase::Over;
        self.reset_to_menu();
    }

    /// Équivalent de « recharger la page » : on repart du tableau de bord.
    pub fn reset_to_menu(&mut self) {
        if let Some(c) = self.s.conn {
            self.net.close_conn(c);
        }
        self.net.destroy_all();
        self.timers.clear();
        self.s = GameState::new();
        self.flash = None;
        self.place_msg = Default::default();
        self.lobby_msg = Default::default();
        self.show_join = false;
        self.confirm = None;
        self.set_awake(false);
        self.open_dash(Pane::Play);
    }

    pub fn copy(&mut self, text: String) {
        let ok = arboard::Clipboard::new().and_then(|mut c| c.set_text(text)).is_ok();
        self.lobby_msg = if ok { (self.t("copied"), MsgKind::Ok) } else { (self.t("copyFail"), MsgKind::Err) };
    }

    /// Empêche la mise en veille de l'écran pendant une partie en ligne.
    fn set_awake(&mut self, on: bool) {
        if on == self.awake {
            return;
        }
        self.awake = on;
        #[cfg(windows)]
        unsafe {
            use windows_sys::Win32::System::Power::*;
            if on {
                SetThreadExecutionState(ES_CONTINUOUS | ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED);
            } else {
                SetThreadExecutionState(ES_CONTINUOUS);
            }
        }
    }

    fn on_visibility(&mut self, minimized: bool) {
        if !self.s.connected || self.s.vs_bot || self.s.lost || self.s.phase == Phase::Over {
            return;
        }
        if minimized {
            self.send(json!({"t":"away"}));
            return;
        }
        // De retour : on repart d'un délai neuf avant de juger la liaison.
        self.s.last_seen = Instant::now();
        self.send(json!({"t":"back"}));
    }

    fn show_away(&mut self) {
        if self.s.phase == Phase::Place {
            if self.s.opp_away {
                self.place_msg = (self.t("oppAway"), MsgKind::Info);
            } else {
                self.place_msg = Default::default();
            }
        }
    }

    fn on_data(&mut self, m: Json) {
        self.s.last_seen = Instant::now();
        if !m.is_object() || self.s.lost {
            return;
        }
        match m["t"].as_str().unwrap_or("") {
            "hello" => {
                let name: String = m["name"].as_str().unwrap_or("?").chars().take(16).collect();
                let photo = m["photo"].as_str().filter(|p| valid_photo(p)).map(|s| s.to_string());
                let level = m["level"].as_u64().or_else(|| m["level"].as_f64().map(|f| f as u64)).unwrap_or(0).min(999);
                let bot = m["bot"].as_bool().unwrap_or(false) && self.s.vs_bot;
                self.s.opp = Some(Opp { name, photo, level, bot });
                if !self.s.connected {
                    self.on_connected();
                }
            }
            "busy" => {
                if !self.s.connected {
                    self.on_reject(true);
                }
            }
            "away" => {
                self.s.opp_away = true;
                self.show_away();
            }
            "back" => {
                self.s.opp_away = false;
                self.show_away();
            }
            "ready" => {
                self.s.opp_ready = true;
                self.maybe_start();
            }
            "act" => self.on_act(&m),
            "res" => self.on_res(&m),
            "pass" => self.on_pass(),
            _ => {}
        }
    }

    /* =========================================================
       Bot
       ========================================================= */

    pub fn start_bot(&mut self) {
        self.s.kind = Some(Kind::Bot);
        self.s.vs_bot = true;
        self.s.is_host = true;
        self.s.bot = Some(Bot::new());
        self.s.connected = true;
        self.s.opp = Some(Opp { name: self.t("botName"), photo: None, level: self.my_identity().level.max(1), bot: true });
        self.enter_placement();
        let ready = 900 + rand::thread_rng().gen_range(0..900);
        self.after_ms(ready, Timer::BotEmit(json!({"t":"ready"})));
    }

    fn bot_receive(&mut self, m: &Json) {
        let Some(bot) = self.s.bot.as_mut() else { return };
        match m["t"].as_str().unwrap_or("") {
            "act" => {
                let cells = sanitize_cells(&m["cells"]);
                let kind = ActKind::parse(m["kind"].as_str().unwrap_or("shot"));
                let (results, over) = resolve_on(&mut bot.ships, &mut bot.incoming, kind, &cells);
                let res = json!({"t":"res","kind":kind.as_str(),"at":m["at"],"results":results.iter().map(ShotRes::to_json).collect::<Vec<_>>(),"over":over});
                self.after_ms(450, Timer::BotEmit(res));
                if !over {
                    let d = 450 + 700 + rand::thread_rng().gen_range(0..600);
                    self.after_ms(d, Timer::BotMove);
                }
            }
            "pass" => self.after_ms(900, Timer::BotMove),
            "res" => {
                let results: Vec<ShotRes> = m["results"].as_array().map(|a| a.iter().filter_map(ShotRes::from_json).collect()).unwrap_or_default();
                bot.learn(&results);
            }
            _ => {}
        }
    }

    fn bot_move(&mut self) {
        if self.s.phase != Phase::Game {
            return;
        }
        let Some(bot) = self.s.bot.as_ref() else { return };
        let (r, c) = bot.choose();
        self.on_data(json!({"t":"act","kind":"shot","at":[r,c],"cells":[[r,c]]}));
    }

    /* =========================================================
       Placement
       ========================================================= */

    fn enter_placement(&mut self) {
        self.s.phase = Phase::Place;
        self.s.sel = 0;
        self.s.preview = None;
        self.view = View::Place;
        self.place_msg = Default::default();
        if !self.s.vs_bot {
            self.set_awake(true);
        }
        self.play("start");
    }

    pub fn next_unplaced(&self) -> i32 {
        self.s.ships.iter().position(|s| s.cells.is_none()).map_or(-1, |i| i as i32)
    }

    /// Cases de l'aperçu (fantôme) du bateau sélectionné, et validité de la position.
    pub fn preview_info(&self) -> Option<(Vec<Cell>, bool)> {
        let p = self.s.preview?;
        if self.s.sel < 0 {
            return None;
        }
        let s = &self.s.ships[self.s.sel as usize];
        let occ = self.occupancy();
        let mut cells = vec![];
        let mut valid = true;
        for (rr, cc) in ship_cells(p.r, p.c, s.len, p.h) {
            if !inside(rr, cc) {
                valid = false;
                continue;
            }
            if occ[rr as usize * N + cc as usize] != -1 {
                valid = false;
            }
            cells.push((rr as usize, cc as usize));
        }
        Some((cells, valid))
    }

    fn place_hint(&mut self, text: String, kind: MsgKind) {
        self.place_msg = (text, kind);
    }

    fn settle_lifted(&mut self) {
        let Some(l) = self.s.lifted.clone() else { return };
        if self.s.ships[l.idx].cells.is_none() {
            let pv = if self.s.sel == l.idx as i32 { self.preview_info() } else { None };
            self.s.ships[l.idx].cells = Some(match pv {
                Some((cells, true)) => cells,
                _ => l.cells,
            });
        }
        self.s.lifted = None;
        self.s.preview = None;
    }

    pub fn select_ship(&mut self, i: usize) {
        if self.s.me_ready {
            return;
        }
        if self.s.lifted.as_ref().is_some_and(|l| l.idx == i) {
            return;
        }
        self.settle_lifted();
        self.s.sel = i as i32;
        if let Some(cells) = self.s.ships[i].cells.clone() {
            // Bateau déjà posé : on le soulève pour pouvoir le déplacer, le pivoter ou le retirer.
            let h = cells.len() < 2 || cells[0].0 == cells[1].0;
            self.s.preview = Some(Preview { r: cells[0].0 as i32, c: cells[0].1 as i32, h });
            self.s.horiz = h;
            self.s.lifted = Some(Lifted { idx: i, cells });
            self.s.ships[i].cells = None;
            let t = self.tf("pSelPlaced", &[("ship", self.ship_name(i))]);
            self.place_hint(t, MsgKind::Info);
        } else {
            self.s.preview = None;
            let t = self.tf("pSelNew", &[("ship", self.ship_name(i))]);
            self.place_hint(t, MsgKind::Info);
        }
        self.play("click");
    }

    pub fn confirm_placement(&mut self) {
        let Some((cells, valid)) = self.preview_info() else { return };
        if !valid {
            let t = self.t("pBad");
            self.place_hint(t, MsgKind::Err);
            return;
        }
        let i = self.s.sel as usize;
        self.s.ships[i].cells = Some(cells);
        self.s.lifted = None;
        self.s.preview = None;
        self.s.sel = self.next_unplaced();
        if self.s.sel >= 0 {
            let t = self.tf("pPlacedNext", &[("ship", self.ship_name(i)), ("next", self.ship_name(self.s.sel as usize))]);
            self.place_hint(t, MsgKind::Info);
        } else {
            let t = self.t("pDone");
            self.place_hint(t, MsgKind::Ok);
        }
        self.play("place");
    }

    pub fn tap_place_cell(&mut self, r: usize, c: usize) {
        if self.s.me_ready {
            return;
        }
        if let Some((cells, _)) = self.preview_info() {
            if cells.contains(&(r, c)) {
                self.confirm_placement(); // cliquer le fantôme le pose
                return;
            }
        }
        let idx = self.occupancy()[r * N + c];
        if idx >= 0 {
            self.select_ship(idx as usize);
            return;
        }
        let sel_is_lifted = self.s.lifted.as_ref().is_some_and(|l| l.idx as i32 == self.s.sel);
        if self.s.sel < 0 || (self.s.ships[self.s.sel as usize].cells.is_some() && !sel_is_lifted) {
            self.s.sel = self.next_unplaced();
            if self.s.sel < 0 {
                let t = self.t("pAllPlaced");
                self.place_hint(t, MsgKind::Info);
                return;
            }
        }
        let h = self.s.preview.map_or(self.s.horiz, |p| p.h);
        self.s.preview = Some(Preview { r: r as i32, c: c as i32, h });
        let valid = self.preview_info().is_some_and(|p| p.1);
        let t = self.t(if valid { "pTapGhost" } else { "pNoFit" });
        self.place_hint(t, if valid { MsgKind::Info } else { MsgKind::Err });
        self.play("click");
    }

    pub fn rotate_ship(&mut self) {
        if self.s.me_ready {
            return;
        }
        if let Some(mut p) = self.s.preview {
            p.h = !p.h;
            self.s.preview = Some(p);
            if !self.preview_info().is_some_and(|x| x.1) {
                // s'il sort de la grille en pivotant, on le recale à l'intérieur
                let len = self.s.ships[self.s.sel as usize].len as i32;
                if p.h {
                    p.c = p.c.min(N as i32 - len);
                } else {
                    p.r = p.r.min(N as i32 - len);
                }
                self.s.preview = Some(p);
            }
            let valid = self.preview_info().is_some_and(|x| x.1);
            let t = if valid { String::new() } else { self.t("pBlocked") };
            self.place_hint(t, if valid { MsgKind::Info } else { MsgKind::Err });
        }
        self.s.horiz = self.s.preview.map_or(!self.s.horiz, |p| p.h);
        self.play("click");
    }

    pub fn remove_ship(&mut self) {
        if self.s.me_ready {
            return;
        }
        let Some(l) = self.s.lifted.take() else { return };
        self.s.preview = None;
        let t = self.tf("pRemoved", &[("ship", self.ship_name(l.idx))]);
        self.place_hint(t, MsgKind::Info);
    }

    pub fn clear_fleet(&mut self) {
        if self.s.me_ready {
            return;
        }
        for s in &mut self.s.ships {
            s.cells = None;
        }
        self.s.lifted = None;
        self.s.preview = None;
        self.s.sel = 0;
        let t = self.t("pCleared");
        self.place_hint(t, MsgKind::Info);
    }

    pub fn random_fleet(&mut self) {
        if self.s.me_ready {
            return;
        }
        self.s.lifted = None;
        self.s.preview = None;
        for (i, cells) in random_cells(&LENS).into_iter().enumerate() {
            self.s.ships[i].cells = Some(cells);
        }
        self.s.sel = -1;
        let t = self.t("pRandom");
        self.place_hint(t, MsgKind::Info);
        self.play("place");
    }

    pub fn set_ready(&mut self) {
        if self.s.me_ready || self.next_unplaced() != -1 || self.s.lifted.is_some() {
            return;
        }
        self.s.me_ready = true;
        self.s.sel = -1;
        self.s.preview = None;
        self.send(json!({"t":"ready"}));
        self.play("click");
        self.maybe_start();
    }

    // Glisser-déposer (souris) sur la grille de placement.
    pub fn place_down(&mut self, r: usize, c: usize) {
        if self.s.me_ready {
            return;
        }
        let on_preview = self.preview_info().is_some_and(|(cells, _)| cells.contains(&(r, c)));
        let idx = self.occupancy()[r * N + c];
        let mut d = Drag { r, c, moved: false, ship: false, was_selected: on_preview, off: (0, 0) };
        if idx >= 0 && !on_preview {
            self.select_ship(idx as usize); // soulève le bateau touché
            d.ship = true;
        } else if on_preview {
            d.ship = true;
        }
        if d.ship {
            if let Some(p) = self.s.preview {
                d.off = (r as i32 - p.r, c as i32 - p.c);
            }
        }
        self.s.drag = Some(d);
    }

    pub fn place_move(&mut self, r: usize, c: usize) {
        let Some(d) = self.s.drag.as_mut() else { return };
        if !d.ship || self.s.preview.is_none() {
            return;
        }
        if r == d.r && c == d.c && !d.moved {
            return;
        }
        d.moved = true;
        let off = d.off;
        if let Some(p) = self.s.preview.as_mut() {
            let (nr, nc) = (r as i32 - off.0, c as i32 - off.1);
            if p.r == nr && p.c == nc {
                return;
            }
            p.r = nr;
            p.c = nc;
        }
        let valid = self.preview_info().is_some_and(|x| x.1);
        let t = self.t(if valid { "pRelease" } else { "pNotHere" });
        self.place_hint(t, if valid { MsgKind::Info } else { MsgKind::Err });
    }

    pub fn place_up(&mut self) {
        let Some(d) = self.s.drag.take() else { return };
        if d.ship && d.moved {
            if self.preview_info().is_some_and(|x| x.1) {
                self.confirm_placement();
                return;
            }
            let i = self.s.sel; // position refusée : on remet le bateau à sa place d'origine
            if let Some(l) = self.s.lifted.take() {
                self.s.ships[l.idx].cells = Some(l.cells);
                self.s.preview = None;
            }
            let name = if i >= 0 { self.ship_name(i as usize) } else { String::new() };
            let t = self.tf("pRestored", &[("ship", name)]);
            self.place_hint(t, MsgKind::Err);
            return;
        }
        if d.ship && !d.was_selected {
            return; // simple clic sur un bateau posé : il reste sélectionné
        }
        self.tap_place_cell(d.r, d.c);
    }

    fn maybe_start(&mut self) {
        if self.s.phase != Phase::Place || !self.s.me_ready || !self.s.opp_ready {
            return;
        }
        self.s.phase = Phase::Game;
        self.s.my_turn = self.s.is_host;
        self.s.m.start = Some(Instant::now());
        self.view = View::Game;
        let t = self.t(if self.s.is_host { "startHost" } else { "startGuest" });
        self.set_log(t, false);
        self.play(if self.s.my_turn { "turn" } else { "start" });
    }

    /* =========================================================
       Partie
       ========================================================= */

    pub fn flash_hint(&mut self, text: String) {
        self.flash = Some((text, Instant::now() + Duration::from_millis(1800)));
    }
    fn set_log(&mut self, text: String, from_opp: bool) {
        self.s.log = (text, from_opp);
    }

    pub fn on_enemy_tap(&mut self, r: usize, c: usize) {
        if self.s.phase != Phase::Game {
            return;
        }
        if !self.s.my_turn || self.s.waiting {
            let t = self.t(if self.s.waiting { "fFiring" } else { "fWaitTurn" });
            self.flash_hint(t);
            return;
        }
        let (ri, ci) = (r as i32, c as i32);
        let cells: Vec<Cell> = match self.s.mode {
            Mode::Shield => {
                let t = self.t("fShieldHere");
                self.flash_hint(t);
                return;
            }
            Mode::Shot => {
                if !self.targetable(r * N + c) {
                    let t = self.t("fAlready");
                    self.flash_hint(t);
                    return;
                }
                vec![(r, c)]
            }
            Mode::Radar => {
                let v: Vec<Cell> = [(ri, ci), (ri, ci + 1), (ri + 1, ci), (ri + 1, ci + 1)]
                    .into_iter()
                    .filter(|&(a, b)| inside(a, b) && self.s.enemy[a as usize * N + b as usize].is_none())
                    .map(|(a, b)| (a as usize, b as usize))
                    .collect();
                if v.is_empty() {
                    let t = self.t("fNoUnknown");
                    self.flash_hint(t);
                    return;
                }
                self.s.inv.radar -= 1;
                self.s.m.upgrades += 1;
                v
            }
            Mode::Cross => {
                let v: Vec<Cell> = [(ri, ci), (ri - 1, ci), (ri + 1, ci), (ri, ci - 1), (ri, ci + 1)]
                    .into_iter()
                    .filter(|&(a, b)| inside(a, b) && self.targetable(a as usize * N + b as usize))
                    .map(|(a, b)| (a as usize, b as usize))
                    .collect();
                if v.is_empty() {
                    let t = self.t("fAllKnown");
                    self.flash_hint(t);
                    return;
                }
                self.s.inv.cross -= 1;
                self.s.m.upgrades += 1;
                v
            }
        };
        let kind = match self.s.mode {
            Mode::Radar => ActKind::Radar,
            Mode::Cross => ActKind::Cross,
            _ => ActKind::Shot,
        };
        self.s.pending_act = Some((kind, (r, c)));
        self.play(if kind == ActKind::Radar { "radar" } else { "shot" });
        self.send(json!({"t":"act","kind":kind.as_str(),"at":[r,c],"cells":cells_json(&cells)}));
        self.s.waiting = true;
        self.s.mode = Mode::Shot;
    }

    pub fn on_my_tap(&mut self, r: usize, c: usize) {
        if self.s.phase != Phase::Game || self.s.mode != Mode::Shield || !self.s.my_turn || self.s.waiting {
            return;
        }
        let idx = self.occupancy()[r * N + c];
        if idx < 0 {
            let t = self.t("fTouchShip");
            self.flash_hint(t);
            return;
        }
        let idx = idx as usize;
        if self.is_sunk(idx) {
            let t = self.t("fShipSunk");
            self.flash_hint(t);
            return;
        }
        if self.s.ships[idx].shield != Shield::None {
            let t = self.t("fHasShield");
            self.flash_hint(t);
            return;
        }
        self.s.ships[idx].shield = Shield::Active;
        self.s.inv.shield -= 1;
        self.s.m.upgrades += 1;
        self.s.mode = Mode::Shot;
        self.s.my_turn = false;
        self.send(json!({"t":"pass"}));
        self.play("shield");
        let t = self.tf("shieldPlaced", &[("ship", self.ship_name(idx))]);
        self.set_log(t, false);
    }

    pub fn set_mode(&mut self, m: Mode) {
        if self.s.phase != Phase::Game || !self.s.my_turn || self.s.waiting {
            return;
        }
        let have = match m {
            Mode::Shot => 1,
            Mode::Radar => self.s.inv.radar,
            Mode::Cross => self.s.inv.cross,
            Mode::Shield => self.s.inv.shield,
        };
        if have == 0 {
            return;
        }
        self.s.mode = if self.s.mode == m { Mode::Shot } else { m };
        self.play("click");
    }

    fn summarize(&self, kind: ActKind, at: Cell, results: &[ShotRes]) -> String {
        let l = self.lang();
        let cell = game::cell_name(at.0, at.1);
        if kind == ActKind::Radar {
            return i18n::sum_radar(l, &cell, results.iter().filter(|x| x.v == "occ").count(), results.len());
        }
        let sunk_names: Vec<String> = results
            .iter()
            .filter(|x| x.v == "sunk")
            .map(|x| x.si.map_or("?".to_string(), |i| self.ship_name(i)))
            .collect();
        if kind == ActKind::Shot {
            let v = results.first().map_or("", |x| x.v.as_str());
            return i18n::sum_shot(l, &cell, v, sunk_names.first().map_or("", |s| s.as_str()));
        }
        i18n::sum_cross(
            l,
            &cell,
            results.iter().filter(|x| x.v == "hit" || x.v == "sunk").count(),
            results.iter().filter(|x| x.v == "miss").count(),
            results.iter().filter(|x| x.v == "blocked").count(),
            &sunk_names.join(tr(l, "and")),
            sunk_names.len(),
        )
    }

    fn play_result_sound(&mut self, results: &[ShotRes]) {
        let has = |v: &str| results.iter().any(|x| x.v == v);
        if has("sunk") {
            self.play("sunk");
        } else if has("hit") {
            self.play("hit");
        } else if has("blocked") {
            self.play("blocked");
        } else if has("miss") {
            self.play("miss");
        }
    }

    // Le défenseur calcule le résultat sur sa propre grille.
    fn on_act(&mut self, m: &Json) {
        if self.s.phase != Phase::Game || self.s.my_turn || self.s.waiting {
            return; // ce n'est pas le tour de l'adversaire
        }
        let kind = ActKind::parse(m["kind"].as_str().unwrap_or("shot"));
        let cells = sanitize_cells(&m["cells"]);
        let (results, over) = resolve_on(&mut self.s.ships, &mut self.s.incoming, kind, &cells);
        self.send(json!({"t":"res","kind":kind.as_str(),"at":m["at"],"results":results.iter().map(ShotRes::to_json).collect::<Vec<_>>(),"over":over}));
        let at = sanitize_cells(&json!([m["at"]])).first().copied().or(cells.first().copied()).unwrap_or((0, 0));
        self.s.last_in = if kind == ActKind::Radar { vec![] } else { cells.iter().map(|&(r, c)| r * N + c).collect() };
        let name = self.s.opp.as_ref().map_or("?".to_string(), |o| o.name.clone());
        let t = self.tf("oppPrefix", &[("name", name)]) + &self.summarize(kind, at, &results);
        self.set_log(t, true);
        if kind == ActKind::Radar {
            self.play("radar");
        } else {
            self.play_result_sound(&results);
        }
        if over {
            self.end_game(false);
            return;
        }
        self.s.my_turn = true;
        self.s.waiting = false;
        self.after_ms(500, Timer::TurnSound);
    }

    // L'attaquant reçoit le résultat.
    fn on_res(&mut self, m: &Json) {
        if self.s.phase != Phase::Game || !self.s.waiting {
            return;
        }
        let results: Vec<ShotRes> = m["results"].as_array().map(|a| a.iter().filter_map(ShotRes::from_json).collect()).unwrap_or_default();
        let act = self.s.pending_act.unwrap_or_else(|| {
            (ActKind::parse(m["kind"].as_str().unwrap_or("shot")), sanitize_cells(&json!([m["at"]])).first().copied().unwrap_or((0, 0)))
        });
        let mut sunk = 0;
        for x in &results {
            if x.v == "sunk" {
                for &(r, c) in x.ship.as_deref().unwrap_or(&[(x.r, x.c)]) {
                    self.s.enemy[r * N + c] = Some(Mark::Sunk);
                }
                if let Some(si) = x.si {
                    if !self.s.enemy_sunk.contains(&si) {
                        self.s.enemy_sunk.push(si);
                    }
                }
                sunk += 1;
            } else {
                let st = match x.v.as_str() {
                    "miss" => Some(Mark::Miss),
                    "hit" => Some(Mark::Hit),
                    "blocked" => Some(Mark::Blocked),
                    "occ" => Some(Mark::ScanOcc),
                    "empty" => Some(Mark::ScanEmpty),
                    _ => None,
                };
                if st.is_some() {
                    self.s.enemy[x.r * N + x.c] = st;
                }
            }
        }
        if act.0 != ActKind::Radar {
            self.s.m.shots += results.len() as u32;
            self.s.m.hits += results.iter().filter(|x| x.v == "hit" || x.v == "sunk").count() as u32;
            self.s.m.sunk += sunk;
            self.play_result_sound(&results);
        }
        let t = self.summarize(act.0, act.1, &results);
        self.set_log(t, false);
        self.s.waiting = false;
        self.s.pending_act = None;
        if m["over"].as_bool().unwrap_or(false) {
            self.end_game(true);
            return;
        }
        self.s.my_turn = false;
        self.s.choices += sunk;
        if self.s.choices > 0 {
            self.after_ms(500, Timer::OpenChoice);
        }
    }

    fn on_pass(&mut self) {
        if self.s.phase != Phase::Game || self.s.my_turn || self.s.waiting {
            return;
        }
        self.s.my_turn = true;
        self.s.last_in.clear();
        let name = self.s.opp.as_ref().map_or("?".to_string(), |o| o.name.clone());
        let t = self.tf("oppShield", &[("name", name)]);
        self.set_log(t, true);
        self.play("turn");
    }

    pub fn pick_upgrade(&mut self, k: Mode) {
        match k {
            Mode::Radar => self.s.inv.radar += 1,
            Mode::Cross => self.s.inv.cross += 1,
            Mode::Shield => self.s.inv.shield += 1,
            Mode::Shot => {}
        }
        self.s.choices = self.s.choices.saturating_sub(1);
        self.play("upgrade");
        self.s.show_choice = self.s.phase == Phase::Game && self.s.choices > 0;
    }

    fn end_game(&mut self, win: bool) {
        self.s.phase = Phase::Over;
        self.s.my_turn = false;
        self.s.waiting = false;
        self.cancel(|t| matches!(t, Timer::BotEmit(_) | Timer::BotMove));
        self.s.show_choice = false;
        let shots = self.s.m.shots;
        let acc = if shots > 0 { (self.s.m.hits as f64 / shots as f64 * 100.0).round() as u32 } else { 0 };
        let text = format!("{} {}", self.t(if win { "winText" } else { "loseText" }), i18n::end_stats(self.lang(), shots as usize, acc));
        let rec = self.record_game(win);
        let xp = rec.and_then(|(gain, lvl, up)| {
            let a = self.me()?;
            let lo = accounts::xp_for(lvl);
            let hi = accounts::xp_for(lvl + 1);
            Some((gain, lvl, up, a.xp - lo, hi - lo))
        });
        self.s.end = Some(EndInfo {
            icon: if win { "🏆" } else { "🌊" },
            title: self.t(if win { "win" } else { "lose" }),
            text,
            guest: xp.is_none(),
            xp,
            bot_note: self.s.vs_bot,
        });
        self.after_ms(600, Timer::EndSound(win));
        self.after_ms(700, Timer::EndShow);
        self.set_awake(false);
    }

    /* =========================================================
       Boucle : événements réseau, minuteurs, liaison
       ========================================================= */

    pub fn tick(&mut self, ctx: &egui::Context) {
        while let Ok(e) = self.ev_rx.try_recv() {
            self.handle_net(e);
        }
        let now = Instant::now();
        // Minuteurs arrivés à échéance.
        let mut due = vec![];
        let mut i = 0;
        while i < self.timers.len() {
            if self.timers[i].0 <= now {
                due.push(self.timers.remove(i).1);
            } else {
                i += 1;
            }
        }
        for t in due {
            self.run_timer(t);
        }
        // Recherche aléatoire : compte à rebours.
        if let Some(d) = self.s.mm_deadline {
            if self.s.connected {
                self.s.mm_deadline = None;
            } else if now >= d {
                self.stop_search();
                self.lobby_msg = (self.t("noOneFound"), MsgKind::Ok);
                self.after_ms(1000, Timer::StartBot);
            }
        }
        // Liaison : ping régulier et détection de coupure.
        if self.s.connected && !self.s.vs_bot && !self.s.lost && self.s.phase != Phase::Over {
            if now >= self.s.next_ping {
                self.s.next_ping = now + HEARTBEAT;
                self.send(json!({"t":"ping"}));
            }
            let limit = if self.s.opp_away { AWAY_TIMEOUT } else { TIMEOUT };
            if now.duration_since(self.s.last_seen) > limit {
                self.lost();
            }
        }
        // Fenêtre réduite = « l'appli est en arrière-plan ».
        let minimized = ctx.input(|i| i.viewport().minimized.unwrap_or(false));
        if minimized != self.was_minimized {
            self.was_minimized = minimized;
            self.on_visibility(minimized);
        }
        if let Some((_, until)) = &self.flash {
            if now >= *until {
                self.flash = None;
            }
        }
        // Prochain réveil.
        let mut next = Duration::from_millis(500);
        if let Some(t) = self.timers.iter().map(|(t, _)| *t).min() {
            next = next.min(t.saturating_duration_since(now));
        }
        if self.s.mm_deadline.is_some() {
            next = next.min(Duration::from_millis(250));
        }
        ctx.request_repaint_after(next);
    }

    fn run_timer(&mut self, t: Timer) {
        match t {
            Timer::BotEmit(m) => {
                if self.s.vs_bot && self.s.phase != Phase::Over {
                    self.on_data(m);
                }
            }
            Timer::BotMove => {
                if self.s.vs_bot && self.s.phase != Phase::Over {
                    self.bot_move();
                }
            }
            Timer::TurnSound => {
                if self.s.my_turn && self.s.phase == Phase::Game {
                    self.play("turn");
                }
            }
            Timer::OpenChoice => self.s.show_choice = self.s.phase == Phase::Game && self.s.choices > 0,
            Timer::EndSound(win) => self.play(if win { "win" } else { "lose" }),
            Timer::EndShow => self.s.end_show = true,
            Timer::StartBot => {
                if !self.s.connected {
                    self.start_bot();
                }
            }
            Timer::JoinTimeout => {
                let t = self.t("errJoinTimeout");
                self.join_fail(t);
            }
            Timer::SlotTimeout(g) => {
                if g == self.s.slot_gen && !self.s.connected {
                    if let Some(PeerRole::SlotJoiner(k)) = self.s.role {
                        self.slot_next(k);
                    }
                }
            }
            Timer::TrySlot(k) => self.try_slot(k),
            Timer::Reconnect(pid) => {
                if !self.s.connected && self.s.peer == Some(pid) {
                    self.net.reconnect(pid);
                }
            }
            Timer::BusyClose(c) => {
                self.net.close_conn(c);
                self.s.reject_conns.retain(|x| *x != c);
            }
        }
    }

    pub fn countdown_left(&self) -> Option<u64> {
        self.s.mm_deadline.map(|d| (d.saturating_duration_since(Instant::now()).as_millis() as u64).div_ceil(1000))
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.set_awake(false);
    }
}
