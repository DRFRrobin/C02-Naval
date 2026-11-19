//! Comptes en ligne : même API que la version web (server/app.js), pseudo unique et profil partagés.
//!
//! Chaque appel part dans un fil d'exécution séparé ; le résultat revient par un canal (`CloudEvent`),
//! pour que l'interface ne se fige jamais.

use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value as Json};

use crate::accounts::Stats;

const DEFAULT_API: &str = "https://api.c02.app";

/// Adresse de l'API (variable `C02NAVAL_API` pour les tests).
pub fn api_url() -> String {
    std::env::var("C02NAVAL_API").unwrap_or_else(|_| DEFAULT_API.to_string()).trim_end_matches('/').to_string()
}

#[derive(Debug, Clone, PartialEq)]
pub struct CloudErr {
    /// Code d'erreur du serveur (`bad_credentials`, `pseudo_taken`…) ou `network` / `server_error`.
    pub code: String,
    pub status: u16,
    pub retry_after: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct Profile {
    pub name: String,
    pub emoji: Option<String>,
    pub xp: u64,
    pub stats: Stats,
}

impl Profile {
    fn from_json(j: &Json) -> Option<Profile> {
        Some(Profile {
            name: j["name"].as_str()?.to_string(),
            emoji: j["emoji"].as_str().map(|s| s.to_string()),
            xp: j["xp"].as_u64()?,
            stats: serde_json::from_value(j["stats"].clone()).unwrap_or_default(),
        })
    }
}

#[derive(Debug)]
pub enum CloudEvent {
    /// Connexion ou création de compte : jeton et profil.
    Auth(Result<(String, Profile), CloudErr>),
    /// Synchronisation : nombre de parties en attente envoyées, puis profil à jour.
    Synced(usize, Result<Profile, CloudErr>),
    Password(Result<(), CloudErr>),
    Deleted(Result<(), CloudErr>),
    Health(bool),
}

fn agent() -> Result<ureq::Agent, CloudErr> {
    let tls = native_tls::TlsConnector::new().map_err(|_| net_err())?;
    Ok(ureq::AgentBuilder::new()
        .tls_connector(Arc::new(tls))
        .timeout_connect(Duration::from_secs(6))
        .timeout_read(Duration::from_secs(12))
        .user_agent("c02-naval")
        .build())
}

fn net_err() -> CloudErr {
    CloudErr { code: "network".into(), status: 0, retry_after: None }
}

/// Un appel à l'API : `Ok(corps JSON)` ou l'erreur du serveur.
pub fn call(method: &str, path: &str, body: Option<&Json>, token: Option<&str>) -> Result<Json, CloudErr> {
    let url = format!("{}{path}", api_url());
    let mut req = agent()?.request(method, &url);
    if let Some(t) = token {
        req = req.set("authorization", &format!("Bearer {t}"));
    }
    let res = match body {
        Some(b) => req.send_json(b.clone()),
        None => req.call(),
    };
    match res {
        Ok(r) => r.into_json::<Json>().map_err(|_| CloudErr { code: "server_error".into(), status: 200, retry_after: None }),
        Err(ureq::Error::Status(status, r)) => {
            let j: Json = r.into_json().unwrap_or(Json::Null);
            Err(CloudErr {
                code: j["error"].as_str().unwrap_or("server_error").to_string(),
                status,
                retry_after: j["retryAfter"].as_u64(),
            })
        }
        Err(_) => Err(net_err()),
    }
}

fn auth_result(j: Json) -> Result<(String, Profile), CloudErr> {
    let bad = || CloudErr { code: "server_error".into(), status: 200, retry_after: None };
    let tok = j["token"].as_str().ok_or_else(bad)?.to_string();
    Ok((tok, Profile::from_json(&j["profile"]).ok_or_else(bad)?))
}

/// Lance `f` dans un fil et envoie son résultat, puis réveille l'interface.
fn spawn(tx: &Sender<CloudEvent>, ctx: &eframe::egui::Context, f: impl FnOnce() -> CloudEvent + Send + 'static) {
    let (tx, ctx) = (tx.clone(), ctx.clone());
    std::thread::spawn(move || {
        let _ = tx.send(f());
        ctx.request_repaint();
    });
}

pub fn auth(tx: &Sender<CloudEvent>, ctx: &eframe::egui::Context, register: bool, name: String, password: String, import: Option<Json>) {
    spawn(tx, ctx, move || {
        let mut body = json!({"name": name, "password": password});
        if let (true, Some(p)) = (register, import) {
            body["profile"] = p; // un profil local du même pseudo est repris (borné par le serveur)
        }
        let path = if register { "/api/register" } else { "/api/login" };
        CloudEvent::Auth(call("POST", path, Some(&body), None).and_then(auth_result))
    });
}

/// Envoie les parties en attente (dans l'ordre) puis recharge le profil. `pending` : (type, corps) ; une
/// partie refusée (400) est abandonnée. Renvoie le nombre d'éléments traités.
pub fn sync(tx: &Sender<CloudEvent>, ctx: &eframe::egui::Context, token: String, pending: Vec<(String, Json)>) {
    spawn(tx, ctx, move || {
        let mut done = 0;
        for (kind, body) in &pending {
            let path = if kind == "i" { "/api/interrupted" } else { "/api/result" };
            match call("POST", path, Some(body), Some(&token)) {
                Ok(_) => done += 1,
                Err(e) if e.status == 400 => done += 1,
                Err(e) => return CloudEvent::Synced(done, Err(e)),
            }
        }
        let me = call("GET", "/api/me", None, Some(&token)).and_then(|j| {
            Profile::from_json(&j["profile"]).ok_or(CloudErr { code: "server_error".into(), status: 200, retry_after: None })
        });
        CloudEvent::Synced(done, me)
    });
}

pub fn change_password(tx: &Sender<CloudEvent>, ctx: &eframe::egui::Context, token: String, old: String, new: String) {
    spawn(tx, ctx, move || {
        CloudEvent::Password(call("POST", "/api/password", Some(&json!({"password": old, "newPassword": new})), Some(&token)).map(|_| ()))
    });
}

pub fn delete(tx: &Sender<CloudEvent>, ctx: &eframe::egui::Context, token: String, password: String) {
    spawn(tx, ctx, move || CloudEvent::Deleted(call("POST", "/api/delete", Some(&json!({"password": password})), Some(&token)).map(|_| ())));
}

pub fn set_emoji(token: String, emoji: Option<String>) {
    std::thread::spawn(move || {
        let _ = call("PATCH", "/api/me", Some(&json!({"emoji": emoji})), Some(&token));
    });
}

pub fn health(tx: &Sender<CloudEvent>, ctx: &eframe::egui::Context) {
    spawn(tx, ctx, || CloudEvent::Health(call("GET", "/api/health", None, None).is_ok()));
}

/// Clé de texte (`e_…`) pour une erreur d'API.
pub fn err_key(e: &CloudErr) -> String {
    match e.code.as_str() {
        "bad_credentials" | "locked" | "pseudo_taken" | "pseudo_invalid" | "pseudo_reserved" | "password_invalid" | "rate_limited" | "network" => {
            format!("e_{}", e.code)
        }
        _ => "e_server_error".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::Mutex;

    /// Un faux serveur HTTP : répond `reply` (état + corps) à la prochaine requête et la renvoie.
    fn fake_server(status: &str, body: &str) -> (String, std::thread::JoinHandle<String>) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", l.local_addr().unwrap());
        let (status, body) = (status.to_string(), body.to_string());
        let h = std::thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            let mut buf = vec![0u8; 8192];
            let n = s.read(&mut buf).unwrap();
            let req = String::from_utf8_lossy(&buf[..n]).to_string();
            let _ = write!(s, "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len());
            req
        });
        (url, h)
    }

    // l'adresse de l'API est une variable d'environnement : les tests passent un par un
    static LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn login_ok_and_errors() {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (url, h) = fake_server(
            "200 OK",
            r#"{"token":"abc","profile":{"name":"Léa","emoji":"🐙","xp":120,"created":1700000000000,"stats":{"games":3,"wins":2,"bestStreak":2}}}"#,
        );
        std::env::set_var("C02NAVAL_API", &url);
        let r = call("POST", "/api/login", Some(&json!({"name":"Léa","password":"motdepasse"})), None).and_then(auth_result).unwrap();
        let req = h.join().unwrap();
        assert!(req.starts_with("POST /api/login"));
        assert!(req.contains("\"password\":\"motdepasse\""));
        assert_eq!((r.0.as_str(), r.1.name.as_str(), r.1.xp, r.1.stats.wins, r.1.stats.best_streak), ("abc", "Léa", 120, 2, 2));
        assert_eq!(r.1.emoji.as_deref(), Some("🐙"));

        let (url, h) = fake_server("401 Unauthorized", r#"{"error":"bad_credentials"}"#);
        std::env::set_var("C02NAVAL_API", &url);
        let e = call("POST", "/api/login", Some(&json!({})), None).unwrap_err();
        h.join().unwrap();
        assert_eq!((e.code.as_str(), e.status), ("bad_credentials", 401));
        assert_eq!(err_key(&e), "e_bad_credentials");

        let (url, h) = fake_server("429 Too Many Requests", r#"{"error":"locked","retryAfter":600}"#);
        std::env::set_var("C02NAVAL_API", &url);
        let e = call("POST", "/api/login", Some(&json!({})), None).unwrap_err();
        h.join().unwrap();
        assert_eq!((e.code.as_str(), e.retry_after), ("locked", Some(600)));
    }

    #[test]
    fn bearer_token_and_unreachable_api() {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (url, h) = fake_server("200 OK", r#"{"ok":true}"#);
        std::env::set_var("C02NAVAL_API", &url);
        call("GET", "/api/me", None, Some("tok123")).unwrap();
        assert!(h.join().unwrap().to_lowercase().contains("authorization: bearer tok123"));
        std::env::set_var("C02NAVAL_API", "http://127.0.0.1:1");
        assert_eq!(call("GET", "/api/health", None, None).unwrap_err().code, "network");
        assert_eq!(err_key(&CloudErr { code: "weird".into(), status: 500, retry_after: None }), "e_server_error");
    }
}
