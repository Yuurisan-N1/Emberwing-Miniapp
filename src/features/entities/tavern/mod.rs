use anyhow::Result;
use serde_json::{json, Value};

use crate::features::Ctx;
use crate::core::logger;

pub async fn run(ctx: &mut Ctx<'_>) -> Result<()> {
    if !ctx.cfg.tavern.enabled {
        let tickets = tickets_of(ctx, &ctx.cfg.tavern.kind);
        if tickets > 0.0 {
            logger::skip(
                "tavern",
                &format!("{} tickets held, rolls disabled in config.json", logger::num(tickets)),
            );
        }
        return Ok(());
    }

    let kind = normalize(&ctx.cfg.tavern.kind);
    let mut left = tickets_of(ctx, &kind);
    if left < 1.0 {
        log_pity(ctx, &kind);
        return Ok(());
    }

    let mut rolled = 0u32;
    while rolled < ctx.cfg.tavern.max_rolls_per_cycle && left >= 1.0 {
        let res = ctx
            .act("/island/tavern/roll", json!({ "kind": kind, "n": 1 }))
            .await?;
        if !res.ok {
            logger::skip("tavern", &format!("roll: {}", res.reason()));
            break;
        }
        rolled += 1;
        logger::ok("tavern", &format!("{} hire, {}", kind, prize_of(&res)));
        left = tickets_of(ctx, &kind);
    }
    if rolled == 0 {
        log_pity(ctx, &kind);
    }
    Ok(())
}

fn normalize(kind: &str) -> String {
    if kind.eq_ignore_ascii_case("i") {
        "i".to_string()
    } else {
        "d".to_string()
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
