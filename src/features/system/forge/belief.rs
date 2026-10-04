use super::logic::{apply, candidates, Board, Boost, Move};
use std::collections::HashMap;
use std::time::Instant;

const NODE_LIMIT: u64 = 3_000_000;

pub fn pool(b: &Board) -> Option<Vec<i64>> {
    let mut vis: HashMap<i64, i64> = HashMap::new();
    let mut gone: HashMap<i64, i64> = HashMap::new();
    let mut hid = 0i64;
    for i in 0..b.n {
        if b.loc[i] != 0 {
            *gone.entry(b.face[i]).or_insert(0) += 1;
        } else if b.face[i] >= 0 {
            *vis.entry(b.face[i]).or_insert(0) += 1;
        } else {
            hid += 1;
        }
    }
    if hid == 0 {
        return Some(Vec::new());
    }
    let mut keys: Vec<i64> = vis.keys().chain(gone.keys()).copied().collect();
    keys.sort_unstable();
    keys.dedup();
    let nf = keys.len() as i64;
    if nf == 0 || b.n as i64 % nf != 0 {
        return None;
    }
    let per = b.n as i64 / nf;
    if per < 3 {
        return None;
    }
    let mut out: Vec<i64> = Vec::with_capacity(hid as usize);
    for &f in &keys {
        let l = per - vis.get(&f).copied().unwrap_or(0) - gone.get(&f).copied().unwrap_or(0);
        if l < 0 {
            return None;
        }
        for _ in 0..l {
            out.push(f);
        }
    }
    if out.len() as i64 != hid {
        return None;
    }
    out.sort_unstable();
    Some(out)
}

fn sig(b: &Board, pool: &[i64]) -> Vec<i64> {
    let mut v: Vec<i64> = Vec::with_capacity(b.n * 2 + 12);
    for i in 0..b.n {
        v.push(b.loc[i] as i64);
    }
    for i in 0..b.n {
        v.push(b.face[i]);
    }
    v.push(b.tray.len() as i64);
    for &t in &b.tray {
        v.push(t as i64);
    }
    v.push(b.side.len() as i64);
    for &s in &b.side {
        v.push(s as i64);
    }
    v.push(pool.len() as i64);
    v.extend_from_slice(pool);
    v
}

fn order(b: &Board, mv: &mut [Move]) {
    let key = |m: &Move| -> i64 {
        match m {
            Move::Tap(i) => {
                if b.face[*i] < 0 {
                    -1
                } else {
                    let t = b.tray.iter().filter(|&&x| b.face[x] == b.face[*i]).count() as i64;
                    100 + 10 * t + b.exposure(*i) as i64
                }
            }
            Move::Remove => 40,
            Move::Rollback => 30,
            Move::Refresh => 20,
        }
    };
    mv.sort_by_key(|m| std::cmp::Reverse(key(m)));
}

struct Search {
    stop: Instant,
    nodes: u64,
    memo: HashMap<Vec<i64>, u8>,
}

fn force(s: &mut Search, b: &Board, bo: &Boost, pool: &[i64], depth: u8) -> bool {
    if b.st == "won" {
        return true;
    }
    if b.st == "lost" || depth == 0 {
        return false;
    }
    s.nodes += 1;
    if s.nodes > NODE_LIMIT {
        return false;
    }
    if s.nodes % 2048 == 0 && Instant::now() > s.stop {
        return false;
    }
    let key = sig(b, pool);
    if let Some(&d) = s.memo.get(&key) {
        if d >= depth {
            return true;
        }
    }
    let out = match (0..b.n).find(|&j| b.loc[j] == 0 && b.face[j] < 0 && b.free(j)) {
        Some(j) => {
            let mut fs: Vec<i64> = pool.to_vec();
            fs.dedup();
            let mut ok = true;
            for f in fs {
                let pos = match pool.iter().position(|&x| x == f) {
                    Some(p) => p,
                    None => continue,
                };
                let mut b2 = b.clone();
                let mut p2: Vec<i64> = pool.to_vec();
                p2.remove(pos);
                b2.face[j] = f;
                let bo2 = *bo;
                if !force(s, &b2, &bo2, &p2, depth) {
                    ok = false;
                    break;
                }
            }
            ok
        }
        None => {
            let mut mv = candidates(b, bo);
            order(b, &mut mv);
            let mut ok = false;
            for m in mv {
                let mut b2 = b.clone();
                let mut bo2 = *bo;
                if !apply(&mut b2, &m, &mut bo2) || b2.st == "lost" {
                    continue;
                }
                if force(s, &b2, &bo2, pool, depth - 1) {
                    ok = true;
                    break;
                }
            }
            ok
        }
    };
    if out {
        s.memo.insert(key, depth);
    }
    out
}

pub fn force_win(b: &Board, bo: &Boost, cands: &[Move], stop: Instant) -> Option<Move> {
    let pool = pool(b)?;
    if pool.len() > 64 || cands.is_empty() {
        return None;
    }
    let mut mv = cands.to_vec();
    order(b, &mut mv);
    for depth in [4u8, 6, 8, 10, 12, 14, 16, 18] {
        if Instant::now() > stop {
            return None;
        }
        let mut s = Search {
            stop,
            nodes: 0,
            memo: HashMap::new(),
        };
        for &m in &mv {
            let mut b2 = b.clone();
            let mut bo2 = *bo;
            if !apply(&mut b2, &m, &mut bo2) || b2.st == "lost" {
                continue;
            }
            if force(&mut s, &b2, &bo2, &pool, depth) {
                return Some(m);
            }
        }
    }
    None
}
