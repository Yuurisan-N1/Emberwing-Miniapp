use serde_json::{json, Value};
use std::path::PathBuf;

use super::sim::Unit;

const MEMORY_FILE: &str = "arena_state.json";
const POOL_WINDOW: usize = 24;
const MARGIN_STEP_UP: f64 = 0.06;
const MARGIN_STEP_DOWN: f64 = 0.03;
const MARGIN_MAX: f64 = 1.30;

#[derive(Debug, Default, Clone)]
pub struct Squad {
    pub power: f64,
    pub units: Vec<Unit>,
}

#[derive(Debug, Default, Clone)]
pub struct ArenaMemory {
    pub squads: Vec<Squad>,
    pub margin: f64,
    pub holds: u32,
    pub wins: u32,
    pub losses: u32,
}

impl Squad {
    fn to_json(&self) -> Value {
        let units: Vec<Value> = self
            .units
            .iter()
            .map(|u| {
                json!({
                    "a": u.atk,
                    "s": u.spd,
                    "h": u.hp,
                    "e": u.el,
                    "v": u.missile.as_ref().map(|m| m.value).unwrap_or(0.0),
                    "c": u.missile.as_ref().map(|m| m.cd_ms).unwrap_or(0.0),
                })
            })
            .collect();
        json!({ "power": self.power, "units": units })
    }

    fn from_json(v: &Value) -> Option<Squad> {
        let units: Vec<Unit> = v
            .get("units")
            .and_then(|x| x.as_array())?
            .iter()
            .filter_map(|u| {
                let atk = u.get("a").and_then(|x| x.as_f64())?;
                let cd = u.get("c").and_then(|x| x.as_f64()).unwrap_or(0.0);
                Some(Unit {
                    atk,
                    spd: u.get("s").and_then(|x| x.as_f64()).unwrap_or(10.0),
                    hp: u.get("h").and_then(|x| x.as_f64()).unwrap_or(1.0),
                    el: u
                        .get("e")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                    missile: if cd > 0.0 {
                        Some(super::sim::Abil {
                            value: u.get("v").and_then(|x| x.as_f64()).unwrap_or(0.0),
                            cd_ms: cd,
                        })
                    } else {
                        None
                    },
                    haste: false,
                })
            })
            .collect();
        if units.is_empty() {
            return None;
        }
        Some(Squad {
            power: v
                .get("power")
                .and_then(|x| x.as_f64())
                .unwrap_or_else(|| units.iter().map(|u| u.atk + u.spd).sum()),
            units,
        })
    }
}

fn path() -> PathBuf {
    let mut p = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    p.pop();
    p.push(MEMORY_FILE);
    if !p.exists() {
        let local = PathBuf::from(MEMORY_FILE);
        if local.exists() {
            return local;
        }
    }
    p
}

impl ArenaMemory {
    pub fn load() -> Self {
        let raw = std::fs::read_to_string(path()).unwrap_or_default();
        let v: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
        let mut squads: Vec<Squad> = v
            .get("squads")
            .and_then(|x| x.as_array())
            .map(|a| a.iter().filter_map(Squad::from_json).collect())
            .unwrap_or_default();
        if squads.is_empty() {
            if let Some(boards) = v.get("boards").and_then(|x| x.as_array()) {
                for b in boards.iter().filter_map(|n| n.as_f64()) {
                    squads.push(Squad {
                        power: b,
                        units: Vec::new(),
                    });
                }
            }
        }
        squads.truncate(POOL_WINDOW);
        ArenaMemory {
            squads,
            margin: v
                .get("margin")
                .and_then(|x| x.as_f64())
                .unwrap_or(1.0)
                .clamp(1.0, MARGIN_MAX),
            holds: v.get("holds").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
            wins: v.get("wins").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
            losses: v.get("losses").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
        }
    }

    fn save(&self) {
        let squads: Vec<Value> = self.squads.iter().map(Squad::to_json).collect();
        let body = json!({
            "squads": squads,
            "boards": self.squads.iter().map(|s| s.power).collect::<Vec<f64>>(),
            "margin": self.margin,
            "holds": self.holds,
            "wins": self.wins,
            "losses": self.losses,
        });
        let text = serde_json::to_string_pretty(&body).unwrap_or_default();
        let _ = std::fs::write(path(), text);
    }

    pub fn floor(&self) -> f64 {
        let v: Vec<f64> = self
            .squads
            .iter()
            .map(|s| s.power)
            .filter(|p| *p > 0.0)
            .collect();
        if v.is_empty() {
            0.0
        } else {
            v.iter().cloned().fold(f64::MAX, f64::min)
        }
    }

    pub fn ceiling(&self) -> f64 {
        self.squads.iter().map(|s| s.power).fold(0.0, f64::max)
    }

    pub fn full_squads(&self) -> Vec<&Squad> {
        self.squads.iter().filter(|s| !s.units.is_empty()).collect()
    }

    pub fn note(&mut self, power: f64, units: Vec<Unit>) {
        if power <= 0.0 || units.is_empty() {
            return;
        }
        self.squads.push(Squad { power, units });
        let extra = self.squads.len().saturating_sub(POOL_WINDOW);
        if extra > 0 {
            self.squads.drain(0..extra);
        }
        self.save();
    }

    pub fn ratio(&self, base: f64) -> f64 {
        base.max(1.0) * self.margin.max(1.0)
    }

    pub fn punish(&mut self) {
        self.margin = (self.margin + MARGIN_STEP_UP).min(MARGIN_MAX);
        self.losses = self.losses.saturating_add(1);
        self.save();
    }

    pub fn praise(&mut self) {
        self.margin = (self.margin - MARGIN_STEP_DOWN).max(1.0);
        self.wins = self.wins.saturating_add(1);
        self.save();
    }

    pub fn hold(&mut self) {
        self.holds = self.holds.saturating_add(1);
        self.save();
    }

    pub fn touched(&mut self) {
        if self.holds != 0 {
            self.holds = 0;
            self.save();
        }
    }
}
