use anyhow::Result;
use serde_json::json;

use crate::features::Ctx;
use crate::core::logger;

pub async fn run(ctx: &mut Ctx<'_>) -> Result<()> {
    if !ctx.cfg.dragons.enabled {
        return Ok(());
    }
    if ctx.cfg.dragons.level_up {
        level_up(ctx).await?;
    }
    if ctx.cfg.dragons.abilities {
        abilities(ctx).await?;
    }
    if ctx.cfg.dragons.rarity_up {
        rarity_up(ctx).await?;
    }
    Ok(())
}

async fn level_up(ctx: &mut Ctx<'_>) -> Result<()> {
    let mut done = 0;
    for _ in 0..30 {
        let meat = ctx.st.meat_now();
        let mut cands: Vec<(i64, f64, String, i64)> = ctx
            .st
            .dragons
            .iter()
            .filter(|d| d.next_meat() > 0.0 && d.next_meat() <= meat && !d.away())
            .map(|d| (d.id, d.pow(), d.name.clone(), d.isl_level()))
            .collect();
        if cands.is_empty() {
            break;
        }
        cands.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let (id, _, name, lvl) = cands[0].clone();
        let need = ctx
            .st
            .dragons
            .iter()
            .find(|d| d.id == id)
            .map(|d| d.next_meat())
            .unwrap_or(0.0);
        let res = ctx
            .act("/island/dragon/levelup", json!({ "dragonId": id }))
            .await?;
        if res.ok {
            done += 1;
            logger::ok(
                "dragon",
                &format!("{} lvl {} to {}, meat {}", name, lvl, lvl + 1, logger::num(need)),
            );
        } else {
            logger::skip("dragon", &format!("level up {}: {}", name, res.reason()));
            break;
        }
    }
    if done == 0 {
        let meat = ctx.st.meat_now();
        let next = ctx
            .st
            .dragons
            .iter()
            .filter(|d| d.next_meat() > 0.0)
            .map(|d| d.next_meat())
            .fold(f64::MAX, f64::min);
        if next.is_finite() && next > meat {
            logger::skip(
                "dragon",
                &format!("meat {} short of {}", logger::num(meat), logger::num(next)),
            );
        }
    }
    Ok(())
}

async fn abilities(ctx: &mut Ctx<'_>) -> Result<()> {
    let mut done = 0;
    for _ in 0..20 {
        let mut pick: Option<(i64, String, String)> = None;
        for d in ctx.st.dragons.iter() {
            if d.away() {
                continue;
            }
            let lvl = d.isl_level();
            for a in d.isl_abils.iter() {
                let key = a.get("key").and_then(|v| v.as_str()).unwrap_or("");
                if key.is_empty() {
                    continue;
                }
                let unlocked = a.get("unlocked").and_then(|v| v.as_bool()).unwrap_or(false);
                let cost = a.get("nextCost").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let gate_lvl = a
                    .get("unlock")
                    .and_then(|u| u.get("lvl"))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let gate_stars = a
                    .get("unlock")
                    .and_then(|u| u.get("stars"))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let affordable = d.star_frags as f64 >= cost;
                let allowed = if unlocked {
                    affordable
                } else {
                    lvl as f64 >= gate_lvl && d.star_frags as f64 >= gate_stars
                };
                if allowed {
                    pick = Some((d.id, key.to_string(), d.name.clone()));
                    break;
                }
            }
            if pick.is_some() {
                break;
            }
        }
        let (id, key, name) = match pick {
            Some(v) => v,
            None => break,
        };
        let res = ctx
            .act(
                "/island/dragon/ability",
                json!({ "dragonId": id, "key": key }),
            )
            .await?;
        if res.ok {
            done += 1;
            logger::ok("dragon", &format!("{} ability {}", name, key));
        } else {
            logger::skip("dragon", &format!("ability {} on {}: {}", key, name, res.reason()));
            break;
        }
    }
    if done == 0 {
        let frags: i64 = ctx.st.dragons.iter().map(|d| d.star_frags).sum();
        let need = ctx
            .st
            .isl_num(&["frag", "ladder"])
            .map(|_| 0.0)
            .unwrap_or(0.0);
        let _ = need;
        if frags > 0 {
            logger::skip("dragon", &format!("{} star fragments, gates not met", frags));
        }
    }
    Ok(())
}

async fn rarity_up(ctx: &mut Ctx<'_>) -> Result<()> {
    let now = ctx.now();
    let free_wizard = ctx.st.wizards.iter().find(|w| {
        w.dragon_id.is_null() && crate::core::state::parse_ts(&w.until) <= now
    });
    let wizard = match free_wizard {
        Some(w) => w.clone(),
        None => {
            if !ctx.st.wizards.is_empty() {
                logger::skip("dragon", "wizard busy");
            }
            return Ok(());
        }
    };

    let mut best: Option<(i64, String)> = None;
    for d in ctx.st.dragons.iter() {
        if d.away() || d.rarity >= 5 {
            continue;
        }
        let need = frag_need(ctx, d.star_frags);
        let have = fuel_of(ctx, &d.element, d.rarity) + jokers(ctx, d.rarity);
        if have >= need {
            best = Some((d.id, d.name.clone()));
            break;
        }
    }
    let (id, name) = match best {
        Some(v) => v,
        None => return Ok(()),
    };
    let res = ctx
        .act(
            "/island/rarup",
            json!({ "dragonId": id, "wizardId": wizard.id }),
        )
        .await?;
    if res.ok {
        logger::ok(
            "dragon",
            &format!("rarity up queued for {} with {}", name, wizard.name),
        );
    } else {
        logger::skip("dragon", &format!("rarity up {}: {}", name, res.reason()));
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

fn fuel_of(ctx: &Ctx<'_>, el: &str, rarity: i64) -> i64 {
    let frags = ctx.st.isl_at(&["frags"]);
    frags
        .get(format!("{}|{}", el, rarity))
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
}

fn jokers(ctx: &Ctx<'_>, rarity: i64) -> i64 {
    ctx.st
        .isl_at(&["jokers"])
        .as_array()
        .and_then(|a| a.get(rarity.max(0) as usize))
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
}

pub async fn convert_fragments(ctx: &mut Ctx<'_>) -> Result<()> {
    if !ctx.cfg.dragons.abilities {
        return Ok(());
    }
    let mut done = 0;
    for _ in 0..20 {
        let mut pick: Option<(i64, String)> = None;
        for d in ctx.st.dragons.iter() {
            if d.away() || d.star_frags >= 50 {
                continue;
            }
            let need = frag_need(ctx, d.star_frags);
            if fuel_of(ctx, &d.element, d.rarity) >= need {
                pick = Some((d.id, d.name.clone()));
                break;
            }
        }
        let (id, name) = match pick {
            Some(v) => v,
            None => break,
        };
        let res = ctx.act("/island/frag", json!({ "dragonId": id })).await?;
        if res.ok {
            done += 1;
            logger::ok("dragon", &format!("star fragment for {}", name));
        } else {
            logger::skip("dragon", &format!("fragment for {}: {}", name, res.reason()));
            break;
        }
    }
    if done == 0 {
        let frags = ctx.st.isl_at(&["frags"]);
        if let Some(map) = frags.as_object() {
            let total: i64 = map.values().filter_map(|v| v.as_i64()).sum();
            if total > 0 {
                logger::skip("dragon", &format!("{} kind fragments held, dragons capped", total));
            }
        }
    }
    Ok(())
}
