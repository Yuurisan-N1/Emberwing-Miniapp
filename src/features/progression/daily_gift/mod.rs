use anyhow::Result;
use serde_json::json;

use crate::features::Ctx;
use crate::core::logger;
use crate::core::state::parse_ts;

pub async fn run(ctx: &mut Ctx<'_>) -> Result<()> {
    if !ctx.cfg.daily_gift.enabled {
        return Ok(());
    }
    let res = ctx.get("/dg/state").await?;
    if !res.ok {
        if !crate::features::is_hold(&res.reason()) {
            logger::skip("gift", &format!("state refused: {}", res.reason()));
        }
        return Ok(());
    }
    ctx.st.dg_state = res.data.clone();

    let can_claim = res.bool_of("canClaim");
    let locked = res.bool_of("locked");
    let day = res.num_of("claimDay") as i64;

    if !can_claim {
        let next = parse_ts(res.get("nextResetAt").unwrap_or(&serde_json::Value::Null));
        let left = if next > 0 {
            logger::clock((next - ctx.now()) / 1000)
        } else {
            "--:--:--".to_string()
        };
        if locked {
            logger::skip("gift", &format!("still locked, next window in {}", left));
        } else {
            logger::skip("gift", &format!("already taken today, next in {}", left));
        }
        return Ok(());
    }

    let claim = ctx.act("/dg/claim", json!({})).await?;
    if claim.ok {
        let reward = summarize(claim.get("rw").or_else(|| claim.get("got")));
        logger::ok(
            "gift",
            &format!("day {} claimed, {}", day.max(1), reward),
        );
    } else {
        logger::skip("gift", &format!("claim refused: {}", claim.reason()));
    }
    Ok(())
}

fn summarize(v: Option<&serde_json::Value>) -> String {
    match v {
        Some(serde_json::Value::Object(m)) => m
            .iter()
            .map(|(k, val)| format!("{} {}", k, logger::num(val.as_f64().unwrap_or(0.0))))
            .collect::<Vec<String>>()
            .join(" + "),
        Some(other) => logger::clean_text(&other.to_string()),
        None => "reward taken".to_string(),
    }
}
