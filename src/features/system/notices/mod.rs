use anyhow::Result;
use serde_json::json;

use crate::features::Ctx;

pub async fn run(ctx: &mut Ctx<'_>) -> Result<()> {
    if ctx.st.notices.is_empty() {
        return Ok(());
    }
    let ids: Vec<serde_json::Value> = ctx
        .st
        .notices
        .iter()
        .filter_map(|n| n.get("id").cloned())
        .collect();
    if ids.is_empty() {
        return Ok(());
    }
    let res = ctx.act("/notices/ack", json!({ "ids": ids })).await?;
    crate::features::report("notices", &res, &format!("{} acknowledged", ids.len()));
    Ok(())
}
