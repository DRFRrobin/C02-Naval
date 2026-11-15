//! Mise à jour obligatoire depuis les « releases » GitHub.
//!
//! Au démarrage, on lit la liste des releases du dépôt. Si une version Windows plus
//! récente existe, le jeu la télécharge, remplace son propre .exe et redémarre.
//! Sans connexion (ou si GitHub ne répond pas), on joue avec la version actuelle.

use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const REPO: &str = "DRFRrobin/C02-Naval";
pub const RELEASES_URL: &str = "https://github.com/DRFRrobin/C02-Naval/releases";

#[derive(Clone, Debug, PartialEq)]
pub enum UpdState {
    Checking,
    UpToDate,
    /// Mise à jour trouvée : téléchargement en cours (octets reçus, taille totale).
    Downloading { version: String, got: u64, total: u64 },
    Restarting,
    Failed { version: String, error: String },
}

pub type Shared = Arc<Mutex<UpdState>>;

pub fn current_version() -> String {
    // C02NAVAL_FAKE_VERSION : pour tester la mise à jour avec un ancien numéro.
    std::env::var("C02NAVAL_FAKE_VERSION").unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string())
}

/// « v1.2.0-windows » -> [1, 2, 0]
pub fn parse_version(tag: &str) -> Vec<u32> {
    let core = tag.trim_start_matches(['v', 'V']).split(['-', '+']).next().unwrap_or("");
    core.split('.').map(|p| p.parse().unwrap_or(0)).collect()
}

pub fn is_newer(remote: &str, local: &str) -> bool {
    let (mut a, mut b) = (parse_version(remote), parse_version(local));
    let n = a.len().max(b.len());
    a.resize(n, 0);
    b.resize(n, 0);
    a > b
}

fn agent() -> Result<ureq::Agent, String> {
    let tls = native_tls::TlsConnector::new().map_err(|e| e.to_string())?;
    Ok(ureq::AgentBuilder::new()
        .tls_connector(Arc::new(tls))
        .timeout_connect(Duration::from_secs(6))
        .timeout_read(Duration::from_secs(30))
        .user_agent("c02-naval-updater")
        .build())
}

/// Lance la vérification en tâche de fond.
pub fn start(ctx: eframe::egui::Context) -> Shared {
    let st: Shared = Arc::new(Mutex::new(UpdState::Checking));
    cleanup_old();
    if std::env::var_os("C02NAVAL_NO_UPDATE").is_some() {
        *st.lock().unwrap() = UpdState::UpToDate;
        return st;
    }
    let st2 = st.clone();
    std::thread::spawn(move || {
        let set = |s: UpdState| {
            *st2.lock().unwrap() = s;
            ctx.request_repaint();
        };
        let found = match find_update() {
            Ok(Some(x)) => x,
            _ => {
                set(UpdState::UpToDate); // pas de mise à jour, ou GitHub injoignable
                return;
            }
        };
        let (version, url) = found;
        set(UpdState::Downloading { version: version.clone(), got: 0, total: 0 });
        match install(&url, |got, total| set(UpdState::Downloading { version: version.clone(), got, total })) {
            Ok(exe) => {
                set(UpdState::Restarting);
                std::thread::sleep(Duration::from_millis(600));
                if std::process::Command::new(&exe).env_remove("C02NAVAL_FAKE_VERSION").spawn().is_ok() {
                    std::process::exit(0);
                }
                set(UpdState::Failed { version, error: "restart".into() });
            }
            Err(e) => set(UpdState::Failed { version, error: e }),
        }
    });
    st
}

/// Cherche la release Windows la plus récente : (version, URL du .exe) si plus récente que la nôtre.
fn find_update() -> Result<Option<(String, String)>, String> {
    let url = format!("https://api.github.com/repos/{REPO}/releases?per_page=30");
    let list: serde_json::Value = agent()?
        .get(&url)
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| e.to_string())?
        .into_json()
        .map_err(|e| e.to_string())?;
    let local = current_version();
    let mut best: Option<(String, String)> = None;
    for r in list.as_array().into_iter().flatten() {
        if r["draft"].as_bool().unwrap_or(false) {
            continue;
        }
        let tag = r["tag_name"].as_str().unwrap_or("");
        if !tag.ends_with("-windows") || !is_newer(tag, &local) {
            continue;
        }
        let asset = r["assets"].as_array().into_iter().flatten().find(|a| {
            let n = a["name"].as_str().unwrap_or("").to_lowercase();
            n.ends_with(".exe") && n.contains("windows")
        });
        let Some(dl) = asset.and_then(|a| a["browser_download_url"].as_str()) else { continue };
        if best.as_ref().map_or(true, |(v, _)| is_newer(tag, v)) {
            best = Some((tag.to_string(), dl.to_string()));
        }
    }
    Ok(best.map(|(tag, dl)| (parse_version(&tag).iter().map(|x| x.to_string()).collect::<Vec<_>>().join("."), dl)))
}

fn old_path(exe: &std::path::Path) -> PathBuf {
    exe.with_extension("old")
}

/// Supprime l'ancien exécutable laissé par une mise à jour précédente.
fn cleanup_old() {
    if let Ok(exe) = std::env::current_exe() {
        let old = old_path(&exe);
        for _ in 0..10 {
            if !old.exists() || std::fs::remove_file(&old).is_ok() {
                break;
            }
            std::thread::sleep(Duration::from_millis(300)); // l'ancien processus se ferme encore
        }
    }
}

/// Télécharge le nouvel exécutable et le met à la place de l'actuel.
fn install(url: &str, progress: impl Fn(u64, u64)) -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let new = exe.with_extension("new");
    let resp = agent()?.get(url).call().map_err(|e| e.to_string())?;
    let total: u64 = resp.header("Content-Length").and_then(|s| s.parse().ok()).unwrap_or(0);
    let mut reader = resp.into_reader();
    let mut data = Vec::with_capacity(total as usize);
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        data.extend_from_slice(&buf[..n]);
        progress(data.len() as u64, total);
    }
    // Contrôles : taille annoncée et en-tête d'exécutable Windows (« MZ »).
    if (total > 0 && data.len() as u64 != total) || data.len() < 1_000_000 || &data[..2] != b"MZ" {
        return Err("download".into());
    }
    std::fs::write(&new, &data).map_err(|e| e.to_string())?;
    // Windows autorise à renommer un .exe en cours d'exécution (mais pas à l'écraser).
    let old = old_path(&exe);
    let _ = std::fs::remove_file(&old);
    std::fs::rename(&exe, &old).map_err(|e| e.to_string())?;
    if let Err(e) = std::fs::rename(&new, &exe) {
        let _ = std::fs::rename(&old, &exe); // on remet l'ancien
        return Err(e.to_string());
    }
    Ok(exe)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn versions() {
        assert_eq!(parse_version("v1.2.0-windows"), vec![1, 2, 0]);
        assert!(is_newer("v1.2.0-windows", "1.1.0"));
        assert!(is_newer("v1.10.0-windows", "1.9.3"));
        assert!(!is_newer("v1.2.0-windows", "1.2.0"));
        assert!(!is_newer("v1.1.0-windows", "1.2.0"));
    }
}
