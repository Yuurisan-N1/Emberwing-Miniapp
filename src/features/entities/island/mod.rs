use anyhow::Result;
use serde_json::{json, Value};

use crate::features::Ctx;
use crate::core::logger;

pub async fn run(ctx: &mut Ctx<'_>) -> Result<()> {
    if !ctx.cfg.island.enabled {
        return Ok(());
    }
    if ctx.cfg.island.collect {
        collect(ctx).await?;
    }
    if ctx.cfg.island.complete_jobs {
        complete_jobs(ctx).await?;
    }
    if ctx.cfg.island.assign_auto {
        assign_auto(ctx).await?;
    }
    if ctx.cfg.island.build {
        build(ctx).await?;
    }
    if ctx.cfg.island.upgrade {
        upgrade(ctx).await?;
    }
    if ctx.cfg.island.den_collect {
        quick_speedup(ctx).await?;
        den(ctx).await?;
    }
    Ok(())
}

async fn collect(ctx: &mut Ctx<'_>) -> Result<()> {
    let mut total = 0.0;
    let mut n = 0;
    for _ in 0..20 {
        let next = ctx
            .st
            .buildings
            .iter()
            .find(|b| b.kind != "hall" && !b.working(ctx.now()) && b.stock >= 1.0)
            .map(|b| (b.id, b.kind.clone(), b.stock));
        let (id, kind, stock) = match next {
            Some(v) => v,
            None => break,
        };
        let res = ctx.act("/island/collect", json!({ "buildingId": id })).await?;
        if res.ok {
            n += 1;
            total += stock;
            logger::ok("island", &format!("{} yielded {}", kind, logger::num(stock)));
        } else {
            if n == 0 {
                logger::skip("island", &format!("collect {}: {}", kind, res.reason()));
            }
            break;
        }
    }
    if n > 0 {
        logger::ok("island", &format!("{} buildings collected, {}", n, logger::num(total)));
    }
    Ok(())
}

async fn complete_jobs(ctx: &mut Ctx<'_>) -> Result<()> {
    for _ in 0..10 {
        let now = ctx.now();
        let next = ctx
            .st
            .buildings
            .iter()
            .find(|b| {
                b.job_type.is_some()
                    && crate::core::state::parse_ts(&b.job_until) > 0
                    && crate::core::state::parse_ts(&b.job_until) <= now
            })
            .map(|b| (b.id, b.kind.clone(), b.lvl));
        let (id, kind, lvl) = match next {
            Some(v) => v,
            None => break,
        };
        let res = ctx.act("/island/complete", json!({ "buildingId": id })).await?;
        if res.ok {
            logger::ok("island", &format!("{} lvl {} finished", kind, lvl));
        } else {
            logger::skip("island", &format!("finish {}: {}", kind, res.reason()));
            break;
        }
    }
    Ok(())
}

async fn assign_auto(ctx: &mut Ctx<'_>) -> Result<()> {
    let idle = ctx
        .st
        .habitants
        .iter()
        .filter(|h| h.building_id.is_null())
        .count();
    if idle == 0 || ctx.st.habitants.is_empty() {
        return Ok(());
    }
    let res = ctx
        .act("/island/assign/auto", json!({ "buildingId": Value::Null }))
        .await?;
    if res.ok {
        logger::ok("island", &format!("{} idle villagers assigned", idle));
    } else {
        logger::skip("island", &format!("assign: {}", res.reason()));
    }
    Ok(())
}

fn guide_build(st: &crate::core::state::Snapshot) -> Option<String> {
    let list = st.quest.get("main")?.as_array()?;
    for q in list {
        let guided = q.get("guided").and_then(|v| v.as_bool()).unwrap_or(false)
            || q.get("guided").and_then(|v| v.as_f64()).unwrap_or(0.0) != 0.0;
        if !guided
            || q.get("claimed").and_then(|v| v.as_bool()).unwrap_or(false)
            || q.get("done").and_then(|v| v.as_bool()).unwrap_or(false)
        {
            continue;
        }
        let go = q.get("go").and_then(|v| v.as_str()).unwrap_or("");
        let spot = match go.strip_prefix("bld:") {
            Some(s) => s,
            None => return None,
        };
        return Some(match spot {
            "residency" => "res".to_string(),
            "hall" => "hall".to_string(),
            other => other.to_string(),
        });
    }
    None
}

fn guide_step(st: &crate::core::state::Snapshot) -> Option<String> {
    let list = st.quest.get("main")?.as_array()?;
    for q in list {
        let guided = q.get("guided").and_then(|v| v.as_bool()).unwrap_or(false)
            || q.get("guided").and_then(|v| v.as_f64()).unwrap_or(0.0) != 0.0;
        if !guided
            || q.get("claimed").and_then(|v| v.as_bool()).unwrap_or(false)
            || q.get("done").and_then(|v| v.as_bool()).unwrap_or(false)
        {
            continue;
        }
        return Some(
            q.get("go")
                .and_then(|v| v.as_str())
                .unwrap_or("the next step")
                .to_string(),
        );
    }
    None
}

async fn build(ctx: &mut Ctx<'_>) -> Result<()> {
    let types = ctx.st.isl_at(&["bld", "types"]);
    let table = match types.as_object() {
        Some(t) => t.clone(),
        None => return Ok(()),
    };
    let want = guide_build(ctx.st);
    if want.is_none() {
        if let Some(step) = guide_step(ctx.st) {
            logger::skip(
                "island",
                &format!("elder wants {} first, the build trade waits", step),
            );
            return Ok(());
        }
    }
    let mut wanted: Vec<(String, i64, f64, f64, f64)> = Vec::new();
    for (kind, spec) in table.iter() {
        let up = spec.get("up").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let inst = ctx.st.buildings.iter().find(|b| &b.kind == kind);
        match inst {
            Some(b) if b.broken == 0 => continue,
            _ => {}
        }
        let cost = up.first().cloned().unwrap_or(Value::Null);
        let logs = cost.get("logs").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let gold = cost.get("gold").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let id = inst.map(|b| b.id).unwrap_or(0);
        let rank = if want.as_deref() == Some(kind.as_str()) {
            -2.0
        } else if kind == "hall" {
            -1.0
        } else {
            gold
        };
        wanted.push((kind.clone(), id, gold, logs, rank));
    }
    wanted.sort_by(|a, b| a.4.partial_cmp(&b.4).unwrap_or(std::cmp::Ordering::Equal));

    let mut waiting = 0u32;
    let mut cheapest: Option<(String, f64, f64)> = None;
    for (kind, id, gold, logs, _) in wanted {
        if !builder_free(ctx) {
            logger::skip("island", "every builder is busy");
            return Ok(());
        }
        if ctx.st.player.gold < gold || ctx.st.player.logs < logs {
            waiting += 1;
            if cheapest.is_none() {
                cheapest = Some((kind.clone(), gold, logs));
            }
            continue;
        }
        let res = ctx
            .act("/island/build", json!({ "type": kind, "buildingId": id }))
            .await?;
        if res.ok {
            waiting = 0;
            logger::ok("island", &format!("{} building started", kind));
        } else if guided(&res) {
            logger::skip(
                "island",
                &format!("{} waits on the elder line, the next trade is tried", kind),
            );
            continue;
        } else {
            logger::skip("island", &format!("build {}: {}", kind, res.reason()));
            return Ok(());
        }
    }
    if waiting > 0 {
        if let Some((kind, gold, logs)) = cheapest {
            logger::skip(
                "island",
                &format!(
                    "{} buildings waiting, next is {} at {} gold {} logs, held {} / {}",
                    waiting,
                    kind,
                    logger::num(gold),
                    logger::num(logs),
                    logger::num(ctx.st.player.gold),
                    logger::num(ctx.st.player.logs)
                ),
            );
        }
    }
    Ok(())
}

async fn upgrade(ctx: &mut Ctx<'_>) -> Result<()> {
    let types = ctx.st.isl_at(&["bld", "types"]);
    let max_lvl = ctx.st.isl_num(&["bld", "maxLvl"]).unwrap_or(30.0) as i64;
    let table = match types.as_object() {
        Some(t) => t.clone(),
        None => return Ok(()),
    };

    let mut wanted: Vec<(String, i64, i64, f64, f64, bool)> = Vec::new();
    for b in ctx.st.buildings.iter() {
        if b.broken != 0 || b.lvl >= max_lvl {
            continue;
        }
        let spec = match table.get(&b.kind) {
            Some(s) => s,
            None => continue,
        };
        let up = spec.get("up").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let cost = match up.get(b.lvl.max(0) as usize) {
            Some(c) => c.clone(),
            None => continue,
        };
        let gold = cost.get("gold").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let logs = cost.get("logs").and_then(|v| v.as_f64()).unwrap_or(0.0);
        wanted.push((
            b.kind.clone(),
            b.id,
            b.lvl,
            gold,
            logs,
            b.kind == "hall",
        ));
    }
    wanted.sort_by(|a, b| {
        b.5.cmp(&a.5)
            .then(a.3.partial_cmp(&b.3).unwrap_or(std::cmp::Ordering::Equal))
    });

    let mut waiting = 0u32;
    let mut cheapest: Option<(String, f64, f64)> = None;
    for (kind, id, lvl, gold, logs, _) in wanted {
        if !builder_free(ctx) {
            return Ok(());
        }
        if ctx.st.player.gold < gold || ctx.st.player.logs < logs {
            waiting += 1;
            if cheapest.is_none() {
                cheapest = Some((kind.clone(), gold, logs));
            }
            continue;
        }
        let res = ctx
            .act("/island/upgrade", json!({ "buildingId": id }))
            .await?;
        if res.ok {
            waiting = 0;
            logger::ok("island", &format!("{} lvl {} to {} queued", kind, lvl, lvl + 1));
        } else if guided(&res) {
            continue;
        } else {
            logger::skip("island", &format!("upgrade {}: {}", kind, res.reason()));
            return Ok(());
        }
    }
    if waiting > 0 {
        if let Some((kind, gold, logs)) = cheapest {
            logger::skip(
                "island",
                &format!(
                    "{} upgrades waiting, next is {} at {} gold {} logs",
                    waiting,
                    kind,
                    logger::num(gold),
                    logger::num(logs)
                ),
            );
        }
    }
    Ok(())
}

fn builder_free(ctx: &Ctx<'_>) -> bool {
    let now = ctx.now();
    let busy = ctx
        .st
        .buildings
        .iter()
        .filter(|b| crate::core::state::parse_ts(&b.job_until) > now)
        .count() as i64;
    busy < ctx.st.player.builders.max(1)
}

async fn den(ctx: &mut Ctx<'_>) -> Result<()> {
    let den = ctx.st.isl_at(&["den"]);
    let ready = den.get("ready").and_then(|v| v.as_f64()).unwrap_or(0.0);
    if ready >= 1.0 {
        let res = ctx.act("/island/den/collect", json!({})).await?;
        crate::features::report("island", &res, &format!("den yielded {} eggs", logger::num(ready)));
    }

    if ctx.st.buildings.iter().any(|b| b.working(ctx.now())) {
        return Ok(());
    }

    let owned = ctx.st.isl_at(&["boosts"]);
    let free: Vec<(&str, &str, f64)> = [("60m", "m60", 0.0), ("5m", "m5", 0.0), ("1m", "m1", 0.0)]
        .iter()
        .map(|(kind, key, _)| {
            (
                *kind,
                *key,
                owned.get(*key).and_then(|v| v.as_f64()).unwrap_or(0.0),
            )
        })
        .filter(|(_, _, n)| *n >= 1.0)
        .collect();
    if let Some((kind, key, n)) = free.first() {
        let res = ctx.act("/island/den/boost", json!({ "kind": kind })).await?;
        crate::features::report(
            "island",
            &res,
            &format!("den boosted {}, {} {} left", kind, logger::num(*n - 1.0), key),
        );
    }
    Ok(())
}

async fn quick_speedup(ctx: &mut Ctx<'_>) -> Result<()> {
    let now = ctx.now();
    let job = ctx
        .st
        .buildings
        .iter()
        .find(|b| crate::core::state::parse_ts(&b.job_until) > now)
        .map(|b| (b.id, b.kind.clone(), (crate::core::state::parse_ts(&b.job_until) - now) / 60_000));
    let (id, kind, remain_min) = match job {
        Some(v) => v,
        None => return Ok(()),
    };
    if remain_min < 1 {
        return Ok(());
    }
    let owned = ctx.st.isl_at(&["boosts"]);
    let mut rem = remain_min;
    let mut plan: Vec<(String, f64)> = Vec::new();
    for (kind_key, span) in [("m60", 60i64), ("m5", 5), ("m1", 1)] {
        let have = owned.get(kind_key).and_then(|v| v.as_f64()).unwrap_or(0.0);
        let take = have.min((rem / span) as f64);
        if take >= 1.0 {
            plan.push((kind_key.to_string(), take));
            rem -= (take as i64) * span;
        }
    }
    if rem > 0 {
        let m5 = owned.get("m5").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let m60 = owned.get("m60").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if m5 > plan.iter().filter(|(k, _)| k == "m5").map(|(_, n)| *n).sum::<f64>()
            || m60 > plan.iter().filter(|(k, _)| k == "m60").map(|(_, n)| *n).sum::<f64>()
        {
            rem = 0;
        }
    }
    if rem > 0 || plan.is_empty() {
        logger::skip(
            "island",
            &format!(
                "{} has {} left, boosters do not cover it",
                kind,
                logger::clock(remain_min * 60)
            ),
        );
        return Ok(());
    }
    let res = ctx.act("/island/speedup", json!({ "buildingId": id, "kind": "quick" })).await?;
    crate::features::report(
        "island",
        &res,
        &format!(
            "{} finished for free, {}",
            kind,
            plan.iter()
                .map(|(k, n)| format!("{}x{}", logger::num(*n), k))
                .collect::<Vec<String>>()
                .join(" + ")
        ),
    );
    Ok(())
}

fn guided(res: &crate::core::http::Res) -> bool {
    let r = res.reason().to_lowercase();
    r.contains("elder") || r.contains("bad kind")
}
