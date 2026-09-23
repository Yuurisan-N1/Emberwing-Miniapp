use serde_json::{json, Value};
use std::path::PathBuf;

pub const CALIB_FILE: &str = "arena_model.json";
pub const BINS: usize = 10;

#[derive(Debug, Clone, Copy, Default)]
pub struct Bin {
    pub w: f64,
    pub n: f64,
}

#[derive(Debug, Clone)]
pub struct Calib {
    pub bins: Vec<Bin>,
    pub fights: f64,
    pub wins: f64,
    pub trophies: f64,
    pub gold: f64,
    pub emb: f64,
}

fn path() -> PathBuf {
    let mut p = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    p.pop();
    p.push(CALIB_FILE);
    if p.exists() {
        return p;
    }
    let local = PathBuf::from(CALIB_FILE);
    if local.exists() {
        return local;
    }
    p
}

impl Default for Calib {
    fn default() -> Self {
        Calib {
            bins: vec![Bin::default(); BINS],
            fights: 0.0,
            wins: 0.0,
            trophies: 0.0,
            gold: 0.0,
            emb: 0.0,
        }
    }
}

fn bin_of(p: f64) -> usize {
    let i = (p.clamp(0.0, 0.999) * BINS as f64) as usize;
    i.min(BINS - 1)
}

impl Calib {
    pub fn load() -> Calib {
        let raw = std::fs::read_to_string(path()).unwrap_or_default();
        let v: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
        let mut c = Calib::default();
        if let Some(arr) = v.get("bins").and_then(|x| x.as_array()) {
            for (i, b) in arr.iter().enumerate() {
                if i >= BINS {
                    break;
                }
                c.bins[i] = Bin {
                    w: b.get("w").and_then(|x| x.as_f64()).unwrap_or(0.0),
                    n: b.get("n").and_then(|x| x.as_f64()).unwrap_or(0.0),
                };
            }
        }
        let num = |key: &str| v.get(key).and_then(|x| x.as_f64()).unwrap_or(0.0);
        c.fights = num("fights");
        c.wins = num("wins");
        c.trophies = num("trophies");
        c.gold = num("gold");
        c.emb = num("emb");
        c
    }

    pub fn save(&self) {
        let bins: Vec<Value> = self
            .bins
            .iter()
            .map(|b| json!({ "w": b.w, "n": b.n }))
            .collect();
        let body = json!({
            "bins": bins,
            "fights": self.fights,
            "wins": self.wins,
            "trophies": self.trophies,
            "gold": self.gold,
            "emb": self.emb,
        });
        let text = serde_json::to_string_pretty(&body).unwrap_or_default();
        let _ = std::fs::write(path(), text);
    }

    pub fn rate(&self, p: f64) -> f64 {
        let i = bin_of(p);
        let b = self.bins[i];
        let centre = (i as f64 + 0.5) / BINS as f64;
        (2.0 * centre + b.w + p) / (2.0 + b.n + 1.0)
    }

    pub fn samples(&self, p: f64) -> f64 {
        self.bins[bin_of(p)].n
    }

    pub fn note(&mut self, p: f64, won: bool) {
        let i = bin_of(p);
        self.bins[i].n += 1.0;
        if won {
            self.bins[i].w += 1.0;
        }
        self.fights += 1.0;
        if won {
            self.wins += 1.0;
        }
    }

    pub fn note_rewards(&mut self, trophies: f64, gold: f64, emb: f64) {
        self.trophies += trophies;
        self.gold += gold;
        self.emb += emb;
    }

    pub fn empirical(&self) -> f64 {
        if self.fights <= 0.0 {
            0.0
        } else {
            self.wins / self.fights
        }
    }

    pub fn table(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        for (i, b) in self.bins.iter().enumerate() {
            if b.n <= 0.0 {
                continue;
            }
            parts.push(format!(
                "{} to {} wins {} of {}",
                i,
                i + 1,
                crate::core::logger::num(b.w),
                crate::core::logger::num(b.n)
            ));
        }
        if parts.is_empty() {
            "no calibrated fight yet".to_string()
        } else {
            parts.join(", ")
        }
    }
}
