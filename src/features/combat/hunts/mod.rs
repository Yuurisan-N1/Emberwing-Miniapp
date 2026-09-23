use anyhow::Result;
use serde_json::{json, Value};

use crate::features::{pick_slots, session_of, slots_len, Ctx};
use crate::core::logger;
use crate::core::state::parse_ts;

pub async fn run(ctx: &mut Ctx<'_>) -> Result<()> {
    if ctx.cfg.hunts.enabled {
        hunt(ctx).await?;
    }
    if ctx.cfg.monsters.enabled {
        monster(ctx).await?;
    }
    if ctx.cfg.camps.enabled {
        camps(ctx).await?;
    }
    if ctx.cfg.areas.enabled {
        areas(ctx).await?;
    }
    Ok(())
}

async fn hunt(ctx: &mut Ctx<'_>) -> Result<()> {
    let hunt = ctx.st.hunt.clone();
    if hunt.is_null() {
        return Ok(());
    }
    let unlocked = hunt
        .get("unlocked")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let hall_req = hunt.get("hall").and_then(|v| v.as_i64()).unwrap_or(3);
    if !unlocked {
        logger::skip(
            "hunt",
            &format!("locked, hall {} required, have {}", hall_req, ctx.st.hall_lvl()),
        );
        return Ok(());
    }

    let cur = hunt.get("cur").cloned().unwrap_or(Value::Null);
    if cur.is_null() {
        let res = ctx.act("/island/hunt/start", json!({})).await?;
        crate::features::report("hunt", &res, "island taken");
        return Ok(());
    }

    let fee = hunt.get("fee").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let free = hunt.get("free").and_then(|v| v.as_bool()).unwrap_or(false);
    if !free && ctx.st.player.gold < fee {
        logger::skip("hunt", &format!("next hunt costs {} gold", logger::num(fee)));
        return Ok(());
    }

    let found = hunt
        .get("found")
        .and_then(|v| v.as_bool())
        .or_else(|| cur.get("found").and_then(|v| v.as_bool()))
        .unwrap_or(false);
    if !found && !scout(ctx).await? {
        return Ok(());
    }

    let attack = ctx.act("/island/hunt/attack", json!({})).await?;
    if !attack.ok {
        logger::skip("hunt", &format!("attack: {}", attack.reason()));
        return Ok(());
    }
    if let Some(session) = session_of(&attack) {
        let n = slots_len(&attack, 3);
        let foes = crate::features::foes_of(&attack);
        let slots = pick_slots(ctx.st, &foes, n, false);
        if slots.is_empty() {
            let _ = ctx
                .act("/island/battle/leave", json!({ "sessionId": session }))
                .await;
            return Ok(());
        }
        let commit = ctx
            .act(
                "/island/hunt/commit",
                json!({ "sessionId": session, "slots": slots }),
            )
            .await?;
        if commit.ok {
            logger::ok("hunt", &format!("boss down, {}", summary(&commit)));
        } else {
            logger::skip("hunt", &format!("commit: {}", commit.reason()));
        }
    } else {
        logger::ok("hunt", "boss engaged");
    }
    Ok(())
}

async fn scout(ctx: &mut Ctx<'_>) -> Result<bool> {
    for _ in 0..20 {
        let res = ctx.act("/island/hunt/scout", json!({})).await?;
        if !res.ok {
            logger::skip("hunt", &format!("scout: {}", res.reason()));
            return Ok(false);
        }
        let hunt = res.get("hunt").cloned().unwrap_or(Value::Null);
        let island = hunt.get("island").cloned().unwrap_or(Value::Null);
        if island.is_null() {
            logger::ok("hunt", "every island walked, the hunt is over");
            return Ok(false);
        }
        if hunt
            .get("found")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            logger::ok("hunt", "boss found, the attack comes next");
            return Ok(true);
        }
    }
    logger::skip("hunt", "the boss is still hiding, scouting goes on next cycle");
    Ok(false)
}

async fn monster(ctx: &mut Ctx<'_>) -> Result<()> {
    let cur = ctx.st.monster.cur.clone();
    if cur.is_null() {
        let next = parse_ts(&ctx.st.monster.next_at);
        if next > 0 {
            let left = (next - ctx.now()) / 1000;
            logger::skip("monster", &format!("next boss in {}", logger::clock(left)));
        }
        return Ok(());
    }
    let id = cur
        .get("id")
        .or_else(|| cur.get("monsterId"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let team = ctx.st.isl_num(&["raid", "team"]).unwrap_or(3.0) as usize;
    let res = ctx.act("/island/monster/start", json!({ "id": id })).await?;
    if !res.ok {
        logger::skip("monster", &format!("start: {}", res.reason()));
        return Ok(());
    }
    let session = match session_of(&res) {
        Some(s) => s,
        None => return Ok(()),
    };
    let n = slots_len(&res, team);
    let foes = crate::features::foes_of(&res);
    let slots = pick_slots(ctx.st, &foes, n, false);
    if slots.is_empty() {
        let _ = ctx
            .act("/island/battle/leave", json!({ "sessionId": session }))
            .await;
        return Ok(());
    }
    let commit = ctx
        .act(
            "/island/monster/commit",
            json!({ "sessionId": session, "slots": slots }),
        )
        .await?;
    if commit.ok {
        logger::ok("monster", &format!("boss killed, {}", summary(&commit)));
    } else {
        logger::skip("monster", &format!("commit: {}", commit.reason()));
    }
    Ok(())
}

async fn camps(ctx: &mut Ctx<'_>) -> Result<()> {
    let camps = ctx.st.isl_at(&["camps"]);
    let cleared = ctx.st.isl_at(&["campsCleared"]);
    let cleared_ids: Vec<String> = cleared
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let list = match camps.as_array() {
        Some(l) => l.clone(),
        None => return Ok(()),
    };
    let hall = ctx.st.hall_lvl();

    for camp in list {
        let id = camp.get("id").and_then(|v| v.as_str()).unwrap_or("");
        if id.is_empty() || cleared_ids.iter().any(|c| c == id) {
            continue;
        }
        let need_hall = camp.get("hall").and_then(|v| v.as_i64()).unwrap_or(0);
        if hall < need_hall {
            continue;
        }
        let gold = camp.get("gold").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if ctx.st.player.gold < gold {
            logger::skip(
                "camp",
                &format!("{} needs {} gold", id, logger::num(gold)),
            );
            continue;
        }
        let n = camp.get("n").and_then(|v| v.as_u64()).unwrap_or(3) as usize;
        let res = ctx.act("/island/camp/start", json!({ "campId": id })).await?;
        if !res.ok {
            logger::skip("camp", &format!("{}: {}", id, res.reason()));
            continue;
        }
        let session = match session_of(&res) {
            Some(s) => s,
            None => continue,
        };
        let foes = crate::features::foes_of(&res);
        let slots = pick_slots(ctx.st, &foes, n, false);
        if slots.is_empty() {
            let _ = ctx
                .act("/island/battle/leave", json!({ "sessionId": session }))
                .await;
            logger::skip("camp", "no dragon free for the rescue");
            continue;
        }
        let commit = ctx
            .act(
                "/island/camp/commit",
                json!({ "sessionId": session, "slots": slots }),
            )
            .await?;
        if commit.ok {
            logger::ok("camp", &format!("{} cleared, {}", id, summary(&commit)));
        } else {
            logger::skip("camp", &format!("commit {}: {}", id, commit.reason()));
            break;
        }
    }
    Ok(())
}

async fn areas(ctx: &mut Ctx<'_>) -> Result<()> {
    let areas = ctx.st.isl_at(&["areas"]);
    let open = ctx.st.isl_at(&["areasOpen"]);
    let prog_map = ctx.st.isl_at(&["roadProg"]);
    let open_ids: Vec<String> = open
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let list = match areas.as_array() {
        Some(l) => l.clone(),
        None => return Ok(()),
    };
    let hall = ctx.st.hall_lvl();
    let mut prev_open = true;

    for area in list {
        let id = area.get("id").and_then(|v| v.as_str()).unwrap_or("");
        if id.is_empty() {
            continue;
        }
        if open_ids.iter().any(|o| o == id) {
            prev_open = true;
            continue;
        }
        let need_hall = area.get("hall").and_then(|v| v.as_i64()).unwrap_or(0);
        let ready = prev_open && hall >= need_hall;
        prev_open = false;
        if !ready {
            continue;
        }

        let prog = prog_map.get(id).and_then(|v| v.as_i64()).unwrap_or(0);
        let road = area.get("road").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let cell = match road.get(prog.max(0) as usize) {
            Some(c) => c.clone(),
            None => continue,
        };
        let kind = cell.get("t").and_then(|v| v.as_str()).unwrap_or("f");

        if kind == "f" {
            let res = ctx.act("/island/area/start", json!({ "areaId": id })).await?;
            if !res.ok {
                logger::skip("area", &format!("{} fight: {}", id, res.reason()));
                continue;
            }
            let session = match session_of(&res) {
                Some(s) => s,
                None => continue,
            };
            let team = ctx.st.isl_num(&["raid", "team"]).unwrap_or(3.0) as usize;
            let n = slots_len(&res, team);
            let foes = crate::features::foes_of(&res);
            let slots = pick_slots(ctx.st, &foes, n, false);
            if slots.is_empty() {
                let _ = ctx
                    .act("/island/battle/leave", json!({ "sessionId": session }))
                    .await;
                continue;
            }
            let commit = ctx
                .act(
                    "/island/area/commit",
                    json!({ "sessionId": session, "slots": slots }),
                )
                .await?;
            if commit.ok {
                logger::ok("area", &format!("{} road cell {} cleared", id, prog + 1));
            } else {
                logger::skip("area", &format!("commit {}: {}", id, commit.reason()));
                break;
            }
        } else {
            if !cell_affordable(ctx, &cell) {
                logger::skip("area", &format!("{} road cell {} not paid yet", id, prog + 1));
                continue;
            }
            let res = ctx.act("/island/area/collect", json!({ "areaId": id })).await?;
            if res.ok {
                let won = res.bool_of("areaUnlocked");
                logger::ok(
                    "area",
                    &format!(
                        "{} road cell {} collected{}",
                        id,
                        prog + 1,
                        if won { ", zone opened" } else { "" }
                    ),
                );
            } else {
                logger::skip("area", &format!("collect {}: {}", id, res.reason()));
            }
        }
    }
    Ok(())
}

fn cell_affordable(ctx: &Ctx<'_>, cell: &Value) -> bool {
    for (key, res) in [
        ("gold", "gold"),
        ("logs", "logs"),
        ("meat", "meat"),
        ("potion", "potion"),
    ] {
        let need = cell.get(key).and_then(|v| v.as_f64()).unwrap_or(0.0);
        if need > 0.0 && ctx.st.res(res) < need {
            return false;
        }
    }
    true
}

fn summary(res: &crate::core::http::Res) -> String {
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
    "cleared".to_string()
}
