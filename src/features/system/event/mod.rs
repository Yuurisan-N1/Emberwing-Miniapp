use anyhow::Result;
use serde_json::{json, Value};

use crate::features::Ctx;
use crate::core::logger;

pub async fn run(ctx: &mut Ctx<'_>) -> Result<()> {
    if ctx.cfg.event.enabled {
        festival(ctx).await?;
    }
    Ok(())
}

async fn festival(ctx: &mut Ctx<'_>) -> Result<()> {
    let ev = ctx.st.ev.clone();
    if ev.is_null() {
        return Ok(());
    }
    let active = ev.get("active").and_then(|v| v.as_bool()).unwrap_or(false);
    let steps = ev.get("steps").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    if steps.is_empty() {
        return Ok(());
    }
    let has_pass = ev.get("pass").and_then(|v| v.as_f64()).unwrap_or(0.0) > 0.0;

    if !active {
        logger::skip("event", "festival not running");
        return Ok(());
    }

    let mut claimed = 0;
    for step in steps.iter() {
        let n = step.get("step").and_then(|v| v.as_i64()).unwrap_or(0);
        let reached = step.get("reached").and_then(|v| v.as_bool()).unwrap_or(false);
        if !reached || n == 0 {
            continue;
        }
        if ctx.cfg.event.claim_free {
            let free_claimed = step
                .get("freeClaimed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !free_claimed {
                let res = ctx
                    .act("/event/claim", json!({ "step": n, "track": "free" }))
                    .await?;
                if res.ok {
                    claimed += 1;
                    logger::ok("event", &format!("step {} free reward", n));
                } else {
                    logger::skip("event", &format!("step {} free: {}", n, res.reason()));
                }
            }
        }
        if has_pass {
            let pass_claimed = step
                .get("passClaimed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !pass_claimed {
                let res = ctx
                    .act("/event/claim", json!({ "step": n, "track": "pass" }))
                    .await?;
                if res.ok {
                    claimed += 1;
                    logger::ok("event", &format!("step {} pass reward", n));
                } else {
                    logger::skip("event", &format!("step {} pass: {}", n, res.reason()));
                }
            }
        }
    }

    if claimed == 0 {
        let pts = ev.get("pts").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let next = steps
            .iter()
            .find(|s| !s.get("reached").and_then(|v| v.as_bool()).unwrap_or(false))
            .and_then(|s| s.get("pts").and_then(|v| v.as_f64()));
        if let Some(need) = next {
            logger::skip(
                "event",
                &format!("{} festival points, next step at {}", logger::num(pts), logger::num(need)),
            );
        }
    }

    let skins = ev.get("skins").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let worn = ev.get("hallSkin").cloned().unwrap_or(Value::Null);
    if worn.is_null() {
        if let Some(skin) = skins.first().and_then(|v| v.as_str()) {
            let res = ctx.act("/event/skin", json!({ "skin": skin })).await?;
            crate::features::report("event", &res, &format!("hall skin {} worn", skin));
        }
    }
    Ok(())
}
