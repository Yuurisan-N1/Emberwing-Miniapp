#![allow(dead_code)]

use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Snapshot {
    #[serde(default)]
    pub player: Player,
    #[serde(default)]
    pub dragons: Vec<Dragon>,
    #[serde(default)]
    pub eggs: Vec<Egg>,
    #[serde(default, rename = "queueN")]
    pub queue_n: i64,
    #[serde(default)]
    pub trainers: Vec<Value>,
    #[serde(default)]
    pub wizards: Vec<Wizard>,
    #[serde(default)]
    pub buildings: Vec<Building>,
    #[serde(default)]
    pub habitants: Vec<Habitant>,
    #[serde(default)]
    pub notices: Vec<Value>,
    #[serde(default)]
    pub exp: Exp,
    #[serde(default)]
    pub monster: Monster,
    #[serde(default)]
    pub ev: Value,
    #[serde(default, rename = "evCfg")]
    pub ev_cfg: Value,
    #[serde(default, rename = "hallSkin")]
    pub hall_skin: Value,
    #[serde(default)]
    pub hunt: Value,
    #[serde(default)]
    pub quest: Value,
    #[serde(default)]
    pub cfg: Value,
    #[serde(default, rename = "cfgV")]
    pub cfg_v: String,
    #[serde(default, rename = "appV")]
    pub app_v: Value,
    #[serde(default, rename = "pendingBattle")]
    pub pending_battle: Value,
    #[serde(default)]
    pub pass: Value,
    #[serde(default)]
    pub dg: bool,
    #[serde(default, rename = "dgClaim")]
    pub dg_claim: bool,
    #[serde(default)]
    pub island: bool,
    #[serde(default)]
    pub now: i64,
    #[serde(skip)]
    pub dg_state: Value,
    #[serde(skip)]
    pub referrals: Value,
    #[serde(skip)]
    pub raid_log: Value,
    #[serde(skip)]
    pub raid_defense: Value,
    #[serde(skip)]
    pub exp_list: Value,
    #[serde(skip)]
    pub leaderboard: Value,
    #[serde(skip)]
    pub pushed_hold: i64,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Player {
    #[serde(default)]
    pub id: Value,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub first_name: Option<String>,
    #[serde(default)]
    pub gold: f64,
    #[serde(default)]
    pub stars: f64,
    #[serde(default)]
    pub emb: f64,
    #[serde(default)]
    pub ton: f64,
    #[serde(default)]
    pub meat: f64,
    #[serde(default)]
    pub meat_max: f64,
    #[serde(default)]
    pub potion: f64,
    #[serde(default)]
    pub logs: f64,
    #[serde(default)]
    pub builders: i64,
    #[serde(default)]
    pub skill_badges: i64,
    #[serde(default)]
    pub keeper_lvl: i64,
    #[serde(default)]
    pub keeper_xp: i64,
    #[serde(default)]
    pub trophies: i64,
    #[serde(default)]
    pub roost_slots: i64,
    #[serde(default)]
    pub locked_cells: Vec<i64>,
    #[serde(default)]
    pub counters: Value,
    #[serde(default)]
    pub ach: Value,
    #[serde(default)]
    pub wallet_addr: Option<String>,
    #[serde(default)]
    pub isl_road: Value,
    #[serde(default)]
    pub isl_areas: Value,
    #[serde(default)]
    pub isl_shield_until: Value,
    #[serde(default)]
    pub isl_def: Value,
    #[serde(default)]
    pub tav_pity_d: i64,
    #[serde(default)]
    pub tav_pity_i: i64,
    #[serde(default)]
    pub tav_pack_step: i64,
    #[serde(default)]
    pub tav_pack_step_i: i64,
    #[serde(default)]
    pub egg_pack_step: i64,
    #[serde(default)]
    pub raid_wins_today: i64,
    #[serde(default)]
    pub isl_hunt: Value,
    #[serde(default)]
    pub hall_skin: Value,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Dragon {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub element: String,
    #[serde(default)]
    pub rarity: i64,
    #[serde(default)]
    pub level: i64,
    #[serde(default)]
    pub atk: f64,
    #[serde(default)]
    pub def: f64,
    #[serde(default)]
    pub spd: f64,
    #[serde(default)]
    pub heat: f64,
    #[serde(default)]
    pub listed: i64,
    #[serde(default)]
    pub training_until: Value,
    #[serde(default)]
    pub trainer_id: Value,
    #[serde(default)]
    pub rest_until: Value,
    #[serde(default)]
    pub isl_fights: i64,
    #[serde(default)]
    pub battles_win: i64,
    #[serde(default)]
    pub battles_today: i64,
    #[serde(default)]
    pub battles_win_u: i64,
    #[serde(default)]
    pub battles_today_u: i64,
    #[serde(default)]
    pub star_frags: i64,
    #[serde(default)]
    pub hp_buf: f64,
    #[serde(default)]
    pub isl_lvl: i64,
    #[serde(default)]
    pub isl_stats: Value,
    #[serde(default)]
    pub isl_abils: Vec<Value>,
    #[serde(default)]
    pub gear: Value,
    #[serde(default)]
    pub exp_id: Value,
}

impl Dragon {
    pub fn pow(&self) -> f64 {
        if let Some(p) = self.isl_stats.get("pow").and_then(|v| v.as_f64()) {
            return p;
        }
        self.atk + self.def + self.spd
    }

    pub fn isl_level(&self) -> i64 {
        self.isl_stats
            .get("lvl")
            .and_then(|v| v.as_i64())
            .unwrap_or(self.isl_lvl)
    }

    pub fn max_level(&self) -> i64 {
        self.isl_stats
            .get("maxLvl")
            .and_then(|v| v.as_i64())
            .unwrap_or(0)
    }

    pub fn next_meat(&self) -> f64 {
        self.isl_stats
            .get("nextMeat")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0)
    }

    pub fn arch(&self) -> String {
        self.isl_stats
            .get("arch")
            .and_then(|v| v.as_str())
            .unwrap_or("bal")
            .to_string()
    }

    pub fn away(&self) -> bool {
        !self.exp_id.is_null()
    }

    pub fn training(&self, now_ms: i64) -> bool {
        parse_ts(&self.training_until) > now_ms
    }

    pub fn resting(&self, now_ms: i64) -> bool {
        parse_ts(&self.rest_until) > now_ms
    }

    pub fn used(&self, marker: f64, unranked: bool) -> f64 {
        let win = if unranked {
            self.battles_win_u as f64
        } else {
            self.battles_win as f64
        };
        if marker != 0.0 && win == marker {
            if unranked {
                self.battles_today_u as f64
            } else {
                self.battles_today as f64
            }
        } else {
            0.0
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Egg {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub tier: i64,
    #[serde(default)]
    pub element: String,
    #[serde(default)]
    pub zone: String,
    #[serde(default)]
    pub cell: Option<i64>,
    #[serde(default)]
    pub queue_pos: Option<i64>,
    #[serde(default)]
    pub sealed: i64,
    #[serde(default)]
    pub incubate_until: Value,
}

impl Egg {
    pub fn incubating(&self, now_ms: i64) -> bool {
        let end = parse_ts(&self.incubate_until);
        end > 0 && end > now_ms
    }

    pub fn ready(&self, now_ms: i64) -> bool {
        let end = parse_ts(&self.incubate_until);
        end > 0 && end <= now_ms
    }

    pub fn idle(&self) -> bool {
        parse_ts(&self.incubate_until) == 0
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Building {
    #[serde(default)]
    pub id: i64,
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(default)]
    pub lvl: i64,
    #[serde(default)]
    pub broken: i64,
    #[serde(default)]
    pub stock: f64,
    #[serde(default)]
    pub job_type: Option<String>,
    #[serde(default)]
    pub job_until: Value,
}

impl Building {
    pub fn working(&self, now_ms: i64) -> bool {
        parse_ts(&self.job_until) > now_ms
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Habitant {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub building_id: Value,
    #[serde(default)]
    pub rarity: i64,
    #[serde(default)]
    pub name_idx: i64,
    #[serde(default)]
    pub star_frags: i64,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Wizard {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub lvl: i64,
    #[serde(default)]
    pub dragon_id: Value,
    #[serde(default)]
    pub until: Value,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Exp {
    #[serde(default)]
    pub mine: Vec<Value>,
    #[serde(default)]
    pub fee: f64,
    #[serde(default)]
    pub bot_left: i64,
    #[serde(default)]
    pub quota: Quota,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Quota {
    #[serde(default)]
    pub max: i64,
    #[serde(default, rename = "windowH")]
    pub window_h: i64,
    #[serde(default)]
    pub used: i64,
    #[serde(default)]
    pub left: i64,
    #[serde(default)]
    pub reset_at: Value,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Monster {
    #[serde(default)]
    pub cur: Value,
    #[serde(default)]
    pub next_at: Value,
}

pub fn parse_ts(v: &Value) -> i64 {
    match v {
        Value::Null => 0,
        Value::Number(n) => n.as_f64().unwrap_or(0.0) as i64,
        Value::String(s) => {
            if s.is_empty() {
                return 0;
            }
            if let Ok(n) = s.parse::<i64>() {
                return n;
            }
            match chrono::DateTime::parse_from_rfc3339(s) {
                Ok(dt) => dt.timestamp_millis(),
                Err(_) => match chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.fZ") {
                    Ok(naive) => naive.and_utc().timestamp_millis(),
                    Err(_) => 0,
                },
            }
        }
        _ => 0,
    }
}

impl Snapshot {
    pub fn apply(&mut self, sn: &Value) {
        if !sn.is_object() {
            return;
        }
        let now = sn.get("now").and_then(|v| v.as_i64()).unwrap_or(0);
        if now > 0 {
            self.now = now;
        }
        merge(sn, "player", &mut self.player);
        merge(sn, "dragons", &mut self.dragons);
        merge(sn, "eggs", &mut self.eggs);
        merge(sn, "queueN", &mut self.queue_n);
        merge(sn, "trainers", &mut self.trainers);
        merge(sn, "wizards", &mut self.wizards);
        merge(sn, "buildings", &mut self.buildings);
        merge(sn, "habitants", &mut self.habitants);
        merge(sn, "notices", &mut self.notices);
        merge(sn, "exp", &mut self.exp);
        merge(sn, "monster", &mut self.monster);
        merge(sn, "ev", &mut self.ev);
        merge(sn, "evCfg", &mut self.ev_cfg);
        merge(sn, "hallSkin", &mut self.hall_skin);
        merge(sn, "hunt", &mut self.hunt);
        merge(sn, "quest", &mut self.quest);
        merge(sn, "cfg", &mut self.cfg);
        merge(sn, "appV", &mut self.app_v);
        merge(sn, "pendingBattle", &mut self.pending_battle);
        merge(sn, "pass", &mut self.pass);
        merge(sn, "dg", &mut self.dg);
        merge(sn, "dgClaim", &mut self.dg_claim);
        merge(sn, "island", &mut self.island);
        if let Some(v) = sn.get("cfgV").and_then(|v| v.as_str()) {
            if !v.is_empty() {
                self.cfg_v = v.to_string();
            }
        }
    }

    pub fn isl_cfg(&self) -> &Value {
        self.cfg.get("isl").unwrap_or(&Value::Null)
    }

    pub fn cfg_num(&self, path: &[&str]) -> Option<f64> {
        let mut cur = &self.cfg;
        for p in path {
            cur = cur.get(*p)?;
        }
        cur.as_f64()
    }

    pub fn cfg_str(&self, path: &[&str]) -> Option<String> {
        let mut cur = &self.cfg;
        for p in path {
            cur = cur.get(*p)?;
        }
        cur.as_str().map(|s| s.to_string())
    }

    pub fn cfg_bool(&self, path: &[&str]) -> Option<bool> {
        let mut cur = &self.cfg;
        for p in path {
            cur = cur.get(*p)?;
        }
        match cur {
            Value::Bool(b) => Some(*b),
            Value::Number(n) => Some(n.as_i64().unwrap_or(0) != 0),
            _ => None,
        }
    }

    pub fn isl_num(&self, path: &[&str]) -> Option<f64> {
        let mut cur = self.isl_cfg();
        for p in path {
            cur = cur.get(*p)?;
        }
        cur.as_f64()
    }

    pub fn isl_at(&self, path: &[&str]) -> Value {
        let mut cur = self.isl_cfg();
        for p in path {
            match cur.get(*p) {
                Some(v) => cur = v,
                None => return Value::Null,
            }
        }
        cur.clone()
    }

    pub fn meat_now(&self) -> f64 {
        let base = self.player.meat;
        let cap = self.player.meat_max.max(base);
        let regen = self.cfg_num(&["meatRegenSec"]).unwrap_or(0.0);
        if regen <= 0.0 || self.now <= 0 {
            return base.min(cap);
        }
        let elapsed_min = (local_now_ms() - self.now).max(0) as f64 / 60_000.0;
        (base + elapsed_min * 60.0 / regen).min(cap)
    }

    pub fn res(&self, name: &str) -> f64 {
        match name {
            "gold" => self.player.gold,
            "logs" => self.player.logs,
            "meat" => self.meat_now(),
            "potion" => self.player.potion,
            "stars" => self.player.stars,
            "emb" => self.player.emb,
            "ton" => self.player.ton,
            _ => 0.0,
        }
    }

    pub fn label(&self) -> String {
        if let Some(u) = &self.player.username {
            if !u.trim().is_empty() {
                return u.clone();
            }
        }
        if let Some(f) = &self.player.first_name {
            if !f.trim().is_empty() {
                return f.clone();
            }
        }
        format!("{}", self.player.id)
    }

    pub fn hall_lvl(&self) -> i64 {
        self.buildings
            .iter()
            .filter(|b| b.kind == "hall")
            .map(|b| b.lvl)
            .max()
            .unwrap_or(0)
    }

    pub fn egg_max_tier(&self) -> i64 {
        self.isl_num(&["eggMax"]).unwrap_or(0.0) as i64
    }

    pub fn atk_limit(&self, league_idx: i64) -> f64 {
        self.cfg
            .get("atkPerWindow")
            .and_then(|v| v.as_array())
            .and_then(|a| a.get(league_idx.max(0) as usize))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0)
    }

    pub fn atk_marker(&self) -> f64 {
        self.cfg_num(&["atkWin"]).unwrap_or(0.0)
    }

    pub fn dragon_free(&self, d: &Dragon, lim: f64, for_raid: bool) -> bool {
        if d.listed != 0 || d.away() {
            return false;
        }
        if d.training(self.now) {
            return false;
        }
        if for_raid && d.resting(self.now) {
            return false;
        }
        d.used(self.atk_marker(), false) < lim
    }

    pub fn free_dragons(&self, lim: f64, for_raid: bool) -> Vec<Dragon> {
        let mut v: Vec<Dragon> = self
            .dragons
            .iter()
            .filter(|d| self.dragon_free(d, lim, for_raid))
            .cloned()
            .collect();
        v.sort_by(|a, b| b.pow().partial_cmp(&a.pow()).unwrap_or(std::cmp::Ordering::Equal));
        v
    }

    pub fn league_idx(&self) -> i64 {
        self.cfg
            .get("leagueMins")
            .and_then(|v| v.as_array())
            .map(|a| {
                let t = self.player.trophies as f64;
                let mut idx = 0i64;
                for (i, m) in a.iter().enumerate() {
                    if t >= m.as_f64().unwrap_or(0.0) {
                        idx = i as i64;
                    }
                }
                idx
            })
            .unwrap_or(0)
    }
}

fn merge<T>(sn: &Value, key: &str, slot: &mut T)
where
    T: for<'de> Deserialize<'de>,
{
    if let Some(v) = sn.get(key) {
        if v.is_null() {
            return;
        }
        if let Ok(parsed) = serde_json::from_value::<T>(v.clone()) {
            *slot = parsed;
        }
    }
}

pub fn foe_pow(f: &Value) -> f64 {
    if let Some(p) = f.get("isl").and_then(|i| i.get("pow")).and_then(|v| v.as_f64()) {
        return p;
    }
    ["atk", "def", "spd"]
        .iter()
        .map(|k| f.get(*k).and_then(|v| v.as_f64()).unwrap_or(0.0))
        .sum()
}

pub fn local_now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn el_mult(a: &str, b: &str) -> f64 {
    const ORDER: [&str; 5] = ["fire", "ice", "storm", "earth", "venom"];
    let ia = ORDER.iter().position(|x| *x == a);
    let ib = ORDER.iter().position(|x| *x == b);
    if let (Some(ia), Some(ib)) = (ia, ib) {
        let d = (ib as i32 - ia as i32 + 5) % 5;
        if d == 1 || d == 2 {
            return 1.25;
        }
        if d >= 3 {
            return 0.8;
        }
        return 1.0;
    }
    if a == "celestial" {
        return if b == "celestial" { 1.0 } else { 1.15 };
    }
    if a == "void" {
        return if b == "celestial" {
            0.85
        } else if b == "void" {
            1.0
        } else {
            1.15
        };
    }
    if b == "void" || b == "celestial" {
        return 0.9;
    }
    1.0
}
