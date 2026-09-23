use anyhow::Result;
use serde_json::{json, Value};

use crate::features::Ctx;
use crate::core::logger;

pub async fn run(ctx: &mut Ctx<'_>) -> Result<()> {
    if !ctx.cfg.referrals.enabled {
        return Ok(());
    }
    let res = ctx.get("/referrals").await?;
    if !res.ok {
        if !crate::features::is_hold(&res.reason()) {
            logger::skip("ref", &format!("state refused: {}", res.reason()));
        }
        return Ok(());
    }
    ctx.st.referrals = res.data.clone();

    let invited = res.arr("invited");
    let earned = res.get("earned").cloned().unwrap_or(Value::Null);
    let l1 = earned.get("l1").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let l2 = earned.get("l2").and_then(|v| v.as_f64()).unwrap_or(0.0);

    let mut pending: Vec<Value> = Vec::new();
    for f in &invited {
        let id = f
            .get("id")
            .or_else(|| f.get("friendId"))
            .or_else(|| f.get("userId"))
            .cloned();
        let amount = ["claimable", "pending", "unclaimed", "reward", "amount"]
            .iter()
            .filter_map(|k| f.get(*k).and_then(|v| v.as_f64()))
            .fold(0.0f64, f64::max);
        if let Some(id) = id {
            if amount > 0.0 {
                pending.push(id);
            }
        }
    }

    if let Some(cfg) = res.get("cfg") {
        logger::skip(
            "ref",
            &format!(
                "{} invited, l1 {} l2 {}, l1 share {} percent, cap {}",
                invited.len(),
                logger::num(l1),
                logger::num(l2),
                cfg.get("l1Pct").and_then(|v| v.as_f64()).unwrap_or(0.0) * 100.0,
                cfg.get("l1Cap").and_then(|v| v.as_f64()).unwrap_or(0.0)
            ),
        );
    }

    for id in pending {
        let r = ctx.act("/referrals/claim", json!({ "friendId": id })).await?;
        if r.ok {
            logger::ok("ref", &format!("collected from friend {}", id));
        } else {
            logger::skip("ref", &format!("friend {}: {}", id, r.reason()));
        }
    }
    Ok(())
}
