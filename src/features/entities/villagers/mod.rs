use anyhow::Result;
use serde_json::json;

use crate::features::Ctx;
use crate::core::logger;

pub async fn run(ctx: &mut Ctx<'_>) -> Result<()> {
    if !ctx.cfg.villagers.enabled || !ctx.cfg.villagers.frag_upgrade {
        return Ok(());
    }
    let mut done = 0;
    for _ in 0..20 {
        let mut pick: Option<(i64, String)> = None;
        for h in ctx.st.habitants.iter() {
            if h.star_frags >= 50 {
                continue;
            }
            let need = frag_need(ctx, h.star_frags);
            let fuel = ctx
                .st
                .isl_at(&["ifrags"])
                .get(format!("{}|{}|{}", h.role, h.rarity, h.name_idx))
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            if fuel >= need {
                pick = Some((h.id, h.name.clone()));
                break;
            }
        }
        let (id, name) = match pick {
            Some(v) => v,
            None => break,
        };
        let res = ctx
            .act("/island/habitant/frag", json!({ "habId": id }))
            .await?;
        if res.ok {
            done += 1;
            logger::ok("villager", &format!("star fragment for {}", name));
        } else {
            logger::skip("villager", &format!("fragment for {}: {}", name, res.reason()));
            break;
        }
    }
    if done == 0 {
        let map = ctx.st.isl_at(&["ifrags"]);
        if let Some(m) = map.as_object() {
            let total: i64 = m.values().filter_map(|v| v.as_i64()).sum();
            if total > 0 {
                logger::skip("villager", &format!("{} islander fragments held, none ready", total));
            }
        }
    }
    Ok(())
}

fn frag_need(ctx: &Ctx<'_>, star_frags: i64) -> i64 {
    let ladder = ctx.st.isl_at(&["frag", "ladder"]);
    let arr = match ladder.as_array() {
        Some(a) if !a.is_empty() => a.clone(),
        _ => return 1,
    };
    let idx = ((star_frags.max(0) / 5) as usize).min(arr.len() - 1);
    arr[idx].as_f64().unwrap_or(1.0) as i64
}
