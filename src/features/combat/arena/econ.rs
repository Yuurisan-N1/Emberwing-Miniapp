use serde_json::Value;

use crate::core::state::Snapshot;

#[derive(Debug, Clone)]
pub struct Econ {
    pub trophy_win: f64,
    pub trophy_loss: f64,
    pub win_gold: f64,
    pub loss_gold: f64,
    pub win_emb: f64,
    pub loss_emb: f64,
    pub reroll_gold: f64,
    pub free_rerolls: f64,
    pub unr_loss_div: f64,
    pub boost_atk: f64,
    pub boost_shield: f64,
    pub unr_boost_pct: f64,
    pub shield_full_leagues: i64,
}

fn arr_at(v: Option<&Value>, i: usize, d: f64) -> f64 {
    v.and_then(|x| x.as_array())
        .and_then(|a| a.get(i))
        .and_then(|x| x.as_f64())
        .unwrap_or(d)
}

impl Econ {
    pub fn of(st: &Snapshot, league: i64) -> Econ {
        let cfg = &st.cfg;
        let i = league.max(0) as usize;
        let tw = cfg.get("trophyWin").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let tl = cfg.get("trophyLoss").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let twl = cfg.get("trophyWinPerLeague").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let tll = cfg.get("trophyLossPerLeague").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let emb_loss = cfg.get("embLoss").and_then(|v| v.as_f64()).unwrap_or(0.0);
        Econ {
            trophy_win: tw + twl * league.max(0) as f64,
            trophy_loss: tl + tll * league.max(0) as f64,
            win_gold: arr_at(cfg.get("winGold"), i, 0.0),
            loss_gold: arr_at(cfg.get("lossGold"), i, 0.0),
            win_emb: arr_at(cfg.get("winEmb"), i, 0.0),
            loss_emb: emb_loss,
            reroll_gold: arr_at(cfg.get("rerollGold"), i, 0.0),
            free_rerolls: cfg.get("passRerolls").and_then(|v| v.as_f64()).unwrap_or(0.0),
            unr_loss_div: cfg.get("unrLossDiv").and_then(|v| v.as_f64()).unwrap_or(1.0),
            boost_atk: boost_price(cfg, "atk"),
            boost_shield: boost_price(cfg, "shield"),
            unr_boost_pct: cfg
                .get("unrBoostPct")
                .and_then(|v| v.as_f64())
                .unwrap_or(100.0),
            shield_full_leagues: cfg
                .get("shieldFullLeagues")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as i64,
        }
    }

    pub fn shield_worth(&self, league: i64, p: f64) -> f64 {
        if p >= self.break_even() {
            return 0.0;
        }
        let full = if league < self.shield_full_leagues {
            self.trophy_loss
        } else {
            self.trophy_loss * 0.5
        };
        full
    }

    pub fn boost_price(&self, base: f64, unranked: bool) -> f64 {
        if unranked {
            (base * (self.unr_boost_pct / 100.0)).round().max(0.0)
        } else {
            base
        }
    }

    pub fn horn_gain(&self, p: f64, p_horn: f64) -> f64 {
        if p_horn <= p || p_horn < self.break_even() {
            return 0.0;
        }
        (p_horn - p) * (self.trophy_win + self.trophy_loss)
            + (self.win_gold - self.loss_gold) * p_horn
            + self.loss_gold * p_horn
    }
}

fn boost_price(cfg: &Value, kind: &str) -> f64 {
    cfg.get("boosts")
        .and_then(|b| b.get(kind))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0)
}

impl Econ {
    pub fn trophy_ev(&self, p: f64) -> f64 {
        p * self.trophy_win - (1.0 - p) * self.trophy_loss
    }

    pub fn gold_break_even(&self) -> f64 {
        let d = self.win_gold + self.loss_gold;
        if d <= 0.0 {
            1.0
        } else {
            self.loss_gold / d
        }
    }

    pub fn emb_break_even(&self) -> f64 {
        let d = self.win_emb + self.loss_emb;
        if d <= 0.0 {
            1.0
        } else {
            self.loss_emb / d
        }
    }

    pub fn break_even(&self) -> f64 {
        let d = self.trophy_win + self.trophy_loss;
        if d <= 0.0 {
            1.0
        } else {
            self.trophy_loss / d
        }
    }

    pub fn free_list(&self, st: &Snapshot) -> f64 {
        let left = st
            .pass
            .get("rrLeft")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        left.min(self.free_rerolls)
    }
}
