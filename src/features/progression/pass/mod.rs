use anyhow::Result;
use serde_json::json;

use crate::features::Ctx;
use crate::core::logger;
use crate::core::state::parse_ts;

pub async fn run(ctx: &mut Ctx<'_>) -> Result<()> {
    if !ctx.cfg.pass.enabled {
        return Ok(());
    }
    let pass = ctx.st.pass.clone();
    let until = parse_ts(pass.get("until").unwrap_or(&serde_json::Value::Null));
    let tier = pass.get("tier").and_then(|v| v.as_i64()).unwrap_or(0);
    if until <= ctx.now() {
        return Ok(());
    }

    logger::skip(
        "pass",
        &format!("tier {} active, perks switched on", tier),
    );

    let caps = pass.get("caps").cloned().unwrap_or(serde_json::Value::Null);
    let auto = caps
        .get("autoBoosts")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let bulk = caps
        .get("bulkEggs")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if auto {
        let r = ctx.act("/pass/automerge", json!({ "tier": tier })).await?;
        crate::features::report("pass", &r, "auto merge on");
    }
    if bulk {
        let r = ctx.act("/pass/incubateAll", json!({})).await?;
        crate::features::report("pass", &r, "incubate all");
        let r = ctx.act("/pass/openAll", json!({})).await?;
        crate::features::report("pass", &r, "open all");
    }

    let gold_day = pass
        .get("goldDay")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !gold_day {
        let r = ctx.act("/pass/gold", json!({})).await?;
        crate::features::report("pass", &r, "daily gold taken");
    }
    Ok(())
}
