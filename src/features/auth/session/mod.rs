use anyhow::Result;
use serde_json::json;

use crate::features::Ctx;
use crate::core::logger;

pub async fn run(ctx: &mut Ctx<'_>) -> Result<bool> {
    let res = ctx.api.auth(ctx.init_data).await?;
    if !res.ok {
        logger::fail("auth", &format!("refused: {}", res.reason()));
        return Ok(false);
    }
    if let Some(token) = res.get("token").and_then(|v| v.as_str()) {
        ctx.api.set_token(token);
    }
    if let Some(sn) = res.snapshot() {
        ctx.st.apply(sn);
    }
    let is_new = res.bool_of("isNew");
    let welcomed = res.bool_of("welcomed");
    logger::ok(
        "auth",
        &format!(
            "session ok, {}, gold {}, trophies {}, {}ms",
            ctx.st.label(),
            logger::num(ctx.st.player.gold),
            ctx.st.player.trophies,
            res.ms
        ),
    );

    if is_new {
        logger::skip("auth", "first login on this account");
    }
    if !welcomed && ctx.cfg.welcome_dm.enabled {
        let r = ctx.act("/me/welcome", json!({})).await?;
        crate::features::report("welcome", &r, "welcome DM sent to the bot chat");
    }
    Ok(true)
}
