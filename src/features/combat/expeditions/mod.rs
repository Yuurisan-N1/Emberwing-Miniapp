use anyhow::Result;
use serde_json::{json, Value};

use crate::core::logger;
use crate::core::state::parse_ts;
use crate::features::{pick_slots, session_of, slots_len, team_score, Ctx};

pub async fn run(ctx: &mut Ctx<'_>) -> Result<()> {
    if !ctx.cfg.expeditions.enabled {
        return Ok(());
    }
    collect(ctx).await?;
    start(ctx).await?;
    if ctx.cfg.expeditions.attack {
        attack(ctx).await?;
    }
    Ok(())
}

async fn collect(ctx: &mut Ctx<'_>) -> Result<()> {
    for _ in 0..10 {
        let now = ctx.now();
        let next = ctx
            .st
            .exp
            .mine
            .iter()
            .find(|t| {
                let end = parse_ts(t.get("endsAt").unwrap_or(&Value::Null));
                let status = t.get("status").and_then(|v| v.as_str()).unwrap_or("");
                (end > 0 && end <= now) || status == "done" || status == "finished"
            })
            .and_then(|t| t.get("id").and_then(|v| v.as_i64()));
        let id = match next {
            Some(v) => v,
            None => break,
        };
        let res = ctx.act("/island/exp/collect", json!({ "id": id })).await?;
        if res.ok {
            let got = res.get("gathered").and_then(|v| v.as_f64()).unwrap_or(0.0);
            logger::ok(
                "exp",
                &format!("trip {} came home with {}", id, logger::num(got)),
            );
        } else {
            logger::skip("exp", &format!("collect refused for trip {}, {}", id, res.reason()));
            break;
        }
    }
    Ok(())
}

async fn start(ctx: &mut Ctx<'_>) -> Result<()> {
    let quota_left = ctx.st.exp.quota.left;
    let running = ctx.st.exp.mine.len();
    if quota_left <= 0 {
        if running == 0 {
            logger::skip("exp", "the window quota is spent");
        }
        return Ok(());
    }

    let max_hours = ctx.st.isl_num(&["exp", "hoursMax"]).unwrap_or(8.0) as u32;
    let hours = ctx.cfg.expeditions.hours.clamp(1, max_hours.max(1));
    let types = ctx.st.isl_at(&["exp", "types"]);
    let allowed: Vec<String> = types
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let resource = if allowed.is_empty() || allowed.contains(&ctx.cfg.expeditions.resource) {
        ctx.cfg.expeditions.resource.clone()
    } else {
        allowed[0].clone()
    };

    let team = ctx
        .st
        .isl_num(&["exp", "team"])
        .or_else(|| ctx.st.isl_num(&["raid", "team"]))
        .unwrap_or(3.0) as usize;
    let lim = ctx.atk_limit();
    let defenders = defender_ids(ctx);
    let mut free = ctx.st.free_dragons(lim, false);
    free.retain(|d| !defenders.contains(&d.id));
    free.sort_by(|a, b| a.pow().partial_cmp(&b.pow()).unwrap_or(std::cmp::Ordering::Equal));
    if free.len() < team {
        logger::skip(
            "exp",
            &format!(
                "{} dragons are free and a trip needs {}, the village guards stay home",
                free.len(),
                team
            ),
        );
        return Ok(());
    }
    let ids: Vec<i64> = free.iter().take(team).map(|d| d.id).collect();

    let res = ctx
        .act(
            "/island/exp/start",
            json!({ "type": resource, "hours": hours, "ids": ids }),
        )
        .await?;
    if res.ok {
        logger::ok(
            "exp",
            &format!(
                "{} hour {} trip sent with {} dragons, quota {} left",
                hours,
                resource,
                team,
                quota_left - 1
            ),
        );
    } else {
        logger::skip("exp", &format!("the trip start was refused, {}", res.reason()));
    }
    Ok(())
}

async fn attack(ctx: &mut Ctx<'_>) -> Result<()> {
    if ctx.st.exp.quota.left <= 0 {
        return Ok(());
    }
    let list = ctx.get("/island/exp/list").await?;
    if !list.ok {
        if !crate::features::is_hold(&list.reason()) {
            logger::skip("exp", &format!("the trip list was refused, {}", list.reason()));
        }
        return Ok(());
    }
    ctx.st.exp_list = list.data.clone();

    let fee = list.num_of("fee");
    let max_attacks = ctx.st.isl_num(&["exp", "attackMax"]).unwrap_or(5.0) as usize;
    let ratio = super::arena::memory::ArenaMemory::load().ratio(ctx.cfg.expeditions.min_power_ratio);

    let mut pickable: Vec<(i64, String, f64, Vec<Value>, f64, f64)> = Vec::new();
    let mut guarded = 0usize;
    for c in list.arr("cards").iter() {
        if c.get("status").and_then(|v| v.as_str()).unwrap_or("") != "running" {
            continue;
        }
        let id = match c.get("id").and_then(|v| v.as_i64()) {
            Some(v) => v,
            None => continue,
        };
        let name = c
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("keeper")
            .to_string();
        let grab = c.get("grab").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let defenders: Vec<Value> = c
            .get("defenders")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let n = defenders.len().clamp(1, 5);
        let slots = pick_slots(ctx.st, &defenders, n, false);
        let (mine, theirs) = team_score(ctx.st, &slots, &defenders);
        if slots.is_empty() || theirs <= 0.0 || mine < theirs * ratio {
            guarded += 1;
            continue;
        }
        pickable.push((id, name, grab, defenders, mine, theirs));
    }
    pickable.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

    if pickable.is_empty() {
        logger::skip(
            "exp",
            &format!(
                "{} running trips all guard better than the team, the fee stays in the bag",
                guarded
            ),
        );
        return Ok(());
    }

    let mut attacked = 0usize;
    for (id, name, grab, defenders, mine, theirs) in pickable.into_iter().take(max_attacks) {
        if ctx.st.exp.quota.left <= 0 {
            break;
        }
        if ctx.st.player.gold < fee {
            logger::skip(
                "exp",
                &format!("the raid fee of {} gold is not on hand", logger::num(fee)),
            );
            break;
        }

        let res = ctx.act("/island/exp/attack", json!({ "id": id })).await?;
        if !res.ok {
            logger::skip("exp", &format!("the raid on {} was refused, {}", name, res.reason()));
            if crate::features::is_hold(&res.reason()) {
                break;
            }
            continue;
        }
        let session = match session_of(&res) {
            Some(s) => s,
            None => continue,
        };
        let n = slots_len(&res, defenders.len().clamp(1, 5));
        let foes = crate::features::foes_of(&res);
        let live_foes = if foes.is_empty() { defenders.clone() } else { foes };
        let slots = pick_slots(ctx.st, &live_foes, n, false);
        if slots.is_empty() {
            let _ = ctx
                .act("/island/battle/leave", json!({ "sessionId": session }))
                .await;
            continue;
        }
        let (live_mine, live_theirs) = team_score(ctx.st, &slots, &live_foes);
        if live_theirs > 0.0 && live_mine < live_theirs * ratio {
            let _ = ctx
                .act("/island/battle/leave", json!({ "sessionId": session }))
                .await;
            logger::skip(
                "exp",
                &format!(
                    "{} fields {} against the promised {}, the field was left",
                    name,
                    logger::num(live_theirs),
                    logger::num(theirs)
                ),
            );
            continue;
        }

        let commit = ctx
            .act(
                "/island/exp/commit",
                json!({ "sessionId": session, "slots": slots }),
            )
            .await?;
        if commit.ok {
            attacked += 1;
            let won = commit
                .data
                .get("result")
                .and_then(|r| r.get("won"))
                .and_then(|v| v.as_bool());
            logger::ok(
                "exp",
                &format!(
                    "{} raided, team {} against {} with {} on the line, {}",
                    name,
                    logger::num(mine),
                    logger::num(theirs),
                    logger::num(grab),
                    if won == Some(true) {
                        loot_of(&commit)
                    } else {
                        "lost and nothing was taken".to_string()
                    }
                ),
            );
        } else {
            logger::skip(
                "exp",
                &format!("the raid commit on {} was refused, {}", name, commit.reason()),
            );
            if crate::features::is_hold(&commit.reason()) {
                break;
            }
        }
    }

    if attacked == 0 {
        logger::skip("exp", "no trip was taken this cycle");
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
                return parts.join(", ");
            }
        }
    }
    "nothing taken".to_string()
}

fn defender_ids(ctx: &Ctx<'_>) -> Vec<i64> {
    ctx.st
        .isl_at(&["raid", "defIds"])
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_i64()).collect())
        .unwrap_or_default()
}
