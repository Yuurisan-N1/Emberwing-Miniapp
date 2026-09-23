use anyhow::Result;
use serde_json::{json, Value};

use crate::features::{session_of, slots_len, Ctx};
use crate::core::logger;

pub async fn run(ctx: &mut Ctx<'_>) -> Result<()> {
    if !ctx.cfg.raids.enabled {
        return Ok(());
    }
    if ctx.cfg.raids.chest {
        chest(ctx).await?;
    }
    if ctx.cfg.raids.defense {
        defense(ctx).await?;
    }
    if ctx.cfg.raids.search {
        search(ctx).await?;
    }
    Ok(())
}

async fn chest(ctx: &mut Ctx<'_>) -> Result<()> {
    let spec = ctx.st.isl_at(&["raid", "chest"]);
    let ready = spec.get("ready").and_then(|v| v.as_bool()).unwrap_or(false);
    let claimed = spec.get("claimed").and_then(|v| v.as_bool()).unwrap_or(false);
    if !ready || claimed {
        if !claimed {
            let wins = spec.get("wins").and_then(|v| v.as_i64()).unwrap_or(0);
            let need = spec.get("need").and_then(|v| v.as_i64()).unwrap_or(0);
            logger::skip("raid", &format!("chest {}/{} wins", wins, need));
        }
        return Ok(());
    }
    let res = ctx.act("/island/raid/chest", json!({})).await?;
    if res.ok {
        logger::ok("raid", "daily chest opened");
    } else {
        logger::skip("raid", &format!("chest: {}", res.reason()));
    }
    Ok(())
}

async fn defense(ctx: &mut Ctx<'_>) -> Result<()> {
    let res = ctx.get("/island/raid/defense").await?;
    if !res.ok {
        if !crate::features::is_hold(&res.reason()) {
            logger::skip("raid", &format!("defense state: {}", res.reason()));
        }
        return Ok(());
    }
    ctx.st.raid_defense = res.data.clone();

    let team = res.num_of("team").max(1.0) as usize;
    let have = res.arr("ids");
    let cap = res.num_of("cap") as usize;
    if !have.is_empty() && (cap == 0 || have.len() >= team.min(cap)) {
        return Ok(());
    }
    let exp_team = ctx
        .st
        .isl_num(&["exp", "team"])
        .or_else(|| ctx.st.isl_num(&["raid", "team"]))
        .unwrap_or(3.0) as usize;
    let spare = ctx.st.dragons.len().saturating_sub(exp_team);
    if spare == 0 {
        logger::skip("raid", "no dragon to spare for the watch, expeditions come first");
        return Ok(());
    }
    let mut pool = ctx.st.dragons.clone();
    pool.retain(|d| !d.away() && d.listed == 0);
    pool.sort_by(|a, b| b.pow().partial_cmp(&a.pow()).unwrap_or(std::cmp::Ordering::Equal));
    let ids: Vec<i64> = pool.iter().take(team.min(spare)).map(|d| d.id).collect();
    if ids.is_empty() {
        return Ok(());
    }
    let set = ctx
        .act("/island/raid/defense", json!({ "ids": ids }))
        .await?;
    crate::features::report(
        "raid",
        &set,
        &format!("{} defenders set", ids.len()),
    );
    Ok(())
}

struct Board {
    p: f64,
    bar: f64,
    name: String,
    session: Value,
    ids: Vec<i64>,
    foes: Vec<super::arena::sim::Unit>,
}

fn stakes(spec: &Value) -> (f64, f64) {
    let pick = |k: &str| {
        spec.get("stakes")
            .and_then(|s| s.get(k))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0)
    };
    (pick("win"), pick("lose"))
}

fn pct(x: f64) -> String {
    format!("{:.0}%", x * 100.0)
}

fn signed(x: f64) -> String {
    if x < 0.0 {
        format!("-{}", logger::num(-x))
    } else {
        format!("+{}", logger::num(x))
    }
}

fn raid_result(name: &str, p: f64, res: &crate::core::http::Res) -> String {
    let won = res
        .data
        .get("result")
        .and_then(|r| r.get("won"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let troph = res
        .data
        .get("rewards")
        .or_else(|| res.data.get("result").and_then(|r| r.get("rewards")))
        .and_then(|r| r.get("trophies"))
        .and_then(|v| v.as_f64());
    let delta = match troph {
        Some(t) => format!(" {} trophies", signed(t)),
        None => String::new(),
    };
    if won {
        format!("beat {}, odds were {}{}, {}", name, pct(p), delta, loot_of(res))
    } else {
        format!("lost to {} at odds {}{}", name, pct(p), delta)
    }
}

async fn search(ctx: &mut Ctx<'_>) -> Result<()> {
    let fee = ctx.st.isl_num(&["raid", "searchGold"]).unwrap_or(0.0);
    if ctx.st.player.gold < fee {
        logger::skip("raid", &format!("search costs {} gold", logger::num(fee)));
        return Ok(());
    }
    let pool = super::arena::policy::candidates(ctx.st, ctx.atk_limit(), 7);
    if pool.is_empty() {
        logger::skip("raid", "no dragon free to raid with");
        return Ok(());
    }

    let model = super::arena::sim::Model::load();
    let calib = super::arena::model::Calib::load();
    let runs = ctx.cfg.arena.sim_runs.max(40) as usize;
    let tries = ctx.cfg.arena.max_rerolls.max(1);
    let mut seen: Vec<i64> = Vec::new();
    let mut best: Option<Board> = None;
    let mut last: Option<Value> = None;

    for round in 0..tries {
        if round > 0 && ctx.st.player.gold - fee < ctx.cfg.arena.gold_floor {
            logger::skip(
                "raid",
                &format!(
                    "a wider search costs {} gold and the floor holds {}, the look stops",
                    logger::num(fee),
                    logger::num(ctx.cfg.arena.gold_floor)
                ),
            );
            break;
        }
        let body = if seen.is_empty() {
            json!({})
        } else {
            json!({ "skip": seen })
        };
        let res = ctx.act("/island/raid/search", body).await?;
        if !res.ok {
            logger::skip("raid", &format!("search: {}", res.reason()));
            break;
        }
        let session = match session_of(&res) {
            Some(s) => s,
            None => break,
        };
        last = Some(session.clone());
        let n = slots_len(&res, 3);
        let spec = res.data.get("raid").cloned().unwrap_or(Value::Null);
        let name = spec
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("a base")
            .to_string();
        let (win, lose) = stakes(&spec);
        let bar = if win + lose > 0.0 {
            lose / (win + lose)
        } else {
            0.5
        };
        if let Some(tid) = spec.get("tid").and_then(|v| v.as_i64()) {
            seen.push(tid);
        }
        let foes_val = crate::features::foes_of(&res);
        let foes = super::arena::sim::foe_units(&foes_val, n);
        if foes.is_empty() {
            break;
        }
        let seed = (crate::core::http::server_now_ms(ctx.st) as u64)
            .wrapping_add((round as u64 + 1).wrapping_mul(7919));
        let (p, ids) = match super::arena::policy::plan(
            ctx.st, &pool, &foes, &model, &calib, n, runs, seed,
        ) {
            Some(pl) => (pl.p_sim, pl.ids),
            None => (0.0, Vec::new()),
        };
        if best.as_ref().map(|b| p > b.p).unwrap_or(true) {
            best = Some(Board {
                p,
                bar,
                name: name.clone(),
                session,
                ids,
                foes,
            });
        }
        if p >= bar {
            logger::ok(
                "raid",
                &format!(
                    "{} taken, board falls {} of the time, needs {}",
                    name,
                    pct(p),
                    pct(bar)
                ),
            );
            break;
        }
        logger::skip(
            "raid",
            &format!("{} odds {} under {}, rerolling", name, pct(p), pct(bar)),
        );
    }

    let board = match best {
        Some(b) => b,
        None => {
            logger::skip("raid", "the scouts came back with nothing");
            return Ok(());
        }
    };
    let stand = last.clone().unwrap_or_else(|| board.session.clone());
    if board.ids.is_empty() {
        logger::skip("raid", "no eligible dragon for this raid, the board is dropped");
        walk_away(ctx, stand).await?;
        return Ok(());
    }
    if board.p < board.bar {
        logger::skip(
            "raid",
            &format!(
                "nothing clears {} in {} looks, best {} on {}, walking off",
                pct(board.bar),
                tries,
                pct(board.p),
                board.name
            ),
        );
        walk_away(ctx, stand).await?;
        return Ok(());
    }
    let ids = super::arena::policy::order_slots(ctx.st, &board.ids, &board.foes, &model);
    let slots: Vec<Value> = ids
        .iter()
        .enumerate()
        .map(|(i, id)| json!({ "slot": i, "id": id }))
        .collect();
    if slots.is_empty() {
        logger::skip("raid", "no eligible dragon for this raid");
        return Ok(());
    }
    let commit = ctx
        .act(
            "/island/raid/commit",
            json!({ "sessionId": board.session, "slots": slots }),
        )
        .await?;
    if !commit.ok {
        logger::skip("raid", &format!("commit: {}", commit.reason()));
        let s = last.clone().unwrap_or_else(|| board.session.clone());
        walk_away(ctx, s).await?;
        return Ok(());
    }
    logger::ok("raid", &raid_result(&board.name, board.p, &commit));
    Ok(())
}

async fn walk_away(ctx: &mut Ctx<'_>, session: Value) -> Result<()> {
    let res = ctx
        .act("/island/battle/leave", json!({ "sessionId": session }))
        .await?;
    if res.ok {
        logger::skip("raid", "walked off the board, the back button cuts no trophies");
    } else {
        logger::skip("raid", &format!("leave: {}", res.reason()));
    }
    Ok(())
}

fn loot_of(res: &crate::core::http::Res) -> String {
    for key in ["loot", "rewards", "rw", "got"] {
        if let Some(Value::Object(m)) = res.data.get(key) {
            let parts: Vec<String> = m
                .iter()
                .filter_map(|(k, v)| {
                    v.as_f64()
                        .filter(|n| *n > 0.0)
                        .map(|n| format!("{} {}", k, logger::num(n)))
                })
                .collect();
            if !parts.is_empty() {
                return parts.join(" + ");
            }
        }
    }
    "nothing taken".to_string()
}
