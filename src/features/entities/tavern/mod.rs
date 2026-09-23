use anyhow::Result;
use serde_json::{json, Value};

use crate::features::Ctx;
use crate::core::logger;

pub async fn run(ctx: &mut Ctx<'_>) -> Result<()> {
    if !ctx.cfg.tavern.enabled {
        for kind in ["i", "d"] {
            let tickets = tickets_of(ctx, kind);
            if tickets > 0.0 {
                logger::skip(
                    "tavern",
                    &format!(
                        "{} {} tickets held, rolls disabled in config.json",
                        logger::num(tickets),
                        wire(kind)
                    ),
                );
            }
        }
        return Ok(());
    }

    // Both tickets are spendable: an islander hire is a worker and the elder line
    // wants one assigned before it lets a single building go up, a dragon hire is a
    // new fighter. Roll the configured kind first, then the other while it still holds a ticket.
    let first = normalize(&ctx.cfg.tavern.kind);
    let order: [&str; 2] = if first == "i" { ["i", "d"] } else { ["d", "i"] };
    for kind in order {
        let mut left = tickets_of(ctx, kind);
        if left < 1.0 {
            log_pity(ctx, kind);
            continue;
        }
        let mut rolled = 0u32;
        while rolled < ctx.cfg.tavern.max_rolls_per_cycle && left >= 1.0 {
            let res = ctx
                .act("/island/tavern/roll", json!({ "kind": wire(kind), "n": 1 }))
                .await?;
            if !res.ok {
                logger::skip("tavern", &format!("roll: {}", res.reason()));
                break;
            }
            rolled += 1;
            logger::ok(
                "tavern",
                &format!("{} hire, {}", wire(kind), prize_of(&res)),
            );
            left = tickets_of(ctx, kind);
        }
        if rolled == 0 {
            log_pity(ctx, kind);
        }
    }
    Ok(())
}

fn normalize(kind: &str) -> String {
    if kind.eq_ignore_ascii_case("i") || kind.eq_ignore_ascii_case("worker") {
        "i".to_string()
    } else {
        "d".to_string()
    }
}

/// The snapshot keeps the tavern tickets under `tav.i` / `tav.d`, but the roll endpoint
/// only takes `worker` / `dragon` and answers `bad roll kind` to the short names.
fn wire(kind: &str) -> &'static str {
    if normalize(kind) == "i" {
        "worker"
    } else {
        "dragon"
    }
}

fn tickets_of(ctx: &Ctx<'_>, kind: &str) -> f64 {
    let k = normalize(kind);
    ctx.st
        .isl_at(&["tav", &k])
        .as_f64()
        .unwrap_or(0.0)
}

fn log_pity(ctx: &Ctx<'_>, kind: &str) {
    let key = if normalize(kind) == "i" { "pityI" } else { "pityD" };
    let pity = ctx.st.isl_at(&["tav", key]).as_f64().unwrap_or(0.0);
    let limit = ctx.st.isl_at(&["tav", "odds", "pity"]).as_f64().unwrap_or(0.0);
    if limit > 0.0 {
        logger::skip(
            "tavern",
            &format!("no tickets, pity {}/{}", logger::num(pity), logger::num(limit)),
        );
    }
}

fn prize_of(res: &crate::core::http::Res) -> String {
    let prize = res.data.get("prize").cloned().unwrap_or(Value::Null);
    if prize.is_null() {
        return "nothing".to_string();
    }
    let kind = prize.get("kind").and_then(|v| v.as_str()).unwrap_or("");
    let name = prize
        .get("name")
        .and_then(|v| v.as_str())
        .or_else(|| prize.get("el").and_then(|v| v.as_str()))
        .unwrap_or("");
    let n = prize.get("n").and_then(|v| v.as_f64()).unwrap_or(0.0);
    match kind {
        "frags" => format!("{} fragments", logger::num(n.max(1.0))),
        "dragon" => format!("dragon {}", logger::clean_text(name)),
        _ => {
            let role = prize.get("role").and_then(|v| v.as_str()).unwrap_or("");
            let label = if role.is_empty() { name } else { role };
            format!("{} {}", logger::clean_text(label), logger::num(n))
        }
    }
}
