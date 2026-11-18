//! Règles du jeu : grille, flotte, résolution des tirs, bot tactique.

use rand::Rng;
use serde_json::{json, Value as Json};

pub const N: usize = 10;
pub const ROWS: &[u8] = b"ABCDEFGHIJ";
pub const LENS: [usize; 5] = [5, 4, 3, 3, 2];
pub const ALPHA: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

pub type Cell = (usize, usize);

pub fn cell_name(r: usize, c: usize) -> String {
    format!("{}{}", ROWS[r] as char, c + 1)
}
pub fn inside(r: i32, c: i32) -> bool {
    r >= 0 && r < N as i32 && c >= 0 && c < N as i32
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Shield {
    None,
    Active,
    Used,
}

#[derive(Clone, Debug)]
pub struct Ship {
    pub len: usize,
    pub cells: Option<Vec<Cell>>,
    pub shield: Shield,
}

impl Ship {
    pub fn fleet() -> Vec<Ship> {
        LENS.iter().map(|&len| Ship { len, cells: None, shield: Shield::None }).collect()
    }
}

/// Ce qu'on voit d'une case.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mark {
    Miss,
    Hit,
    Sunk,
    Blocked,
    ScanOcc,
    ScanEmpty,
}

pub fn occ_of(ships: &[Ship]) -> [i32; N * N] {
    let mut m = [-1i32; N * N];
    for (i, s) in ships.iter().enumerate() {
        if let Some(cells) = &s.cells {
            for &(r, c) in cells {
                m[r * N + c] = i as i32;
            }
        }
    }
    m
}

pub fn sunk_of(s: &Ship, incoming: &[Option<Mark>]) -> bool {
    s.cells.as_ref().is_some_and(|cells| cells.iter().all(|&(r, c)| incoming[r * N + c] == Some(Mark::Hit)))
}

pub fn ship_cells(r: i32, c: i32, len: usize, h: bool) -> Vec<(i32, i32)> {
    (0..len as i32).map(|i| if h { (r, c + i) } else { (r + i, c) }).collect()
}

pub fn random_cells(lens: &[usize]) -> Vec<Vec<Cell>> {
    let mut rng = rand::thread_rng();
    loop {
        let mut ships: Vec<Ship> = lens.iter().map(|&len| Ship { len, cells: None, shield: Shield::None }).collect();
        let mut ok = true;
        for i in 0..ships.len() {
            let mut placed = false;
            for _ in 0..3000 {
                let occ = occ_of(&ships);
                let h = rng.gen_bool(0.5);
                let (r, c) = (rng.gen_range(0..N) as i32, rng.gen_range(0..N) as i32);
                let cells = ship_cells(r, c, ships[i].len, h);
                if cells.iter().all(|&(a, b)| inside(a, b) && occ[a as usize * N + b as usize] == -1) {
                    ships[i].cells = Some(cells.iter().map(|&(a, b)| (a as usize, b as usize)).collect());
                    placed = true;
                    break;
                }
            }
            if !placed {
                ok = false;
                break;
            }
        }
        if ok {
            return ships.into_iter().map(|s| s.cells.unwrap()).collect();
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ActKind {
    Shot,
    Radar,
    Cross,
}
impl ActKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ActKind::Shot => "shot",
            ActKind::Radar => "radar",
            ActKind::Cross => "cross",
        }
    }
    pub fn parse(s: &str) -> ActKind {
        match s {
            "radar" => ActKind::Radar,
            "cross" => ActKind::Cross,
            _ => ActKind::Shot,
        }
    }
}

/// Résultat d'un tir sur une case (tel qu'envoyé sur le réseau).
#[derive(Clone, Debug)]
pub struct ShotRes {
    pub r: usize,
    pub c: usize,
    pub v: String, // miss | hit | sunk | blocked | occ | empty
    pub ship: Option<Vec<Cell>>,
    pub si: Option<usize>,
}

impl ShotRes {
    pub fn to_json(&self) -> Json {
        let mut o = json!({ "r": self.r, "c": self.c, "v": self.v });
        if let Some(sh) = &self.ship {
            o["ship"] = json!(sh.iter().map(|&(r, c)| json!([r, c])).collect::<Vec<_>>());
        }
        if let Some(si) = self.si {
            o["si"] = json!(si);
        }
        o
    }
    pub fn from_json(j: &Json) -> Option<ShotRes> {
        let r = j["r"].as_i64()?;
        let c = j["c"].as_i64()?;
        if !inside(r as i32, c as i32) {
            return None;
        }
        let ship = j["ship"].as_array().map(|a| {
            a.iter()
                .filter_map(|p| {
                    let (r, c) = (p[0].as_i64()?, p[1].as_i64()?);
                    inside(r as i32, c as i32).then_some((r as usize, c as usize))
                })
                .collect()
        });
        Some(ShotRes {
            r: r as usize,
            c: c as usize,
            v: j["v"].as_str().unwrap_or("").to_string(),
            ship,
            si: j["si"].as_u64().map(|x| x as usize).filter(|&x| x < 5),
        })
    }
}

pub fn sanitize_cells(j: &Json) -> Vec<Cell> {
    j.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|p| {
                    let (r, c) = (p.get(0)?.as_i64()?, p.get(1)?.as_i64()?);
                    inside(r as i32, c as i32).then_some((r as usize, c as usize))
                })
                .take(5)
                .collect()
        })
        .unwrap_or_default()
}

pub fn cells_json(cells: &[Cell]) -> Json {
    json!(cells.iter().map(|&(r, c)| json!([r, c])).collect::<Vec<_>>())
}

/// Calcule le résultat de tirs sur une flotte (la sienne ou celle du bot).
pub fn resolve_on(ships: &mut [Ship], incoming: &mut [Option<Mark>], kind: ActKind, cells: &[Cell]) -> (Vec<ShotRes>, bool) {
    let occ = occ_of(ships);
    let mut results = vec![];
    for &(r, c) in cells {
        let k = r * N + c;
        let i = occ[k];
        let mk = |v: &str| ShotRes { r, c, v: v.into(), ship: None, si: None };
        if kind == ActKind::Radar {
            results.push(mk(if i >= 0 { "occ" } else { "empty" }));
            continue;
        }
        match incoming[k] {
            Some(Mark::Miss) => {
                results.push(mk("miss"));
                continue;
            }
            Some(Mark::Hit) => {
                results.push(mk("hit"));
                continue;
            }
            _ => {}
        }
        if i < 0 {
            incoming[k] = Some(Mark::Miss);
            results.push(mk("miss"));
            continue;
        }
        let s = &mut ships[i as usize];
        if s.shield == Shield::Active {
            s.shield = Shield::Used;
            incoming[k] = Some(Mark::Blocked);
            results.push(mk("blocked"));
            continue;
        }
        incoming[k] = Some(Mark::Hit);
        if sunk_of(s, incoming) {
            results.push(ShotRes { r, c, v: "sunk".into(), ship: s.cells.clone(), si: Some(i as usize) });
        } else {
            results.push(mk("hit"));
        }
    }
    let over = ships.iter().all(|s| sunk_of(s, incoming));
    (results, over)
}

/* =========================================================
   Bot tactique : pour chaque case, il compte combien de positions
   de bateaux restants y passent (en privilégiant celles qui prolongent
   un coup au but), puis tire sur la case la plus probable.
   ========================================================= */
pub struct Bot {
    pub ships: Vec<Ship>,
    pub incoming: Vec<Option<Mark>>,
    know: Vec<Option<Mark>>,
    remaining: Vec<usize>,
    /// Niveau : 0 facile, 1 moyen, 2 difficile.
    skill: u8,
}

impl Bot {
    pub fn new(skill: u8) -> Bot {
        let ships = random_cells(&LENS)
            .into_iter()
            .zip(LENS)
            .map(|(cells, len)| Ship { len, cells: Some(cells), shield: Shield::None })
            .collect();
        Bot { ships, incoming: vec![None; N * N], know: vec![None; N * N], remaining: LENS.to_vec(), skill }
    }

    /// Le bot apprend le résultat de son propre tir.
    pub fn learn(&mut self, results: &[ShotRes]) {
        for x in results {
            let k = x.r * N + x.c;
            match x.v.as_str() {
                "sunk" => {
                    let ship = x.ship.clone().unwrap_or_default();
                    for &(r, c) in &ship {
                        self.know[r * N + c] = Some(Mark::Sunk);
                    }
                    if let Some(i) = self.remaining.iter().position(|&l| l == ship.len()) {
                        self.remaining.remove(i);
                    }
                }
                "miss" => self.know[k] = Some(Mark::Miss),
                "hit" => self.know[k] = Some(Mark::Hit),
                "blocked" => self.know[k] = Some(Mark::Blocked),
                _ => {}
            }
        }
    }

    fn random_unknown(&self) -> Cell {
        let free: Vec<usize> = (0..N * N).filter(|&k| self.know[k].is_none()).collect();
        let k = free[rand::thread_rng().gen_range(0..free.len().max(1)).min(free.len().saturating_sub(1))];
        (k / N, k % N)
    }

    pub fn choose(&self) -> Cell {
        let k_ = &self.know;
        let mut rng = rand::thread_rng();
        if self.skill == 0 {
            // facile : tirs au hasard, avec un peu de bon sens autour d'un touché
            let mut near = vec![];
            for k in 0..N * N {
                if k_[k] != Some(Mark::Hit) {
                    continue;
                }
                let (r, c) = ((k / N) as i32, (k % N) as i32);
                for (a, b) in [(r - 1, c), (r + 1, c), (r, c - 1), (r, c + 1)] {
                    if inside(a, b) && k_[a as usize * N + b as usize].is_none() {
                        near.push((a as usize, b as usize));
                    }
                }
            }
            if !near.is_empty() && rng.gen::<f64>() < 0.6 {
                return near[rng.gen_range(0..near.len())];
            }
            return self.random_unknown();
        }
        if self.skill == 1 && rng.gen::<f64>() < 0.3 {
            return self.random_unknown();
        }
        if let Some(k) = k_.iter().position(|v| *v == Some(Mark::Blocked)) {
            return (k / N, k % N); // bouclier : on retire au même endroit
        }
        let has_hits = k_.contains(&Some(Mark::Hit));
        let mut score = vec![0f64; N * N];
        for &len in &self.remaining {
            for r in 0..N as i32 {
                for c in 0..N as i32 {
                    for h in [true, false] {
                        let mut ok = true;
                        let mut nh = 0;
                        let mut cells = vec![];
                        for (rr, cc) in ship_cells(r, c, len, h) {
                            if !inside(rr, cc) {
                                ok = false;
                                break;
                            }
                            let k = rr as usize * N + cc as usize;
                            match k_[k] {
                                Some(Mark::Miss) | Some(Mark::Sunk) => {
                                    ok = false;
                                    break;
                                }
                                Some(Mark::Hit) => nh += 1,
                                _ => {}
                            }
                            cells.push(k);
                        }
                        if !ok || (has_hits && nh == 0) {
                            continue;
                        }
                        let w = if has_hits { 12f64.powi(nh) } else { 1.0 };
                        for k in cells {
                            if k_[k].is_none() {
                                score[k] += w;
                            }
                        }
                    }
                }
            }
        }
        let mut best = -1.0;
        let mut best_k: Option<usize> = None;
        for k in 0..N * N {
            if k_[k].is_some() {
                continue;
            }
            let parity = if !has_hits && (k / N + k % N) % 2 == 0 { 1.25 } else { 1.0 };
            let v = score[k] * parity + rng.gen::<f64>() * 0.5;
            if v > best {
                best = v;
                best_k = Some(k);
            }
        }
        let k = best_k.or_else(|| k_.iter().position(|v| v.is_none())).unwrap_or(0);
        (k / N, k % N)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_fleet_is_valid() {
        for _ in 0..50 {
            let f = random_cells(&LENS);
            let mut seen = std::collections::HashSet::new();
            for (cells, len) in f.iter().zip(LENS) {
                assert_eq!(cells.len(), len);
                for c in cells {
                    assert!(seen.insert(*c));
                }
            }
        }
    }

    #[test]
    fn bot_finishes_a_game() {
        for skill in 0..=2 {
            let mut target = Bot::new(2);
            let shooter = &mut Bot::new(skill);
            let mut done = false;
            for _ in 0..100 {
                let at = shooter.choose();
                let (res, over) = resolve_on(&mut target.ships, &mut target.incoming, ActKind::Shot, &[at]);
                shooter.learn(&res);
                if over {
                    done = true;
                    break;
                }
            }
            assert!(done, "le bot de niveau {skill} n'a pas fini en 100 tirs");
        }
    }
}

/* =========================================================
   Anti-triche (même protocole que la version web)
   Chaque joueur décide seul si un tir l'a touché : un client modifié pourrait mentir. En se déclarant
   prêt, il envoie donc l'empreinte SHA-256 de sa grille (avec un sel aléatoire). En fin de partie,
   chacun dévoile sa grille et son sel ; l'autre contrôle l'empreinte, la validité de la flotte et que
   chaque résultat annoncé correspond bien à cette grille.
   ========================================================= */

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn board_hash(salt: &str, ships: &[Vec<Cell>]) -> String {
    use sha2::{Digest, Sha256};
    let board = ships
        .iter()
        .map(|cells| cells.iter().map(|&(r, c)| (r * N + c).to_string()).collect::<Vec<_>>().join(","))
        .collect::<Vec<_>>()
        .join(";");
    let lens = LENS.iter().map(|l| l.to_string()).collect::<Vec<_>>().join(",");
    hex(&Sha256::digest(format!("c02-naval-v1|{N}|{lens}|{salt}|{board}").as_bytes()))
}

pub fn new_salt() -> String {
    let mut b = [0u8; 16];
    rand::Rng::fill(&mut rand::thread_rng(), &mut b);
    hex(&b)
}

pub fn is_hash(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Contrôle la grille dévoilée par l'adversaire. `true` = honnête.
pub fn check_reveal(m: &Json, commit: &str, enemy: &[Option<Mark>], blocked_seen: u32, opp_shields: u32) -> bool {
    let Some(salt) = m["salt"].as_str().filter(|s| s.len() == 32 && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))) else {
        return false;
    };
    let Some(raw) = m["ships"].as_array().filter(|a| a.len() == LENS.len()) else { return false };
    let mut occ = [false; N * N];
    let mut ships: Vec<Vec<Cell>> = vec![];
    for (i, rs) in raw.iter().enumerate() {
        let Some(rs) = rs.as_array().filter(|a| a.len() == LENS[i]) else { return false };
        let mut cells: Vec<Cell> = vec![];
        for p in rs {
            let (Some(r), Some(c)) = (p[0].as_i64(), p[1].as_i64()) else { return false };
            if !inside(r as i32, c as i32) {
                return false;
            }
            cells.push((r as usize, c as usize));
        }
        let horiz = cells.iter().all(|p| p.0 == cells[0].0);
        let vert = cells.iter().all(|p| p.1 == cells[0].1);
        if !horiz && !vert {
            return false;
        }
        let mut pos: Vec<usize> = cells.iter().map(|p| if horiz { p.1 } else { p.0 }).collect();
        pos.sort_unstable();
        if pos.windows(2).any(|w| w[1] != w[0] + 1) {
            return false;
        }
        for &(r, c) in &cells {
            if occ[r * N + c] {
                return false;
            }
            occ[r * N + c] = true;
        }
        ships.push(cells);
    }
    if board_hash(salt, &ships) != commit || blocked_seen > opp_shields {
        return false;
    }
    for k in 0..N * N {
        let has = occ[k];
        match enemy[k] {
            Some(Mark::Miss) | Some(Mark::ScanEmpty) if has => return false,
            Some(Mark::Hit) | Some(Mark::Sunk) | Some(Mark::Blocked) | Some(Mark::ScanOcc) if !has => return false,
            _ => {}
        }
    }
    // un bateau annoncé coulé l'est en entier
    !ships.iter().any(|cells| {
        cells.iter().any(|&(r, c)| enemy[r * N + c] == Some(Mark::Sunk)) && !cells.iter().all(|&(r, c)| enemy[r * N + c] == Some(Mark::Sunk))
    })
}

/* ---------- XP (même formule que js/profile.js et server/app.js) ---------- */

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct XpParts {
    pub base: u64,
    pub hits: u64,
    pub sunk: u64,
    pub acc: u64,
    pub time: u64,
}

impl XpParts {
    pub fn sum(&self) -> u64 {
        self.base + self.hits + self.sunk + self.acc + self.time
    }
}

/// Détail de l'XP et gain : participation/victoire, touchés, bateaux coulés, précision, rapidité (victoire).
pub fn xp_parts(win: bool, shots: u32, hits: u32, sunk: u32, duration: u64, half: bool) -> (XpParts, u64) {
    let acc = if shots > 0 { hits as f64 / shots as f64 } else { 0.0 };
    let p = XpParts {
        base: if win { 100 } else { 30 },
        hits: hits as u64 * 5,
        sunk: sunk as u64 * 20,
        acc: (acc * if win { 100.0 } else { 30.0 }).round() as u64,
        time: if win { (60.0 * ((900.0 - duration as f64) / 720.0).clamp(0.0, 1.0)).round() as u64 } else { 0 },
    };
    let sum = p.sum();
    (p, if half { (sum as f64 / 2.0).round() as u64 } else { sum })
}

#[cfg(test)]
mod anticheat_tests {
    use super::*;

    fn fleet() -> Vec<Vec<Cell>> {
        random_cells(&LENS)
    }

    #[test]
    fn random_fleets_are_contiguous_and_ascending() {
        for _ in 0..200 {
            let f = fleet();
            for (i, cells) in f.iter().enumerate() {
                assert_eq!(cells.len(), LENS[i]);
                let horiz = cells.iter().all(|c| c.0 == cells[0].0);
                let key: Vec<usize> = cells.iter().map(|c| if horiz { c.1 } else { c.0 }).collect();
                assert!(key.windows(2).all(|w| w[1] == w[0] + 1), "{cells:?}");
            }
        }
    }

    #[test]
    fn board_hash_matches_web_vector() {
        // Vecteur calculé avec la version web (crypto.subtle) : voir tests/game.test.js (« c02-naval-v1 »).
        let ships: Vec<Vec<Cell>> = vec![
            (0..5).map(|c| (0, c)).collect(),
            (0..4).map(|c| (1, c)).collect(),
            (0..3).map(|c| (2, c)).collect(),
            (0..3).map(|c| (3, c)).collect(),
            (0..2).map(|c| (4, c)).collect(),
        ];
        let salt = "00112233445566778899aabbccddeeff";
        let h = board_hash(salt, &ships);
        assert_eq!(h.len(), 64);
        assert_eq!(h, include_str!("../testdata/hash_vector.txt").trim());
    }

    #[test]
    fn honest_reveal_passes_and_lies_fail() {
        let ships = fleet();
        let salt = new_salt();
        let commit = board_hash(&salt, &ships);
        let reveal = json!({"t":"reveal","salt":salt,"ships":ships.iter().map(|c| cells_json(c)).collect::<Vec<_>>()});
        let mut enemy = vec![None; N * N];
        assert!(check_reveal(&reveal, &commit, &enemy, 0, 0));
        // un « raté » sur une case occupée est un mensonge
        let (r, c) = ships[0][0];
        enemy[r * N + c] = Some(Mark::Miss);
        assert!(!check_reveal(&reveal, &commit, &enemy, 0, 0));
        enemy[r * N + c] = None;
        // un « bloqué » sans bouclier annoncé aussi
        assert!(!check_reveal(&reveal, &commit, &enemy, 1, 0));
        // une grille différente de l'empreinte
        assert!(!check_reveal(&reveal, &"0".repeat(64), &enemy, 0, 0));
    }

    #[test]
    fn xp_formula() {
        // victoire : 100 + 17 touchés × 5 + 5 coulés × 20 + précision 17/40 × 100 + rapidité (10 min : 60 × 300/720)
        let (p, gain) = xp_parts(true, 40, 17, 5, 600, false);
        assert_eq!((p.base, p.hits, p.sunk, p.acc, p.time), (100, 85, 100, 43, 25));
        assert_eq!(gain, 353);
        let (_, half) = xp_parts(true, 40, 17, 5, 600, true);
        assert_eq!(half, 177);
        let (p, gain) = xp_parts(false, 10, 2, 0, 600, false);
        assert_eq!((p.base, p.hits, p.acc, p.time), (30, 10, 6, 0));
        assert_eq!(gain, 46);
    }
}
