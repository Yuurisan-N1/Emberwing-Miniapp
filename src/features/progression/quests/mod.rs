use anyhow::Result;
use serde_json::{json, Value};

use crate::features::Ctx;
use crate::core::logger;

pub async fn run(ctx: &mut Ctx<'_>) -> Result<()> {
    if !ctx.cfg.quests.enabled {
        return Ok(());
    }
    let quest = ctx.st.quest.clone();
    if quest.is_null() {
        return Ok(());
    }

    let mut ids: Vec<String> = Vec::new();
    for key in ["main", "side", "chains"] {
        if let Some(list) = quest.get(key).and_then(|v| v.as_array()) {
            for q in list {
                collect_id(q, &mut ids);
            }
            if let Some(chains) = quest.get(key).and_then(|v| v.as_array()) {
                for chain in chains {
                    if let Some(steps) = chain.get("steps").and_then(|v| v.as_array()) {
                        for s in steps {
                            collect_id(s, &mut ids);
                        }
                    }
                }
            }
        }
    }

    if ids.is_empty() {
        let next = quest
            .get("main")
            .and_then(|v| v.as_array())
            .and_then(|a| a.iter().find(|q| !q.get("done").and_then(|v| v.as_bool()).unwrap_or(false)))
            .map(|q| {
                (
                    q.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    q.get("need").and_then(|v| v.as_f64()).unwrap_or(0.0),
                    q.get("cur").and_then(|v| v.as_f64()).unwrap_or(0.0),
                )
            });
        if let Some((id, need, cur)) = next {
            logger::skip(
                "quest",
                &format!("next: {} {}/{}", id, logger::num(cur), logger::num(need)),
            );
        }
        return Ok(());
    }

    for id in ids {
        let res = ctx
            .act("/island/quest/claim", json!({ "id": id }))
            .await?;
        if res.ok {
            logger::ok("quest", &format!("{} claimed", id));
        } else {
            logger::skip("quest", &format!("{}: {}", id, res.reason()));
        }
    }
    Ok(())
}

fn collect_id(q: &Value, out: &mut Vec<String>) {
    let done = q.get("done").and_then(|v| v.as_bool()).unwrap_or(false);
    let claimed = q.get("claimed").and_then(|v| v.as_bool()).unwrap_or(false);
    if !done || claimed {
        return;
    }
    if let Some(id) = q.get("id").and_then(|v| v.as_str()) {
        if !id.is_empty() && !out.iter().any(|x| x == id) {
            out.push(id.to_string());
        }
    }
}
