//! Comptes locaux (pseudo, mot de passe, photo, XP, statistiques) et réglages.
//! Enregistrés dans %APPDATA%\C02 Naval\ : un fichier JSON par compte.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::i18n::Lang;

pub fn data_dir() -> PathBuf {
    // Dossier de données remplaçable (tests, version portable sur clé USB).
    if let Some(d) = std::env::var_os("C02NAVAL_DATA") {
        return PathBuf::from(d);
    }
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("C02 Naval")
}
fn acc_dir() -> PathBuf {
    data_dir().join("accounts")
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Settings {
    pub sound: bool,
    pub lang: Lang,
    #[serde(default)]
    pub current: Option<String>, // dernier compte connecté ("guest" = invité)
    #[serde(default)]
    pub theme: Option<String>, // thème choisi (None = automatique : Rétro Noël en décembre, Océan sinon)
    #[serde(default = "yes")]
    pub bot: bool, // adversaire bot autorisé (partie contre le bot et bot de secours du mode aléatoire)
    #[serde(default = "skill_default")]
    pub skill: u8, // niveau du bot : 0 facile, 1 moyen, 2 difficile
    #[serde(default)]
    pub turn_url: String, // relais TURN personnel (facultatif)
    #[serde(default)]
    pub turn_user: String,
    #[serde(default)]
    pub turn_cred: String,
}

fn yes() -> bool {
    true
}
fn skill_default() -> u8 {
    2
}

impl Settings {
    pub fn load() -> Settings {
        fs::read_to_string(data_dir().join("settings.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(Settings { sound: true, lang: Lang::detect(), current: None, theme: None, bot: true, skill: 2, turn_url: String::new(), turn_user: String::new(), turn_cred: String::new() })
    }
    pub fn save(&self) {
        let _ = fs::create_dir_all(data_dir());
        let _ = fs::write(data_dir().join("settings.json"), serde_json::to_string_pretty(self).unwrap_or_default());
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PassHash {
    pub salt: String,
    pub hash: String,
    pub iter: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Stats {
    pub games: u32,
    pub wins: u32,
    pub losses: u32,
    pub interrupted: u32,
    pub streak: u32,
    pub best_streak: u32,
    pub shots: u32,
    pub hits: u32,
    pub sunk: u32,
    pub upgrades: u32,
    pub fastest_win: Option<u32>,
    pub time_played: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub name: String,
    pub pass: Option<PassHash>,
    pub photo: Option<String>, // data:image/jpeg;base64,… (même format que la version web)
    pub xp: u64,
    #[serde(default)]
    pub created: String,
    #[serde(default)]
    pub last_login: String,
    #[serde(default)]
    pub stats: Stats,
}

pub fn now_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

/// XP total pour atteindre le niveau l : 0, 100, 300, 600, 1000…
pub fn xp_for(l: u64) -> u64 {
    50 * l * (l - 1)
}
pub fn level_of(xp: u64) -> u64 {
    let mut l = 1;
    while xp >= xp_for(l + 1) {
        l += 1;
    }
    l
}

pub fn valid_photo(p: &str) -> bool {
    let ok_prefix = ["data:image/jpeg;base64,", "data:image/png;base64,", "data:image/webp;base64,"];
    p.len() < 90000
        && ok_prefix.iter().any(|pre| {
            p.starts_with(pre)
                && p[pre.len()..].bytes().all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/' || b == b'=')
        })
}

#[derive(Default)]
pub struct Store {
    pub accounts: HashMap<String, Account>,
    pub storage_ok: bool,
}

impl Store {
    pub fn load(&mut self) {
        let mut out = HashMap::new();
        match fs::read_dir(acc_dir()) {
            Ok(rd) => {
                for e in rd.flatten() {
                    if e.path().extension().is_some_and(|x| x == "json") {
                        if let Some(a) = fs::read_to_string(e.path()).ok().and_then(|s| serde_json::from_str::<Account>(s.trim_start_matches('\u{feff}')).ok()) {
                            if !a.id.is_empty() && !a.name.is_empty() {
                                out.insert(a.id.clone(), a);
                            }
                        }
                    }
                }
                self.storage_ok = true;
            }
            Err(_) => self.storage_ok = fs::create_dir_all(acc_dir()).is_ok(),
        }
        self.accounts = out;
    }

    pub fn save(&mut self, a: &Account) {
        self.accounts.insert(a.id.clone(), a.clone());
        let ok = fs::create_dir_all(acc_dir()).is_ok()
            && fs::write(acc_dir().join(format!("{}.json", a.id)), serde_json::to_string_pretty(a).unwrap_or_default()).is_ok();
        self.storage_ok = ok;
    }

    pub fn delete(&mut self, id: &str) {
        let _ = fs::remove_file(acc_dir().join(format!("{id}.json")));
        self.accounts.remove(id);
    }

    pub fn name_taken(&self, name: &str, except: Option<&str>) -> bool {
        self.accounts
            .values()
            .any(|a| Some(a.id.as_str()) != except && a.name.to_lowercase() == name.to_lowercase())
    }

    pub fn sorted(&self) -> Vec<Account> {
        let mut v: Vec<Account> = self.accounts.values().cloned().collect();
        v.sort_by(|a, b| b.last_login.cmp(&a.last_login));
        v
    }
}

pub fn new_id() -> String {
    let ms = chrono::Utc::now().timestamp_millis() as u64;
    let mut r = rand::thread_rng();
    let tail: String = (0..4).map(|_| char::from_digit(r.gen_range(0..36), 36).unwrap()).collect();
    format!("a{}{}", radix36(ms), tail)
}
fn radix36(mut n: u64) -> String {
    let mut s = vec![];
    while n > 0 {
        s.push(char::from_digit((n % 36) as u32, 36).unwrap());
        n /= 36;
    }
    s.iter().rev().collect()
}

/// Mot de passe : PBKDF2-SHA-256 salé, 120 000 itérations (jamais stocké en clair).
pub fn hash_pass(pw: &str, salt_b64: Option<&str>, iter: u32) -> PassHash {
    let salt: Vec<u8> = match salt_b64.and_then(|s| B64.decode(s).ok()) {
        Some(s) => s,
        None => {
            let mut s = [0u8; 16];
            rand::thread_rng().fill(&mut s);
            s.to_vec()
        }
    };
    let mut out = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(pw.as_bytes(), &salt, iter, &mut out);
    PassHash { salt: B64.encode(&salt), hash: B64.encode(out), iter }
}

pub fn check_pass(pw: &str, h: &PassHash) -> bool {
    hash_pass(pw, Some(&h.salt), h.iter).hash == h.hash
}

/// Photo : recadrée en carré 160 × 160 et compressée en JPEG (quelques Ko).
pub fn make_photo(path: &std::path::Path) -> Option<String> {
    let img = image::open(path).ok()?;
    let s = img.width().min(img.height());
    let crop = img.crop_imm((img.width() - s) / 2, (img.height() - s) / 2, s, s);
    let small = crop.resize_exact(160, 160, image::imageops::FilterType::Lanczos3).to_rgb8();
    let mut buf = Vec::new();
    let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 82);
    enc.encode_image(&small).ok()?;
    let url = format!("data:image/jpeg;base64,{}", B64.encode(&buf));
    valid_photo(&url).then_some(url)
}

/// Décode une photo « data URL » en pixels RGBA.
pub fn decode_photo(url: &str) -> Option<(usize, usize, Vec<u8>)> {
    if !valid_photo(url) {
        return None;
    }
    let b64 = url.split_once(',')?.1;
    let bytes = B64.decode(b64).ok()?;
    let img = image::load_from_memory(&bytes).ok()?.to_rgba8();
    Some((img.width() as usize, img.height() as usize, img.into_raw()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn levels() {
        assert_eq!(level_of(0), 1);
        assert_eq!(level_of(99), 1);
        assert_eq!(level_of(100), 2);
        assert_eq!(level_of(300), 3);
    }
    #[test]
    fn pbkdf2_known_vector() {
        // Vecteur RFC 7914 (PBKDF2-HMAC-SHA256, "passwd"/"salt", 1 itération).
        let h = hash_pass("passwd", Some(&B64.encode(b"salt")), 1);
        let raw = B64.decode(h.hash).unwrap();
        assert_eq!(&raw[..8], &[0x55, 0xac, 0x04, 0x6e, 0x56, 0xe3, 0x08, 0x9f]);
    }
}
