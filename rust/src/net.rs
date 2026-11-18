//! Réseau pair-à-pair compatible PeerJS 1.5.
//!
//! - Mise en relation : WebSocket vers le service public PeerJS (0.peerjs.com),
//!   exactement comme la version navigateur (même protocole OFFER/ANSWER/CANDIDATE).
//! - Liaison directe : WebRTC (canal de données) via la bibliothèque `webrtc`.
//! - Messages : sérialisation BinaryPack, découpage en morceaux de 16 300 octets.
//!
//! Résultat : la version PC et la version web peuvent jouer ensemble.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use rand::Rng;
use serde_json::{json, Value as Json};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio_tungstenite::tungstenite::Message;
use webrtc::api::APIBuilder;
use webrtc::data_channel::data_channel_init::RTCDataChannelInit;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::data_channel::RTCDataChannel;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::ice_transport::ice_credential_type::RTCIceCredentialType;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;

use crate::binarypack::{self as bp, Value as Bp};

const SIGNAL_URL: &str = "wss://0.peerjs.com:443/peerjs";
const PEERJS_KEY: &str = "peerjs";
const PEERJS_VERSION: &str = "1.5.4";
const CHUNK_MTU: usize = 16300;
const PING_SECS: u64 = 5;

/// Relais TURN personnel facultatif (voir README) : (urls, utilisateur, mot de passe).
const MY_TURN: Option<(&[&str], &str, &str)> = None;

pub type PeerId = u64;
pub type ConnId = u64;

#[derive(Debug, Clone, PartialEq)]
pub enum PeerErr {
    UnavailableId,
    PeerUnavailable,
    Network,
    Other(String),
}

#[derive(Debug)]
pub enum NetEvent {
    PeerOpen(PeerId),
    PeerError(PeerId, PeerErr),
    PeerDisconnected(PeerId),
    /// Un autre joueur se connecte à nous (côté hôte).
    Incoming(PeerId, ConnId),
    ConnOpen(ConnId),
    ConnData(ConnId, Json),
    ConnClosed(ConnId),
}

enum PeerCmd {
    Connect { conn: ConnId, dst: String },
    Send { conn: ConnId, msg: Json },
    CloseConn(ConnId),
    Disconnect,
    Reconnect,
    Destroy,
}

enum Inner {
    Dc { key: String, dc: Arc<RTCDataChannel> },
    Closed(String),
}

#[derive(Clone)]
struct Emitter {
    tx: Sender<NetEvent>,
    ctx: eframe::egui::Context,
}
impl Emitter {
    fn emit(&self, e: NetEvent) {
        let _ = self.tx.send(e);
        self.ctx.request_repaint();
    }
}

/// Journal réseau (activé par la variable d'environnement C02NAVAL_DEBUG=1).
pub fn dlog(s: impl AsRef<str>) {
    if std::env::var_os("C02NAVAL_DEBUG").is_none() {
        return;
    }
    use std::io::Write;
    let path = crate::accounts::data_dir().join("net.log");
    let _ = std::fs::create_dir_all(crate::accounts::data_dir());
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{} {}", chrono::Utc::now().format("%H:%M:%S%.3f"), s.as_ref());
    }
}

/// Relaie les journaux internes de WebRTC vers net.log (mode débogage).
struct NetLogger;
impl log::Log for NetLogger {
    fn enabled(&self, m: &log::Metadata) -> bool {
        m.level() <= log::Level::Debug && !m.target().starts_with("tungstenite") && !m.target().starts_with("tokio")
    }
    fn log(&self, r: &log::Record) {
        if self.enabled(r.metadata()) {
            dlog(format!("  {} {}: {}", r.level(), r.target(), r.args()));
        }
    }
    fn flush(&self) {}
}
pub fn init_debug_log() {
    if std::env::var_os("C02NAVAL_DEBUG").is_some() {
        static L: NetLogger = NetLogger;
        let _ = log::set_logger(&L);
        log::set_max_level(log::LevelFilter::Debug);
    }
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
fn next_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

pub struct Net {
    rt: tokio::runtime::Runtime,
    em: Emitter,
    peers: HashMap<PeerId, UnboundedSender<PeerCmd>>,
    conn_peer: HashMap<ConnId, PeerId>,
}

impl Net {
    pub fn new(tx: Sender<NetEvent>, ctx: eframe::egui::Context) -> Self {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("tokio");
        init_debug_log();
        Net { rt, em: Emitter { tx, ctx }, peers: HashMap::new(), conn_peer: HashMap::new() }
    }

    /// Crée un pair. `id = None` : identifiant aléatoire (pour rejoindre).
    pub fn create_peer(&mut self, id: Option<String>) -> PeerId {
        let pid = next_id();
        let (tx, rx) = unbounded_channel();
        self.peers.insert(pid, tx);
        let id = id.unwrap_or_else(random_peer_id);
        let em = self.em.clone();
        self.rt.spawn(peer_task(pid, id, rx, em));
        pid
    }

    pub fn connect(&mut self, pid: PeerId, dst: &str) -> ConnId {
        let conn = next_id();
        self.conn_peer.insert(conn, pid);
        if let Some(tx) = self.peers.get(&pid) {
            let _ = tx.send(PeerCmd::Connect { conn, dst: dst.to_string() });
        }
        conn
    }

    /// À appeler quand un événement `Incoming` arrive, pour savoir à quel pair il appartient.
    pub fn adopt(&mut self, pid: PeerId, conn: ConnId) {
        self.conn_peer.insert(conn, pid);
    }

    pub fn send(&self, conn: ConnId, msg: Json) {
        if let Some(tx) = self.conn_peer.get(&conn).and_then(|p| self.peers.get(p)) {
            let _ = tx.send(PeerCmd::Send { conn, msg });
        }
    }

    pub fn close_conn(&mut self, conn: ConnId) {
        if let Some(tx) = self.conn_peer.get(&conn).and_then(|p| self.peers.get(p)) {
            let _ = tx.send(PeerCmd::CloseConn(conn));
        }
        self.conn_peer.remove(&conn);
    }

    /// Libère l'identifiant sur le service de mise en relation (les liaisons directes restent).
    pub fn disconnect(&self, pid: PeerId) {
        if let Some(tx) = self.peers.get(&pid) {
            let _ = tx.send(PeerCmd::Disconnect);
        }
    }

    pub fn reconnect(&self, pid: PeerId) {
        if let Some(tx) = self.peers.get(&pid) {
            let _ = tx.send(PeerCmd::Reconnect);
        }
    }

    pub fn destroy(&mut self, pid: PeerId) {
        if let Some(tx) = self.peers.remove(&pid) {
            let _ = tx.send(PeerCmd::Destroy);
        }
        self.conn_peer.retain(|_, p| *p != pid);
    }

    pub fn destroy_all(&mut self) {
        let ids: Vec<_> = self.peers.keys().copied().collect();
        for p in ids {
            self.destroy(p);
        }
    }
}

impl Drop for Net {
    fn drop(&mut self) {
        self.destroy_all();
    }
}

fn random_peer_id() -> String {
    let mut r = rand::thread_rng();
    (0..4).map(|_| format!("{:08x}", r.gen::<u32>())).collect::<Vec<_>>().join("-")
}

fn random_token() -> String {
    const A: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut r = rand::thread_rng();
    (0..11).map(|_| A[r.gen_range(0..A.len())] as char).collect()
}

/* =========================================================
   Tâche d'un pair : signalisation + connexions
   ========================================================= */

struct Conn {
    id: ConnId,
    pc: Arc<RTCPeerConnection>,
    dc: Option<Arc<RTCDataChannel>>,
    remote_set: bool,
    pending: Vec<RTCIceCandidateInit>,
    peer: String,
    data_count: i64,
}

type Ws = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

async fn open_ws(id: &str, token: &str) -> Option<Ws> {
    let signal = std::env::var("C02NAVAL_SIGNAL").unwrap_or_else(|_| SIGNAL_URL.to_string()); // test : serveur PeerJS local
    let url = format!("{signal}?key={PEERJS_KEY}&id={id}&token={token}&version={PEERJS_VERSION}");
    match tokio::time::timeout(Duration::from_secs(15), tokio_tungstenite::connect_async(url)).await {
        Ok(Ok((ws, _))) => Some(ws),
        _ => None,
    }
}

async fn next_ws(ws: &mut Option<Ws>) -> Option<Result<Message, tokio_tungstenite::tungstenite::Error>> {
    match ws.as_mut() {
        Some(s) => s.next().await,
        None => std::future::pending().await,
    }
}

async fn peer_task(pid: PeerId, id: String, mut rx: UnboundedReceiver<PeerCmd>, em: Emitter) {
    let token = random_token();
    let (out_tx, mut out_rx) = unbounded_channel::<Json>();
    let (in_tx, mut in_rx) = unbounded_channel::<Inner>();
    let mut conns: HashMap<String, Conn> = HashMap::new();
    let mut ws = open_ws(&id, &token).await;
    dlog(format!("[{id}] ws open: {}", ws.is_some()));
    if ws.is_none() {
        em.emit(NetEvent::PeerError(pid, PeerErr::Network));
    }
    let mut ping = tokio::time::interval(Duration::from_secs(PING_SECS));
    let mut opened = false;

    loop {
        tokio::select! {
            msg = next_ws(&mut ws) => {
                match msg {
                    Some(Ok(Message::Text(txt))) => {
                        let Ok(m) = serde_json::from_str::<Json>(&txt) else { continue };
                        let kind = m["type"].as_str().unwrap_or("").to_string();
                        dlog(format!("[{id}] <- {}", if kind == "OFFER" || kind == "ANSWER" { kind.clone() } else { txt.to_string() }));
                        match kind.as_str() {
                            "OPEN" => { opened = true; em.emit(NetEvent::PeerOpen(pid)); }
                            "ID-TAKEN" => { em.emit(NetEvent::PeerError(pid, PeerErr::UnavailableId)); ws = None; }
                            "INVALID-KEY" | "ERROR" => {
                                let t = m["payload"]["msg"].as_str().unwrap_or("server-error").to_string();
                                em.emit(NetEvent::PeerError(pid, PeerErr::Other(t)));
                            }
                            "EXPIRE" => {
                                // Le pair visé n'existe pas (ou ne répond pas).
                                let src = m["src"].as_str().unwrap_or("").to_string();
                                let keys: Vec<String> = conns.iter().filter(|(_, c)| c.peer == src && c.dc.is_none()).map(|(k, _)| k.clone()).collect();
                                for k in keys { if let Some(c) = conns.remove(&k) { let _ = c.pc.close().await; } }
                                em.emit(NetEvent::PeerError(pid, PeerErr::PeerUnavailable));
                            }
                            "LEAVE" => {
                                let src = m["src"].as_str().unwrap_or("").to_string();
                                let keys: Vec<String> = conns.iter().filter(|(_, c)| c.peer == src).map(|(k, _)| k.clone()).collect();
                                for k in keys { close_conn(&mut conns, &k, &em).await; }
                            }
                            "OFFER" => handle_offer(pid, &m, &mut conns, &out_tx, &in_tx, &em).await,
                            "ANSWER" => {
                                let key = m["payload"]["connectionId"].as_str().unwrap_or("").to_string();
                                if let Some(c) = conns.get_mut(&key) {
                                    if let Ok(sd) = serde_json::from_value::<RTCSessionDescription>(m["payload"]["sdp"].clone()) {
                                        let r = c.pc.set_remote_description(sd).await;
                                        if let Err(e) = &r { dlog(format!("answer: set_remote error {e}")); }
                                        if r.is_ok() {
                                            c.remote_set = true;
                                            for cand in c.pending.drain(..) { let _ = c.pc.add_ice_candidate(cand).await; }
                                        }
                                    }
                                }
                            }
                            "CANDIDATE" => {
                                let key = m["payload"]["connectionId"].as_str().unwrap_or("").to_string();
                                dlog(format!("[{id}] cand {}", m["payload"]["candidate"]));
                                if let (Some(c), Ok(cand)) = (conns.get_mut(&key), serde_json::from_value::<RTCIceCandidateInit>(m["payload"]["candidate"].clone())) {
                                    if c.remote_set {
                                        if let Err(e) = c.pc.add_ice_candidate(cand).await { dlog(format!("add cand error {e}")); }
                                    } else { c.pending.push(cand); }
                                }
                            }
                            _ => {}
                        }
                    }
                    Some(Ok(_)) => {}
                    Some(Err(e)) => {
                        dlog(format!("[{id}] ws error {e}"));
                        ws = None;
                        if opened { em.emit(NetEvent::PeerDisconnected(pid)); }
                        else { em.emit(NetEvent::PeerError(pid, PeerErr::Network)); }
                    }
                    None => {
                        dlog(format!("[{id}] ws closed"));
                        ws = None;
                        if opened { em.emit(NetEvent::PeerDisconnected(pid)); }
                        else { em.emit(NetEvent::PeerError(pid, PeerErr::Network)); }
                    }
                }
            }
            out = out_rx.recv() => {
                if let (Some(m), Some(s)) = (out, ws.as_mut()) {
                    dlog(format!("[{id}] -> {}", if m["type"] == "CANDIDATE" { m.to_string() } else { m["type"].to_string() }));
                    let _ = s.send(Message::Text(m.to_string().into())).await;
                }
            }
            inner = in_rx.recv() => {
                match inner {
                    Some(Inner::Dc { key, dc }) => { if let Some(c) = conns.get_mut(&key) { c.dc = Some(dc); } }
                    Some(Inner::Closed(key)) => close_conn(&mut conns, &key, &em).await,
                    None => {}
                }
            }
            _ = ping.tick() => {
                if let Some(s) = ws.as_mut() {
                    let _ = s.send(Message::Text(json!({"type":"HEARTBEAT"}).to_string().into())).await;
                }
            }
            cmd = rx.recv() => {
                match cmd {
                    Some(PeerCmd::Connect { conn, dst }) => {
                        if let Err(e) = start_connect(conn, &dst, &mut conns, &out_tx, &in_tx, &em).await {
                            em.emit(NetEvent::PeerError(pid, PeerErr::Other(e)));
                        }
                    }
                    Some(PeerCmd::Send { conn, msg }) => {
                        if let Some(c) = conns.values_mut().find(|c| c.id == conn) {
                            send_msg(c, &msg).await;
                        }
                    }
                    Some(PeerCmd::CloseConn(conn)) => {
                        if let Some(k) = conns.iter().find(|(_, c)| c.id == conn).map(|(k, _)| k.clone()) {
                            if let Some(c) = conns.remove(&k) {
                                if let Some(dc) = &c.dc { let _ = dc.close().await; }
                                let _ = c.pc.close().await;
                            }
                        }
                    }
                    Some(PeerCmd::Disconnect) => {
                        if let Some(mut s) = ws.take() { let _ = s.close(None).await; }
                    }
                    Some(PeerCmd::Reconnect) => {
                        if ws.is_none() {
                            opened = false;
                            ws = open_ws(&id, &token).await;
                            if ws.is_none() { em.emit(NetEvent::PeerError(pid, PeerErr::Network)); }
                        }
                    }
                    Some(PeerCmd::Destroy) | None => {
                        for (_, c) in conns.drain() {
                            if let Some(dc) = &c.dc { let _ = dc.close().await; }
                            let _ = c.pc.close().await;
                        }
                        if let Some(mut s) = ws.take() { let _ = s.close(None).await; }
                        return;
                    }
                }
            }
        }
    }
}

async fn close_conn(conns: &mut HashMap<String, Conn>, key: &str, em: &Emitter) {
    if let Some(c) = conns.remove(key) {
        let _ = c.pc.close().await;
        em.emit(NetEvent::ConnClosed(c.id));
    }
}

fn ice_config() -> RTCConfiguration {
    let mut servers = vec![
        RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".into(), "stun:stun1.l.google.com:19302".into()],
            ..Default::default()
        },
        RTCIceServer {
            urls: vec!["turn:eu-0.turn.peerjs.com:3478".into(), "turn:us-0.turn.peerjs.com:3478".into()],
            username: "peerjs".into(),
            credential: "peerjsp".into(),
            credential_type: RTCIceCredentialType::Password,
        },
    ];
    if let Some((urls, user, cred)) = MY_TURN {
        servers.push(RTCIceServer {
            urls: urls.iter().map(|s| s.to_string()).collect(),
            username: user.into(),
            credential: cred.into(),
            credential_type: RTCIceCredentialType::Password,
        });
    }
    RTCConfiguration { ice_servers: servers, ..Default::default() }
}

async fn new_pc(
    key: &str,
    peer: &str,
    out_tx: &UnboundedSender<Json>,
    in_tx: &UnboundedSender<Inner>,
) -> Result<Arc<RTCPeerConnection>, String> {
    let api = APIBuilder::new().build();
    let pc = Arc::new(api.new_peer_connection(ice_config()).await.map_err(|e| e.to_string())?);
    // Nos candidats ICE partent vers l'autre joueur via le service de mise en relation.
    {
        let out = out_tx.clone();
        let key = key.to_string();
        let dst = peer.to_string();
        pc.on_ice_candidate(Box::new(move |c| {
            let out = out.clone();
            let key = key.clone();
            let dst = dst.clone();
            Box::pin(async move {
                if let Some(c) = c {
                    if let Ok(init) = c.to_json() {
                        let _ = out.send(json!({
                            "type": "CANDIDATE", "dst": dst,
                            "payload": { "candidate": init, "type": "data", "connectionId": key }
                        }));
                    }
                }
            })
        }));
    }
    {
        let tx = in_tx.clone();
        let key = key.to_string();
        pc.on_peer_connection_state_change(Box::new(move |s| {
            dlog(format!("pc {key}: {s}"));
            if matches!(s, RTCPeerConnectionState::Failed | RTCPeerConnectionState::Closed) {
                let _ = tx.send(Inner::Closed(key.clone()));
            }
            Box::pin(async {})
        }));
    }
    Ok(pc)
}

/// Branche les événements d'un canal de données (ouverture, messages, fermeture).
fn wire_dc(dc: &Arc<RTCDataChannel>, conn: ConnId, key: &str, in_tx: &UnboundedSender<Inner>, em: &Emitter) {
    {
        let em = em.clone();
        dc.on_open(Box::new(move || {
            dlog(format!("dc {conn} open"));
            em.emit(NetEvent::ConnOpen(conn));
            Box::pin(async {})
        }));
    }
    {
        let em = em.clone();
        let chunks: Arc<Mutex<HashMap<i64, (usize, Vec<Option<Vec<u8>>>, usize)>>> = Arc::default();
        dc.on_message(Box::new(move |m: DataChannelMessage| {
            handle_incoming(&m.data, m.is_string, conn, &chunks, &em);
            Box::pin(async {})
        }));
    }
    {
        let tx = in_tx.clone();
        let key = key.to_string();
        dc.on_close(Box::new(move || {
            dlog(format!("dc {key} close"));
            let _ = tx.send(Inner::Closed(key.clone()));
            Box::pin(async {})
        }));
    }
}

type Chunks = Arc<Mutex<HashMap<i64, (usize, Vec<Option<Vec<u8>>>, usize)>>>;

fn handle_incoming(data: &[u8], is_string: bool, conn: ConnId, chunks: &Chunks, em: &Emitter) {
    if is_string {
        // Sérialisation JSON (au cas où l'autre côté l'utiliserait).
        if let Ok(j) = serde_json::from_slice::<Json>(data) {
            em.emit(NetEvent::ConnData(conn, j));
        }
        return;
    }
    let Some(v) = bp::unpack(data) else {
        dlog(format!("dc {conn}: undecodable {} bytes", data.len()));
        return;
    };
    if let Some(pd) = v.get("__peerData") {
        if let Some(t) = pd.get("type") {
            if *t == Bp::Str("close".into()) {
                em.emit(NetEvent::ConnClosed(conn));
            }
            return;
        }
        // Morceau d'un gros message : on recolle les morceaux.
        let (Some(id), Some(n), Some(total), Some(Bp::Bin(part))) =
            (pd.as_i64(), v.get("n").and_then(Bp::as_i64), v.get("total").and_then(Bp::as_i64), v.get("data"))
        else {
            return;
        };
        if total <= 0 || total > 2000 || n < 0 || n >= total {
            return;
        }
        let full = {
            let mut map = chunks.lock().unwrap();
            let e = map.entry(id).or_insert_with(|| (total as usize, vec![None; total as usize], 0));
            if e.1.len() != total as usize || e.1[n as usize].is_some() {
                return;
            }
            e.1[n as usize] = Some(part.clone());
            e.2 += 1;
            if e.2 == e.0 {
                let (_, parts, _) = map.remove(&id).unwrap();
                Some(parts.into_iter().flatten().flatten().collect::<Vec<u8>>())
            } else {
                None
            }
        };
        if let Some(buf) = full {
            handle_incoming(&buf, false, conn, chunks, em);
        }
        return;
    }
    let j = bp::to_json(&v);
    if j["t"] != "ping" {
        dlog(format!("dc {conn} <- {}", j.to_string().chars().take(200).collect::<String>()));
    }
    em.emit(NetEvent::ConnData(conn, j));
}

async fn send_msg(c: &mut Conn, msg: &Json) {
    let Some(dc) = c.dc.clone() else { return };
    let blob = bp::pack(&bp::from_json(msg));
    if blob.len() <= CHUNK_MTU {
        if let Err(e) = dc.send(&Bytes::from(blob)).await {
            dlog(format!("send error {e}"));
        }
        return;
    }
    dlog(format!("send chunked {} bytes as #{}", blob.len(), c.data_count));
    // Numérotation à partir de 1 : PeerJS ignore un numéro 0 (il teste « if (__peerData) »).
    let id = c.data_count;
    c.data_count += 1;
    let total = blob.len().div_ceil(CHUNK_MTU);
    for (n, part) in blob.chunks(CHUNK_MTU).enumerate() {
        let chunk = Bp::Map(vec![
            (Bp::Str("__peerData".into()), Bp::Int(id)),
            (Bp::Str("n".into()), Bp::Int(n as i64)),
            (Bp::Str("total".into()), Bp::Int(total as i64)),
            (Bp::Str("data".into()), Bp::Bin(part.to_vec())),
        ]);
        if let Err(e) = dc.send(&Bytes::from(bp::pack(&chunk))).await {
            dlog(format!("send chunk error {e}"));
        }
    }
}

/// Nous rejoignons quelqu'un : nous créons l'offre WebRTC.
async fn start_connect(
    conn: ConnId,
    dst: &str,
    conns: &mut HashMap<String, Conn>,
    out_tx: &UnboundedSender<Json>,
    in_tx: &UnboundedSender<Inner>,
    em: &Emitter,
) -> Result<(), String> {
    let key = format!("dc_{}", random_token());
    let pc = new_pc(&key, dst, out_tx, in_tx).await?;
    let dc = pc
        .create_data_channel(&key, Some(RTCDataChannelInit { ordered: Some(true), ..Default::default() }))
        .await
        .map_err(|e| e.to_string())?;
    wire_dc(&dc, conn, &key, in_tx, em);
    let offer = pc.create_offer(None).await.map_err(|e| e.to_string())?;
    pc.set_local_description(offer.clone()).await.map_err(|e| e.to_string())?;
    // On impose d'être le client DTLS (« active ») : les navigateurs récents proposent DTLS 1.3,
    // que la bibliothèque WebRTC ne sait pas gérer côté serveur. L'autre côté répondra « passive ».
    let sent = json!({ "type": "offer", "sdp": offer.sdp.replace("a=setup:actpass", "a=setup:active") });
    let _ = out_tx.send(json!({
        "type": "OFFER", "dst": dst,
        "payload": {
            "sdp": sent, "type": "data", "connectionId": key, "label": key,
            "reliable": true, "serialization": "binary", "browser": "rust"
        }
    }));
    conns.insert(
        key,
        Conn { id: conn, pc, dc: Some(dc), remote_set: false, pending: vec![], peer: dst.to_string(), data_count: 1 },
    );
    Ok(())
}

/// Quelqu'un nous rejoint : nous répondons à son offre.
async fn handle_offer(
    pid: PeerId,
    m: &Json,
    conns: &mut HashMap<String, Conn>,
    out_tx: &UnboundedSender<Json>,
    in_tx: &UnboundedSender<Inner>,
    em: &Emitter,
) {
    let p = &m["payload"];
    if p["type"].as_str() != Some("data") {
        return;
    }
    let key = p["connectionId"].as_str().unwrap_or("").to_string();
    let src = m["src"].as_str().unwrap_or("").to_string();
    if key.is_empty() || conns.contains_key(&key) {
        return;
    }
    let sd = match serde_json::from_value::<RTCSessionDescription>(p["sdp"].clone()) {
        Ok(sd) => sd,
        Err(e) => {
            dlog(format!("offer: bad sdp {e}"));
            return;
        }
    };
    let pc = match new_pc(&key, &src, out_tx, in_tx).await {
        Ok(pc) => pc,
        Err(e) => {
            dlog(format!("offer: pc error {e}"));
            return;
        }
    };
    let conn = next_id();
    {
        let tx = in_tx.clone();
        let em2 = em.clone();
        let key2 = key.clone();
        let in_tx2 = in_tx.clone();
        pc.on_data_channel(Box::new(move |dc: Arc<RTCDataChannel>| {
            wire_dc(&dc, conn, &key2, &in_tx2, &em2);
            let _ = tx.send(Inner::Dc { key: key2.clone(), dc });
            Box::pin(async {})
        }));
    }
    if let Err(e) = pc.set_remote_description(sd).await {
        dlog(format!("offer: set_remote error {e}"));
        let _ = pc.close().await;
        return;
    }
    let answer = match pc.create_answer(None).await {
        Ok(a) => a,
        Err(e) => {
            dlog(format!("offer: create_answer error {e}"));
            return;
        }
    };
    if let Err(e) = pc.set_local_description(answer.clone()).await {
        dlog(format!("offer: set_local error {e}"));
        return;
    }
    let _ = out_tx.send(json!({
        "type": "ANSWER", "dst": src,
        "payload": { "sdp": answer, "type": "data", "connectionId": key, "browser": "rust" }
    }));
    conns.insert(key, Conn { id: conn, pc, dc: None, remote_set: true, pending: vec![], peer: src, data_count: 1 });
    em.emit(NetEvent::Incoming(pid, conn));
}
