use anyhow::Result;
use serde_json::{json, Value};

use crate::features::{pick_slots, session_of, slots_len, Ctx};
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

async fn search(ctx: &mut Ctx<'_>) -> Result<()> {
    let fee = ctx.st.isl_num(&["raid", "searchGold"]).unwrap_or(0.0);
    if ctx.st.player.gold < fee {
        logger::skip("raid", &format!("search costs {} gold", logger::num(fee)));
        return Ok(());
    }
    let lim = ctx.atk_limit();
    if ctx.st.free_dragons(lim, true).is_empty() {
        logger::skip("raid", "no dragon free to raid with");
        return Ok(());
    }

    let ratio = super::arena::memory::ArenaMemory::load().ratio(ctx.cfg.raids.min_power_ratio);

    for _ in 0..3 {
        let res = ctx.act("/island/raid/search", json!({})).await?;
        if !res.ok {
            logger::skip("raid", &format!("search: {}", res.reason()));
            break;
        }
        let session = match session_of(&res) {
            Some(s) => s,
            None => break,
        };
        let n = slots_len(&res, 3);
        let foes = crate::features::foes_of(&res);
        let slots = pick_slots(ctx.st, &foes, n, true);
        if slots.is_empty() {
            let _ = ctx
                .act("/island/battle/leave", json!({ "sessionId": session }))
                .await;
            logger::skip("raid", "no eligible dragon for this raid");
            break;
        }
        let (mine, theirs) = crate::features::team_score(ctx.st, &slots, &foes);
        if theirs > 0.0 && mine < theirs * ratio {
            let _ = ctx
                .act("/island/battle/leave", json!({ "sessionId": session }))
                .await;
            logger::skip(
                "raid",
                &format!(
                    "base too strong: {} vs {}, left without fighting",
                    logger::num(mine),
                    logger::num(theirs)
                ),
            );
            break;
        }
        let commit = ctx
            .act(
                "/island/raid/commit",
                json!({ "sessionId": session, "slots": slots }),
            )
            .await?;
        if commit.ok {
            logger::ok("raid", &format!("base raided, {}", loot_of(&commit)));
        } else {
            logger::skip("raid", &format!("commit: {}", commit.reason()));
            break;
        }
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
