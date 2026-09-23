use anyhow::Result;
use serde_json::{json, Value};

use crate::features::Ctx;
use crate::core::logger;

pub async fn run(ctx: &mut Ctx<'_>) -> Result<()> {
    if !ctx.cfg.gear.enabled {
        return Ok(());
    }
    let gear = ctx.st.isl_at(&["gear"]);
    let on = gear.get("on").and_then(|v| v.as_i64()).unwrap_or(0) != 0;
    if !on || gear.is_null() {
        return Ok(());
    }
    if ctx.cfg.gear.craft {
        craft(ctx).await?;
    }
    if ctx.cfg.gear.equip {
        equip(ctx).await?;
    }
    if ctx.cfg.gear.level_up {
        level_up(ctx).await?;
    }
    if ctx.cfg.gear.fuse {
        fuse(ctx).await?;
        salvage(ctx).await?;
        dismantle(ctx).await?;
    }
    Ok(())
}

async fn craft(ctx: &mut Ctx<'_>) -> Result<()> {
    let cfg = ctx.st.isl_at(&["gearCfg"]);
    let hall_req = cfg.get("hall").and_then(|v| v.as_i64()).unwrap_or(5);
    let hall = ctx.st.hall_lvl();
    if hall < hall_req {
        logger::skip("gear", &format!("forge needs hall {}", hall_req));
        return Ok(());
    }
    let building = ctx
        .st
        .buildings
        .iter()
        .find(|b| b.kind == "gear" && b.broken == 0)
        .map(|b| b.id);
    let building = match building {
        Some(b) => b,
        None => {
            logger::skip("gear", "no forge standing");
            return Ok(());
        }
    };

    let rars = cfg
        .get("rars")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let types = cfg
        .get("types")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mats = ctx.st.isl_at(&["gear", "mats"]);
    let gold = ctx.st.player.gold;
    let gate_epic = cfg.get("gateEpic").and_then(|v| v.as_i64()).unwrap_or(5);
    let gate_leg = cfg.get("gateLeg").and_then(|v| v.as_i64()).unwrap_or(10);
    let crafts = ctx
        .st
        .isl_at(&["gear", "crafts"])
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);

    let mut best: Option<(i64, String, String, f64, f64)> = None;
    for rar in rars.iter().filter_map(|v| v.as_i64()) {
        let gate = if rar >= 4 {
            gate_leg
        } else if rar == 3 {
            gate_epic
        } else {
            0
        };
        if hall < gate {
            continue;
        }
        let spec = cfg
            .get("craft")
            .and_then(|c| c.get(rar.to_string()))
            .cloned()
            .unwrap_or(Value::Null);
        if spec.is_null() {
            continue;
        }
        let mat = spec.get("mat").and_then(|v| v.as_str()).unwrap_or("");
        let need = spec.get("n").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let price = spec.get("gold").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let have = mats.get(mat).and_then(|v| v.as_f64()).unwrap_or(0.0);
        if mat.is_empty() || have < need || gold < price {
            continue;
        }
        let kind = types
            .get(crafts % types.len().max(1))
            .and_then(|v| v.as_str())
            .unwrap_or("sword")
            .to_string();
        best = Some((rar, kind, mat.to_string(), need, price));
    }

    let (rar, kind, mat, need, price) = match best {
        Some(v) => v,
        None => {
            logger::skip("gear", "no craft affordable yet");
            return Ok(());
        }
    };
    let res = ctx
        .act(
            "/island/gear/craft",
            json!({ "buildingId": building, "type": kind, "rarity": rar }),
        )
        .await?;
    crate::features::report(
        "gear",
        &res,
        &format!(
            "rarity {} {} started, {} {} + {} gold",
            rar,
            kind,
            need,
            mat,
            logger::num(price)
        ),
    );
    Ok(())
}

async fn equip(ctx: &mut Ctx<'_>) -> Result<()> {
    let items = ctx
        .st
        .isl_at(&["gear", "items"])
        .as_array()
        .cloned()
        .unwrap_or_default();

    for item in items {
        let id = item.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
        let kind = item.get("type").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let worn = item
            .get("dragonId")
            .or_else(|| item.get("dragon_id"))
            .map(|v| !v.is_null())
            .unwrap_or(false);
        if id == 0 || kind.is_empty() || worn {
            continue;
        }
        let mut cands: Vec<(i64, f64)> = ctx
            .st
            .dragons
            .iter()
            .filter(|d| !d.away() && !wears(d, &kind))
            .map(|d| (d.id, d.pow()))
            .collect();
        cands.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let Some((dragon_id, _)) = cands.first().cloned() else { continue };
        let res = ctx
            .act(
                "/island/gear/equip",
                json!({ "gearId": id, "dragonId": dragon_id }),
            )
            .await?;
        crate::features::report("gear", &res, &format!("{} worn by dragon {}", kind, dragon_id));
    }
    Ok(())
}

async fn level_up(ctx: &mut Ctx<'_>) -> Result<()> {
    for _ in 0..20 {
        let gems = ctx
            .st
            .isl_at(&["gear", "gems"])
            .as_f64()
            .unwrap_or(0.0);
        let gold = ctx.st.player.gold;
        let items = ctx
            .st
            .isl_at(&["gear", "items"])
            .as_array()
            .cloned()
            .unwrap_or_default();
        let mut pick: Option<(i64, f64, f64, String)> = None;
        for item in items.iter() {
            let worn = item
                .get("dragonId")
                .or_else(|| item.get("dragon_id"))
                .map(|v| !v.is_null())
                .unwrap_or(false);
            if !worn {
                continue;
            }
            let next = item.get("next").cloned().unwrap_or(Value::Null);
            let need_gems = next.get("gems").and_then(|v| v.as_f64()).unwrap_or(f64::MAX);
            let need_gold = next.get("gold").and_then(|v| v.as_f64()).unwrap_or(f64::MAX);
            if need_gems <= gems && need_gold <= gold {
                pick = Some((
                    item.get("id").and_then(|v| v.as_i64()).unwrap_or(0),
                    need_gems,
                    need_gold,
                    item.get("type").and_then(|v| v.as_str()).unwrap_or("gear").to_string(),
                ));
                break;
            }
        }
        let (id, need_gems, need_gold, kind) = match pick {
            Some(v) => v,
            None => break,
        };
        let res = ctx
            .act("/island/gear/levelup", json!({ "gearId": id }))
            .await?;
        crate::features::report(
            "gear",
            &res,
            &format!(
                "{} raised, {} gems {} gold",
                kind,
                logger::num(need_gems),
                logger::num(need_gold)
            ),
        );
    }
    Ok(())
}

async fn fuse(ctx: &mut Ctx<'_>) -> Result<()> {
    let cfg = ctx.st.isl_at(&["gearCfg"]);
    let per = cfg.get("fuseN").and_then(|v| v.as_f64()).unwrap_or(4.0).max(1.0);
    let mats = ctx.st.isl_at(&["gear", "mats"]);
    let order = cfg
        .get("mats")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    for _ in 0..10 {
        let mut pick: Option<String> = None;
        for (i, mat) in order.iter().enumerate() {
            if i + 1 >= order.len() {
                break;
            }
            let name = mat.as_str().unwrap_or("");
            let have = mats.get(name).and_then(|v| v.as_f64()).unwrap_or(0.0);
            if have >= per {
                pick = Some(name.to_string());
                break;
            }
        }
        let mat = match pick {
            Some(m) => m,
            None => break,
        };
        let res = ctx
            .act("/island/gear/fuse", json!({ "mat": mat, "n": 1 }))
            .await?;
        if res.ok {
            logger::ok(
                "gear",
                &format!("{} fused, {} into one", logger::num(per), mat),
            );
        } else {
            logger::skip("gear", &format!("fuse {}: {}", mat, res.reason()));
            break;
        }
    }
    Ok(())
}

async fn salvage(ctx: &mut Ctx<'_>) -> Result<()> {
    let order: Vec<String> = ctx
        .st
        .isl_at(&["gearCfg"])
        .get("mats")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    if order.len() < 2 {
        return Ok(());
    }
    for _ in 0..3 {
        let mats = ctx.st.isl_at(&["gear", "mats"]);
        let mut pick: Option<String> = None;
        for i in (1..order.len()).rev() {
            let have = mats.get(&order[i]).and_then(|v| v.as_f64()).unwrap_or(0.0);
            let below = mats
                .get(&order[i - 1])
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            if have >= 2.0 && below <= 0.0 {
                pick = Some(order[i].clone());
                break;
            }
        }
        let mat = match pick {
            Some(m) => m,
            None => return Ok(()),
        };
        let res = ctx
            .act("/island/gear/salvage", json!({ "mat": mat, "n": 1 }))
            .await?;
        if res.ok {
            let got = res.data.get("got").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let to = res.data.get("to").and_then(|v| v.as_str()).unwrap_or("");
            logger::ok(
                "gear",
                &format!("{} salvaged into {} {}", mat, logger::num(got), to),
            );
        } else {
            logger::skip("gear", &format!("salvage {}: {}", mat, res.reason()));
            return Ok(());
        }
    }
    Ok(())
}

async fn dismantle(ctx: &mut Ctx<'_>) -> Result<()> {
    for _ in 0..3 {
        let items = ctx
            .st
            .isl_at(&["gear", "items"])
            .as_array()
            .cloned()
            .unwrap_or_default();
        let mut worn: Vec<i64> = Vec::new();
        for d in ctx.st.dragons.iter() {
            if let Some(slots) = d.gear.as_object() {
                for v in slots.values() {
                    if let Some(id) = v.as_i64() {
                        worn.push(id);
                    }
                }
            }
        }
        let mut pick: Option<(i64, i64, String)> = None;
        for item in items.iter() {
            let id = item.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
            if id == 0 || worn.contains(&id) {
                continue;
            }
            let lvl = item.get("lvl").and_then(|v| v.as_i64()).unwrap_or(0);
            if lvl != 0 {
                continue;
            }
            let rar = item.get("rar").and_then(|v| v.as_i64()).unwrap_or(0);
            let kind = item
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("gear")
                .to_string();
            if pick.as_ref().map(|p| rar < p.1).unwrap_or(true) {
                pick = Some((id, rar, kind));
            }
        }
        let (id, rar, kind) = match pick {
            Some(v) => v,
            None => return Ok(()),
        };
        let res = ctx
            .act("/island/gear/dismantle", json!({ "ids": [id] }))
            .await?;
        if res.ok {
            let got = res.data.get("n").and_then(|v| v.as_f64()).unwrap_or(1.0);
            logger::ok(
                "gear",
                &format!("rarity {} {} recycled, {} back", rar, kind, logger::num(got)),
            );
        } else {
            logger::skip("gear", &format!("dismantle {}: {}", kind, res.reason()));
            return Ok(());
        }
    }
    Ok(())
}

fn wears(d: &crate::core::state::Dragon, kind: &str) -> bool {
    match d.gear.as_object() {
        Some(m) => m.iter().any(|(k, v)| k.contains(kind) && !v.is_null()),
        None => false,
    }
}
