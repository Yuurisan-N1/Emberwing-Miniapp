use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use super::bits::Bits;
use super::logic::Move;

#[derive(Clone)]
pub struct Brd {
    pub n: usize,
    pub up: Arc<Vec<Bits>>,
    pub on: Bits,
    pub face: Vec<i64>,
    pub tray: Vec<usize>,
    pub side: Vec<usize>,
    pub slots: usize,
    pub stash: usize,
    pub st: u8,
}

impl Brd {
    pub fn new(n: usize, up: Arc<Vec<Bits>>, face: Vec<i64>, slots: usize, stash: usize) -> Self {
        Brd {
            n,
            up,
            on: Bits::ones(n),
            face,
            tray: Vec::new(),
            side: Vec::new(),
            slots,
            stash,
            st: 0,
        }
    }

    pub fn free(&self, i: usize) -> bool {
        !self.up[i].intersects(&self.on)
    }

    pub fn free_ids(&self) -> Vec<usize> {
        self.on
            .ids()
            .into_iter()
            .filter(|&i| self.free(i))
            .collect()
    }

    pub fn face_on(&self, f: i64) -> usize {
        self.on
            .ids()
            .iter()
            .filter(|&&i| self.face[i] == f)
            .count()
    }

}

pub fn load_min(b: &Brd) -> usize {
    let mut cnt: HashMap<i64, usize> = HashMap::new();
    for i in b.on.ids() {
        *cnt.entry(b.face[i]).or_insert(0) += 1;
    }
    for i in 0..b.tray.len() {
        *cnt.entry(b.face[b.tray[i]]).or_insert(0) += 1;
    }
    for i in 0..b.side.len() {
        *cnt.entry(b.face[b.side[i]]).or_insert(0) += 1;
    }
    let mut s = 0;
    for (_, &c) in &cnt {
        s += (3 - c % 3) % 3;
    }
    s
}

pub fn upmask(geo: &[(i64, i64, i64)]) -> Vec<Bits> {
    let n = geo.len();
    let mut up = vec![Bits::zeros(n); n];
    for i in 0..n {
        for j in 0..n {
            if geo[j].2 <= geo[i].2 {
                continue;
            }
            if (geo[j].0 - geo[i].0).abs() < 2 && (geo[j].1 - geo[i].1).abs() < 2 {
                up[i].set(j);
            }
        }
    }
    up
}


pub fn key(b: &Brd, rm: i64, rf: i64, rb: i64) -> u64 {
    let mut h = 0x9e37_79b9_7f4a_7c15u64;
    h = mix(h, b.on.digest());
    h = mix(h, b.tray.len() as u64);
    let mut t: Vec<i64> = b.tray.iter().map(|&i| b.face[i]).collect();
    t.sort_unstable();
    for f in t {
        h = mix(h, f as u64);
    }
    h = mix(h, b.side.len() as u64);
    let mut s: Vec<i64> = b.side.iter().map(|&i| b.face[i]).collect();
    s.sort_unstable();
    for f in s {
        h = mix(h, f as u64);
    }
    h = mix(h, rm as u64);
    h = mix(h, rf as u64);
    h = mix(h, rb as u64);
    h
}

fn mix(mut h: u64, x: u64) -> u64 {
    h ^= x.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    h = h.rotate_left(27).wrapping_mul(0x94d0_49bb_1331_11eb);
    h ^ x.rotate_left(31)
}

pub struct Search {
    pub nodes: u64,
    pub cap: u64,
    pub stop: Option<Instant>,
    pub memo: HashMap<u64, bool>,
    pub tot: HashMap<i64, usize>,
    pub overflow: bool,
}

impl Search {
    pub fn new(tot: HashMap<i64, usize>, cap: u64, stop: Option<Instant>) -> Self {
        Search {
            nodes: 0,
            cap,
            stop,
            memo: HashMap::new(),
            tot,
            overflow: false,
        }
    }

    fn budget(&self) -> bool {
        if self.nodes >= self.cap {
            return false;
        }
        if let Some(t) = self.stop {
            if t.elapsed().as_millis() as u64 > 0 && Instant::now() >= t {
                return false;
            }
        }
        true
    }

    pub fn win(&mut self, b: &Brd, rm: i64, rf: i64, rb: i64) -> Option<bool> {
        if b.st == 1 {
            return Some(true);
        }
        if b.st == 2 {
            return Some(false);
        }
        if load_min(b) >= b.slots {
            return Some(false);
        }
        if !self.budget() {
            self.overflow = true;
            return None;
        }
        let k = key(b, rm, rf, rb);
        if let Some(&v) = self.memo.get(&k) {
            return Some(v);
        }
        self.nodes += 1;
        if b.tray.len() >= b.slots {
            return Some(false);
        }
        if b.on.is_empty() && b.tray.is_empty() && b.side.is_empty() {
            return Some(true);
        }
        let free = b.free_ids();
        let mut cands: Vec<Move> = Vec::new();
        for i in free {
            cands.push(Move::Tap(i));
        }
        for i in 0..b.side.len() {
            cands.push(Move::Tap(b.side[i]));
        }
        if rm > 0 && !b.tray.is_empty() && b.stash.saturating_sub(b.side.len()) >= b.tray.len().min(3) {
            cands.push(Move::Remove);
        }
        if rb > 0 && !b.tray.is_empty() {
            cands.push(Move::Rollback);
        }
        if rf > 0 && refresh_ok(b) {
            cands.push(Move::Refresh);
        }
        order(b, &self.tot, &mut cands);
        for m in cands {
            let mut nb = b.clone();
            let mut nrm = rm;
            let mut nrf = rf;
            let mut nrb = rb;
            if !step(&mut nb, &m, &mut nrm, &mut nrf, &mut nrb) {
                continue;
            }
            if nb.st == 0 && nb.tray.len() >= nb.slots {
                nb.st = 2;
            }
            if let Some(r) = self.win(&nb, nrm, nrf, nrb) {
                if r {
                    self.memo.insert(k, true);
                    return Some(true);
                }
            } else {
                self.overflow = true;
                return None;
            }
        }
        self.memo.insert(k, false);
        Some(false)
    }
}

pub fn refresh_ok(b: &Brd) -> bool {
    if b.tray.len() == b.slots - 1 {
        let mut set = std::collections::HashSet::new();
        for &t in &b.tray {
            set.insert(b.face[t]);
        }
        if set.len() == b.slots - 1 {
            return false;
        }
    }
    b.on.count() >= 2
}

fn order(b: &Brd, tot: &HashMap<i64, usize>, mv: &mut [Move]) {
    let cnt = |f: i64| b.tray.iter().filter(|&&t| b.face[t] == f).count() as i64;
    mv.sort_by_key(|m| match m {
        Move::Tap(i) => {
            if b.face[*i] < 0 {
                (3i64, 0i64)
            } else {
                let c = tot.get(&b.face[*i]).copied().unwrap_or(0) as i64;
                let left = c - b.face_on(b.face[*i]) as i64;
                let t = cnt(b.face[*i]);
                if t >= 2 {
                    (0, -1000 - t)
                } else if t == 1 && left >= 2 {
                    (1, -100 - left)
                } else {
                    (2, -left)
                }
            }
        }
        Move::Remove => (4, 0),
        Move::Rollback => (5, 0),
        Move::Refresh => (6, 0),
    });
}

pub fn step(b: &mut Brd, m: &Move, rm: &mut i64, rf: &mut i64, rb: &mut i64) -> bool {
    match m {
        Move::Tap(id) => {
            let id = *id;
            if !b.on.test(id) && !b.side.contains(&id) {
                return false;
            }
            if b.on.test(id) && !b.free(id) {
                return false;
            }
            if let Some(p) = b.side.iter().position(|&x| x == id) {
                b.side.remove(p);
            }
            b.on.clear(id);
            let f = b.face[id];
            let mut at: i64 = -1;
            for (i, &t) in b.tray.iter().enumerate() {
                if b.face[t] == f {
                    at = i as i64;
                }
            }
            let pos = if at < 0 {
                b.tray.len()
            } else {
                (at + 1) as usize
            };
            b.tray.insert(pos, id);
            let same: Vec<usize> = b
                .tray
                .iter()
                .copied()
                .filter(|&t| b.face[t] == f)
                .collect();
            if same.len() >= 3 {
                let cl: Vec<usize> = same[..3].to_vec();
                b.tray.retain(|t| !cl.contains(t));
            }
            if b.on.is_empty() && b.tray.is_empty() && b.side.is_empty() {
                b.st = 1;
            } else if b.tray.len() >= b.slots {
                b.st = 2;
            }
            true
        }
        Move::Remove => {
            if *rm <= 0 || b.tray.is_empty() {
                return false;
            }
            let k = 3.min(b.tray.len());
            if b.stash.saturating_sub(b.side.len()) < k {
                return false;
            }
            let ids: Vec<usize> = b.tray.drain(0..k).collect();
            for t in ids {
                b.side.push(t);
            }
            *rm -= 1;
            true
        }
        Move::Rollback => {
            if *rb <= 0 || b.tray.is_empty() {
                return false;
            }
            let mut t = b.tray[0];
            for &x in &b.tray {
                if x > t {
                    t = x;
                }
            }
            let p = b.tray.iter().position(|&x| x == t).unwrap_or(0);
            b.tray.remove(p);
            b.on.set(t);
            *rb -= 1;
            true
        }
        Move::Refresh => {
            if *rf <= 0 || !refresh_ok(b) {
                return false;
            }
            for i in 0..b.n {
                if b.on.test(i) {
                    b.face[i] = -1;
                }
            }
            *rf -= 1;
            true
        }
    }
}
