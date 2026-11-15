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
}

impl Bot {
    pub fn new() -> Bot {
        let ships = random_cells(&LENS)
            .into_iter()
            .zip(LENS)
            .map(|(cells, len)| Ship { len, cells: Some(cells), shield: Shield::None })
            .collect();
        Bot { ships, incoming: vec![None; N * N], know: vec![None; N * N], remaining: LENS.to_vec() }
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

    pub fn choose(&self) -> Cell {
        let k_ = &self.know;
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
        let mut rng = rand::thread_rng();
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
        let mut target = Bot::new();
        let shooter = &mut Bot::new();
        for _ in 0..100 {
            let at = shooter.choose();
            let (res, over) = resolve_on(&mut target.ships, &mut target.incoming, ActKind::Shot, &[at]);
            shooter.learn(&res);
            if over {
                return;
            }
        }
        panic!("le bot n'a pas fini en 100 tirs");
    }
}
