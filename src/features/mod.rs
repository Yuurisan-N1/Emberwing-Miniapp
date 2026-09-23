use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::core::config::Config;
use crate::core::http::{Api, Res};
use crate::core::logger;
use crate::core::state::{el_mult, Snapshot};

pub mod auth;
pub mod combat;
pub mod entities;
pub mod progression;
pub mod system;

pub use auth::{presence, referrals, session};
pub use combat::{arena, expeditions, hunts, raids};
pub use entities::{dragons, eggs, gear, island, tavern, villagers};
pub use progression::{achievements, daily_gift, pass, quests};
pub use system::{event, notices};

pub struct Ctx<'a> {
    pub api: Arc<Api>,
    pub st: &'a mut Snapshot,
    pub cfg: &'a Config,
    pub init_data: &'a str,
}

impl<'a> Ctx<'a> {
    pub async fn act(&mut self, path: &str, body: Value) -> Result<Res> {
        self.api.act(path, body, self.st).await
    }

    pub async fn get(&mut self, path: &str) -> Result<Res> {
        let res = self.api.get(path).await?;
        if let Some(sn) = res.snapshot() {
            self.st.apply(sn);
        }
        Ok(res)
    }

    pub fn now(&self) -> i64 {
        crate::core::http::server_now_ms(self.st)
    }

    pub fn league(&self) -> i64 {
        self.st.league_idx()
    }

    pub fn atk_limit(&self) -> f64 {
        self.st.atk_limit(self.league())
    }
}

pub fn is_hold(reason: &str) -> bool {
    reason.starts_with("cooldown") || reason.contains("maintenance")
}

pub fn report(tag: &str, res: &Res, good: &str) {
    if res.ok {
        logger::ok(tag, good);
    } else if is_hold(&res.reason()) {
        logger::skip(tag, &res.reason());
    } else {
        logger::skip(tag, &format!("refused: {}", res.reason()));
    }
}

pub fn pick_slots(st: &Snapshot, foes: &[Value], n: usize, for_raid: bool) -> Vec<Value> {
    let lim = st.atk_limit(st.league_idx());
    let mut pool = st.free_dragons(lim, for_raid);
    if pool.is_empty() || n == 0 {
        return vec![];
    }

    let mut order: Vec<(usize, f64, String)> = foes
        .iter()
        .enumerate()
        .take(n)
        .map(|(i, f)| {
            let el = f
                .get("el")
                .or_else(|| f.get("element"))
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            (i, crate::core::state::foe_pow(f), el)
        })
        .collect();
    order.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut out: Vec<Value> = Vec::new();
    for (slot, _, foe_el) in order {
        if pool.is_empty() {
            break;
        }
        let mut best = 0usize;
        let mut best_score = f64::MIN;
        for (j, d) in pool.iter().enumerate() {
            let score = el_mult(&d.element, &foe_el) * 1000.0 + d.pow();
            if score > best_score {
                best_score = score;
                best = j;
            }
        }
        let d = pool.remove(best);
        out.push(json!({ "slot": slot, "id": d.id }));
    }
    out
}

pub fn team_score(st: &Snapshot, slots: &[Value], foes: &[Value]) -> (f64, f64) {
    let mine: f64 = slots
        .iter()
        .filter_map(|s| s.get("id").and_then(|v| v.as_i64()))
        .filter_map(|id| st.dragons.iter().find(|d| d.id == id))
        .map(|d| d.pow())
        .sum();
    let theirs: f64 = foes.iter().map(crate::core::state::foe_pow).sum();
    (mine, theirs)
}

pub fn session_of(res: &Res) -> Option<Value> {
    res.get("sessionId")
        .cloned()
        .filter(|v| !v.is_null())
        .or_else(|| res.get("session").cloned().filter(|v| !v.is_null()))
}

pub fn foes_of(res: &Res) -> Vec<Value> {
    res.arr("foes")
}

pub fn slots_len(res: &Res, fallback: usize) -> usize {
    res.get("slots")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .unwrap_or(fallback)
        .clamp(1, 5)
}
