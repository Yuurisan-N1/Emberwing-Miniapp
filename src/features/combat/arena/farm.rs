use anyhow::Result;
use serde_json::{json, Value};

use super::policy;
use crate::core::logger;
use crate::features::{foes_of, session_of, slots_len, Ctx};

pub async fn run(ctx: &mut Ctx<'_>) -> Result<()> {
    if !ctx.cfg.arena.farm_unranked {
        return Ok(());
    }
    let floor = ctx.cfg.arena.gold_floor;
    let gate_shut = {
        let mem = super::memory::ArenaMemory::load();
        let pool_floor = mem.floor();
        let team: f64 = policy::candidates(ctx.st, ctx.atk_limit(), 7)
            .iter()
            .take(3)
            .map(|d| d.pow())
            .sum();
        pool_floor > 0.0 && team < pool_floor * mem.ratio(ctx.cfg.arena.min_power_ratio)
    };
    if ctx.st.player.gold >= floor && !gate_shut {
        return Ok(());
    }
    let econ = crate::features::combat::arena::econ::Econ::of(ctx.st, ctx.league());
    let loss_pay = if econ.unr_loss_div > 0.0 {
        econ.win_gold / econ.unr_loss_div
    } else {
        0.0
    };
    let lim = ctx.atk_limit();
    let budget = ctx.cfg.arena.max_fights_per_cycle.max(1);
    let mut fights = 0u32;
    let mut earned = 0.0f64;
    let start_gold = ctx.st.player.gold;

    logger::ok(
        "arena farm",
        &format!(
            "{} so the unranked trips run, free gold with no trophies riding on them",
            if start_gold < floor {
                format!(
                    "gold {} is under the floor {}",
                    logger::num(start_gold),
                    logger::num(floor)
                )
            } else {
                format!(
                    "gold {} is on hand but the ranked pool still outclasses the team",
                    logger::num(start_gold)
                )
            }
        ),
    );
    logger::skip(
        "arena farm",
        &format!(
            "a win there pays {} gold and a loss still pays {} gold, so the farm is always ahead",
            logger::num(econ.win_gold),
            logger::num(loss_pay)
        ),
    );

    while fights < budget {
        let pool = policy::unranked_candidates(ctx.st, lim);
        if pool.is_empty() {
            logger::skip(
                "arena farm",
                &format!(
                    "no dragon has an unranked attack left, {} were used in this window",
                    logger::num(lim)
                ),
            );
            break;
        }
        let start = ctx
            .act("/battle/v2/start", json!({ "unranked": true }))
            .await?;
        if !start.ok {
            logger::skip("arena farm", &format!("start refused, {}", start.reason()));
            break;
        }
        let session = match session_of(&start) {
            Some(s) => s,
            None => {
                logger::skip("arena farm", "the start answer carried no session");
                break;
            }
        };
        let n = slots_len(&start, 3).min(pool.len());
        let foes_val = foes_of(&start);
        let foes = crate::features::combat::arena::sim::foe_units(&foes_val, n);
        let ids: Vec<i64> = pool.iter().take(n).map(|d| d.id).collect();
        let ordered = policy::order_slots(ctx.st, &ids, &foes, &crate::features::combat::arena::sim::Model::load());
        let slots: Vec<Value> = ordered
            .iter()
            .enumerate()
            .map(|(i, id)| json!({ "slot": i, "id": id }))
            .collect();
        let before = ctx.st.player.gold;
        let commit = ctx
            .act(
                "/battle/v2/commit",
                json!({ "sessionId": session, "slots": slots }),
            )
            .await?;
        if !commit.ok {
            logger::skip("arena farm", &format!("commit refused, {}", commit.reason()));
            break;
        }
        fights += 1;
        let won = commit
            .data
            .get("result")
            .and_then(|r| r.get("won"))
            .and_then(|v| v.as_bool());
        let gain = (ctx.st.player.gold - before).max(0.0);
        earned += gain;
        logger::ok(
            "arena farm",
            &format!(
                "{} against {} foes with {} attack together, gold plus {}, {} fights left in this cycle",
                if won == Some(true) { "win" } else { "loss" },
                logger::num(foes.len() as f64),
                logger::num(foes.iter().map(|f| f.atk).sum::<f64>()),
                logger::num(gain),
                budget.saturating_sub(fights)
            ),
        );
        if commit.bool_of("capped") {
            logger::skip("arena farm", "the unranked window cap is reached");
            break;
        }
        if ctx.st.player.gold >= floor {
            break;
        }
    }

    if fights > 0 {
        logger::ok(
            "arena farm",
            &format!(
                "{} unranked fights for {} gold, {} now on hand",
                logger::num(fights as f64),
                logger::num(earned),
                logger::num(ctx.st.player.gold)
            ),
        );
    }
    Ok(())
}
