use anyhow::Result;
use serde_json::json;

use crate::features::Ctx;
use crate::core::logger;

pub async fn run(ctx: &mut Ctx<'_>) -> Result<()> {
    if !ctx.cfg.achievements.enabled {
        return Ok(());
    }

    const ACTIONS: [&str; 3] = ["homescreen", "news", "community"];

    let mut claimed = 0;
    for _ in 0..25 {
        let defs = ctx
            .st
            .isl_at(&["ach", "defs"])
            .as_array()
            .cloned()
            .unwrap_or_default();
        if defs.is_empty() {
            return Ok(());
        }
        let mut pick: Option<String> = None;
        let mut action_waiting = 0;
        for def in defs.iter() {
            let key = def.get("k").and_then(|v| v.as_str()).unwrap_or("");
            if key.is_empty() {
                continue;
            }
            let goals = def.get("goals").and_then(|v| v.as_array()).cloned().unwrap_or_default();
            let tier = ctx
                .st
                .player
                .ach
                .get(key)
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            if tier as usize >= goals.len() {
                continue;
            }
            let goal = goals[tier as usize].as_f64().unwrap_or(f64::MAX);
            let val = ctx
                .st
                .isl_at(&["ach", "vals"])
                .get(key)
                .and_then(|v| v.as_f64())
                .or_else(|| ctx.st.player.counters.get(key).and_then(|v| v.as_f64()))
                .unwrap_or(0.0);
            if val >= goal {
                if ACTIONS.contains(&key) {
                    action_waiting += 1;
                    continue;
                }
                pick = Some(key.to_string());
                break;
            }
        }

        let key = match pick {
            Some(k) => k,
            None => {
                if action_waiting > 0 {
                    logger::skip(
                        "achv",
                        &format!(
                            "{} deeds need a Telegram action, join / subscribe / home screen",
                            action_waiting
                        ),
                    );
                }
                break;
            }
        };
        let res = ctx.act("/claims/ach", json!({ "key": key })).await?;
        if res.ok {
            claimed += 1;
            logger::ok("achv", &format!("{} tier claimed", key));
        } else {
            logger::skip("achv", &format!("{}: {}", key, res.reason()));
            break;
        }
    }
    if claimed == 0 {
        let done = ctx
            .st
            .isl_at(&["ach", "defs"])
            .as_array()
            .map(|defs| {
                defs.iter()
                    .filter(|d| {
                        let key = d.get("k").and_then(|v| v.as_str()).unwrap_or("");
                        let tiers = d
                            .get("goals")
                            .and_then(|v| v.as_array())
                            .map(|a| a.len())
                            .unwrap_or(0) as i64;
                        ctx.st
                            .player
                            .ach
                            .get(key)
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0)
                            >= tiers
                    })
                    .count()
            })
            .unwrap_or(0);
        if done > 0 {
            logger::skip("achv", &format!("{} tracks finished, none ready", done));
        }
    }
    Ok(())
}
