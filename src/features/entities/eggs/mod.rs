use anyhow::Result;
use serde_json::{json, Value};

use crate::features::Ctx;
use crate::core::logger;

pub async fn run(ctx: &mut Ctx<'_>) -> Result<()> {
    if !ctx.cfg.eggs.enabled {
        return Ok(());
    }
    if ctx.cfg.eggs.merge {
        merge(ctx).await?;
    }
    if ctx.cfg.eggs.incubate {
        incubate(ctx).await?;
    }
    if ctx.cfg.eggs.open_free {
        open_ready(ctx).await?;
    }
    Ok(())
}

async fn merge(ctx: &mut Ctx<'_>) -> Result<()> {
    let cap = ctx.st.egg_max_tier();
    for _ in 0..40 {
        let eggs = ctx.st.eggs.clone();
        let mut groups: Vec<(i64, String, Vec<i64>)> = Vec::new();
        for e in &eggs {
            if e.sealed != 0 || e.tier >= cap {
                continue;
            }
            match groups
                .iter_mut()
                .find(|(t, el, _)| *t == e.tier && *el == e.element)
            {
                Some(g) => g.2.push(e.id),
                None => groups.push((e.tier, e.element.clone(), vec![e.id])),
            }
        }
        let pair = groups.iter().find(|(_, _, ids)| ids.len() >= 2);
        let (tier, _, ids) = match pair {
            Some(p) => p.clone(),
            None => break,
        };
        let (src, dst) = (ids[0], ids[1]);
        let res = ctx.act("/eggs/merge", json!({ "srcId": src, "dstId": dst })).await?;
        if res.ok {
            logger::ok("egg", &format!("tier {} + tier {} merged", tier, tier));
        } else {
            logger::skip("egg", &format!("merge tier {}: {}", tier, res.reason()));
            break;
        }
    }
    Ok(())
}

async fn incubate(ctx: &mut Ctx<'_>) -> Result<()> {
    let now = ctx.now();
    let mut placed = 0;
    for _ in 0..30 {
        let next = ctx
            .st
            .eggs
            .iter()
            .find(|e| e.sealed == 0 && e.idle() && e.zone == "board")
            .map(|e| (e.id, e.tier));
        let (id, tier) = match next {
            Some(v) => v,
            None => break,
        };
        let res = ctx.act(&format!("/eggs/{}/incubate", id), json!({})).await?;
        if res.ok {
            placed += 1;
            logger::ok("egg", &format!("tier {} in a nest", tier));
        } else {
            if placed == 0 {
                logger::skip("egg", &format!("incubate: {}", res.reason()));
            }
            break;
        }
    }
    let ready = ctx
        .st
        .eggs
        .iter()
        .filter(|e| e.ready(now))
        .count();
    if placed == 0 && ready == 0 {
        let mins = ctx
            .st
            .eggs
            .iter()
            .filter_map(|e| {
                let end = crate::core::state::parse_ts(&e.incubate_until);
                if end > now {
                    Some((end - now) / 1000)
                } else {
                    None
                }
            })
            .min();
        if let Some(s) = mins {
            logger::skip("egg", &format!("nests busy, next hatch in {}", logger::clock(s)));
        }
    }
    Ok(())
}

async fn open_ready(ctx: &mut Ctx<'_>) -> Result<()> {
    let mut opened = 0;
    for _ in 0..30 {
        let now = ctx.now();
        let next = ctx
            .st
            .eggs
            .iter()
            .find(|e| e.sealed == 0 && e.ready(now))
            .map(|e| (e.id, e.tier, e.element.clone()));
        let (id, tier, el) = match next {
            Some(v) => v,
            None => break,
        };
        let res = ctx
            .act(&format!("/eggs/{}/open", id), json!({ "instant": false }))
            .await?;
        if res.ok {
            opened += 1;
            let got = hatch_summary(&res);
            logger::ok(
                "egg",
                &format!("tier {} {} hatched, {}", tier, el, got),
            );
        } else {
            logger::skip("egg", &format!("open {}: {}", id, res.reason()));
            break;
        }
    }
    if opened == 0 {
        let now = ctx.now();
        let incubating = ctx
            .st
            .eggs
            .iter()
            .filter(|e| e.incubating(now))
            .count();
        if incubating > 0 {
            logger::skip("egg", &format!("{} eggs still hatching", incubating));
        }
    }
    Ok(())
}

fn hatch_summary(res: &crate::core::http::Res) -> String {
    for key in ["dragon", "prize", "got", "rw"] {
        if let Some(v) = res.get(key) {
            if !v.is_null() {
                if let Some(name) = v.get("name").and_then(|x| x.as_str()) {
                    return format!("{} (rarity {})", logger::clean_text(name), v.get("rarity").and_then(|r| r.as_i64()).unwrap_or(0));
                }
                if let Some(Value::Object(_)) = Some(v) {
                    let parts: Vec<String> = v
                        .as_object()
                        .unwrap()
                        .iter()
                        .map(|(k, val)| format!("{} {}", k, logger::num(val.as_f64().unwrap_or(0.0))))
                        .collect();
                    return parts.join(" + ");
                }
            }
        }
    }
    "hatched".to_string()
}
