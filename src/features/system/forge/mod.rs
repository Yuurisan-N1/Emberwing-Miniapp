use anyhow::Result;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use crate::core::logger;
use crate::features::Ctx;

mod belief;
mod bits;
mod hw;
mod logic;
mod search;
use logic::{plan, parse_geo, Board, Boost, Budget, Move};

const MAX_MOVES: u32 = 500;
const MAX_REFUSED: u32 = 3;
const MAX_TRIES: u32 = 240;

struct Ticker {
    stop: Arc<AtomicBool>,
    join: Option<std::thread::JoinHandle<()>>,
}

impl Ticker {
    fn new(stage: i64, moves: u32, c0: u64, t0: Instant) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let join = std::thread::spawn(move || {
            while !flag.load(Ordering::Relaxed) {
                let el = t0.elapsed().as_secs_f64().max(1e-6);
                let dc = logic::combos().saturating_sub(c0) as f64;
                logger::live(&format!(
                    "Forge stage {} move {} combos per second {}",
                    stage,
                    moves,
                    logic::fmt_count(dc / el)
                ));
                std::thread::sleep(std::time::Duration::from_millis(150));
            }
        });
        Ticker {
            stop,
            join: Some(join),
        }
    }

    fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.join.take() {
            let _ = h.join();
        }
    }
}

pub async fn run(ctx: &mut Ctx<'_>) -> Result<()> {
    if !ctx.cfg.forge.enabled {
        return Ok(());
    }

    logger::vt_on();
    logger::lg(&format!("Forge hardware: {}", hw::describe()));
    if hw::cuda_ready() {
        logger::ok("forge", "cuda backend armed, monte carlo runs on the gpu");
    } else {
        logger::skip(
            "forge",
            &format!("{} backend, monte carlo runs on the cpu pool", hw::backend().tag()),
        );
    }

    let res = ctx.get("/event/tile/state").await?;
    if !res.ok {
        logger::skip("forge", &format!("state: {}", res.reason()));
        return Ok(());
    }
    let mut tile = res.get("tile").cloned().unwrap_or(Value::Null);
    if tile.is_null() {
        logger::skip("forge", "festival not running");
        return Ok(());
    }

    let live = res.get("live").cloned().unwrap_or(Value::Null);
    let mut cleared = 0u32;
    if live.get("v").map(|v| !v.is_null()).unwrap_or(false) {
        let stage = live.get("stage").and_then(|v| v.as_i64()).unwrap_or(0);
        logger::ok("forge", &format!("stage {} attempt resumed", stage));
        let st = play(ctx, &live, &mut tile).await?;
        logger::skip("forge", &format!("stage {} attempt {}", stage, st));
        if st == "won" {
            cleared += 1;
        }
        if let Some(t) = refresh(ctx).await {
            tile = t;
        }
    }

    let mut tries = 0u32;
    while ctx.cfg.forge.max_stages == 0 || cleared < ctx.cfg.forge.max_stages {
        if tries >= MAX_TRIES {
            break;
        }
        if !festival_open(&tile, ctx.now()) {
            break;
        }
        if tile.get("playable").and_then(|v| v.as_bool()) == Some(false) {
            logger::skip("forge", "all stages cleared");
            break;
        }
        let free = tickets(ctx, &tile);
        if free <= 0 {
            logger::skip("forge", "no tickets, waiting for the refill");
            break;
        }
        let stage = tile.get("stage").and_then(|v| v.as_i64()).unwrap_or(0) + 1;
        let r = ctx
            .act("/event/tile/start", json!({ "stage": stage }))
            .await?;
        if !r.ok {
            logger::skip("forge", &format!("stage {} start: {}", stage, r.reason()));
            break;
        }
        if let Some(t) = r.get("tile") {
            tile = t.clone();
        }
        let live = r.get("live").cloned().unwrap_or(Value::Null);
        if live.is_null() {
            break;
        }
        tries += 1;
        let st = play(ctx, &live, &mut tile).await?;
        if st != "won" {
            logger::skip("forge", &format!("stage {} {}", stage, st));
            if st == "no move" {
                break;
            }
            if let Some(t) = refresh(ctx).await {
                tile = t;
            }
            continue;
        }
        cleared += 1;
        logger::ok("forge", &format!("stage {} cleared", stage));
        if let Some(t) = refresh(ctx).await {
            tile = t;
        }
    }

    if cleared == 0 {
        let stage = tile.get("stage").and_then(|v| v.as_i64()).unwrap_or(0) + 1;
        let free = tickets(ctx, &tile);
        logger::skip(
            "forge",
            &format!("stage {} next, {} ticket(s) held", stage, free),
        );
    }
    Ok(())
}

async fn refresh(ctx: &mut Ctx<'_>) -> Option<Value> {
    match ctx.get("/event/tile/state").await {
        Ok(res) if res.ok => res.get("tile").cloned(),
        _ => None,
    }
}

fn festival_open(tile: &Value, now: i64) -> bool {
    match tile.get("finishUntil").and_then(|v| v.as_i64()) {
        Some(until) if until > 0 => until > now,
        _ => true,
    }
}

fn tickets(ctx: &Ctx<'_>, tile: &Value) -> i64 {
    let tix = tile.get("tix").cloned().unwrap_or(Value::Null);
    let cfg = ctx.st.ev_cfg.get("tile").cloned().unwrap_or(Value::Null);
    let cap = tix
        .get("cap")
        .and_then(|v| v.as_i64())
        .or_else(|| cfg.get("tixCap").and_then(|v| v.as_i64()))
        .unwrap_or(10);
    let per = cfg.get("tixMin").and_then(|v| v.as_i64()).unwrap_or(60).max(1) * 60_000;
    let mut free = tix.get("free").and_then(|v| v.as_i64()).unwrap_or(0);
    let mut next = tix.get("nextAt").and_then(|v| v.as_i64()).unwrap_or(0);
    let now = ctx.now();
    while next > 0 && now >= next && free < cap {
        free += 1;
        next = if free < cap { next + per } else { 0 };
    }
    let paid = tix.get("paid").and_then(|v| v.as_i64()).unwrap_or(0);
    free + paid
}

async fn play(ctx: &mut Ctx<'_>, live: &Value, tile: &mut Value) -> Result<String> {
    let aid = live.get("aid").and_then(|v| v.as_i64()).unwrap_or(0);
    let geo = parse_geo(live.get("geo"));
    let rules = live.get("rules").cloned().unwrap_or(Value::Null);
    let slots = rules.get("slots").and_then(|v| v.as_u64()).unwrap_or(7) as usize;
    let stash = rules.get("stash").and_then(|v| v.as_u64()).unwrap_or(3) as usize;
    let mut v = live.get("v").cloned().unwrap_or(Value::Null);
    if v.is_null() || geo.is_empty() {
        return give_up(ctx, aid, "closed").await;
    }

    let mut boost = Boost::of(tile);
    let mut moves = 0u32;
    let mut refused = 0u32;
    let envu = |k: &str| std::env::var(k).ok().and_then(|s| s.parse::<u64>().ok());
    let bud = Budget {
        samples: envu("EMB_FORGE_SAMPLES").unwrap_or(ctx.cfg.forge.samples as u64) as u32,
        threads: envu("EMB_FORGE_THREADS").unwrap_or(ctx.cfg.forge.threads as u64) as usize,
        deadline_ms: envu("EMB_FORGE_DEADLINE_MS")
            .unwrap_or(ctx.cfg.forge.deadline_ms.max(1)),
        nodes: envu("EMB_FORGE_NODES").unwrap_or(ctx.cfg.forge.nodes.max(1)),
        worlds: envu("EMB_FORGE_WORLDS").unwrap_or(ctx.cfg.forge.worlds as u64) as usize,
    };

    loop {
        if moves >= MAX_MOVES || refused >= MAX_REFUSED {
            return give_up(ctx, aid, "stuck").await;
        }
        let b = Board::from(&v, &geo, slots, stash);
        if b.st != "live" {
            return give_up(ctx, aid, &b.st).await;
        }
        let stage = live.get("stage").and_then(|v| v.as_i64()).unwrap_or(0);
        let t0 = Instant::now();
        let c0 = logic::combos();
        let mut tick = Ticker::new(stage, moves, c0, t0);
        let hit = plan(&b, &boost, &bud);
        tick.stop();
        if let Some(rep) = logic::gpu_report_once() {
            logger::lg(&format!("Forge {}", rep));
        }
        let mv = match hit.map(|(m, _, _)| m) {
            Some(m) => m,
            None => {
                logger::rlive(&format!(
                    "Forge stage {} stuck, tray {} of {} hidden {}",
                    stage,
                    b.tray.len(),
                    slots,
                    logic::hidden_left(&b)
                ));
                if ctx.cfg.forge.buy_boosters && boost.can_buy(&b) && buy(ctx, &b, &mut boost, tile).await? {
                    continue;
                }
                return give_up(ctx, aid, "no move").await;
            }
        };
        if boost.holds(&mv) {
            boost.take(&mv);
        }
        moves += 1;
        let wire = vec![json!(mv.wire(b.seq + 1))];
        let r = ctx
            .act("/event/tile/move", json!({ "aid": aid, "moves": wire }))
            .await?;
        if let Some(t) = r.get("tile") {
            *tile = t.clone();
        }
        if !r.ok {
            let st = r.status;
            if st == 409 || st == 404 || (st == 400 && r.data.get("error").is_some()) {
                return give_up(ctx, aid, "closed").await;
            }
            continue;
        }
        if let Some(won) = r.get("won").filter(|w| !w.is_null()) {
            let rw = won.get("rw").cloned().unwrap_or(Value::Null);
            let stage = won.get("stage").and_then(|v| v.as_i64()).unwrap_or(0);
            logger::ok("forge", &format!("stage {} won, {}", stage, reward(&rw)));
        }
        if let Some(rej) = r.get("rej").filter(|x| !x.is_null()) {
            let why = rej.get("why").and_then(|v| v.as_str()).unwrap_or("");
            refused += 1;
            match r.get("boost").filter(|b| !b.is_null()) {
                Some(b2) => boost.sync(b2),
                None => {
                    if boost.holds(&mv) {
                        boost.restore(&mv);
                    }
                }
            }
            if boost.holds(&mv) {
                logger::skip("forge", &format!("move refused: {}", why));
            }
        } else {
            refused = 0;
            if let Some(b2) = r.get("boost") {
                boost.sync(b2);
            }
        }
        if let Some(v2) = r.get("v") {
            if v2.is_null() {
                return give_up(ctx, aid, "closed").await;
            }
            v = v2.clone();
            let nb = Board::from(&v, &geo, slots, stash);
            logger::live(&format!(
                "Forge stage {} move {} tap {} tray {}/{}",
                stage,
                moves,
                match mv {
                    Move::Tap(i) => i as i64,
                    _ => -1,
                },
                nb.tray.len(),
                slots
            ));
        } else {
            return give_up(ctx, aid, "closed").await;
        }
    }
}

async fn give_up(ctx: &mut Ctx<'_>, aid: i64, st: &str) -> Result<String> {
    if aid > 0 && st != "won" {
        quit(ctx, aid).await;
    }
    Ok(st.to_string())
}

async fn quit(ctx: &mut Ctx<'_>, aid: i64) {
    let _ = ctx
        .act("/event/tile/quit", json!({ "aid": aid }))
        .await;
}

fn reward(rw: &Value) -> String {
    if rw.is_null() {
        return "rewards claimed".to_string();
    }
    let logs = rw.get("logs").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let gold = rw.get("gold").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let sparks = rw.get("sparks").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let embers = rw.get("embers").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let jokers = rw
        .get("jokers")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|j| j.as_array().and_then(|p| p.first()).and_then(|v| v.as_i64()))
                .count()
        })
        .unwrap_or(0);
    format!(
        "logs {}, gold {}, sparks {}, embers {}, {} joker(s)",
        logger::num(logs),
        logger::num(gold),
        logger::num(sparks),
        logger::num(embers),
        jokers
    )
}

async fn buy(ctx: &mut Ctx<'_>, b: &Board, boost: &mut Boost, tile: &mut Value) -> Result<bool> {
    let pack = match boost.want(b) {
        Some(p) => p,
        None => return Ok(false),
    };
    let nonce = nonce();
    let r = ctx
        .act("/event/tile/pack", json!({ "pack": pack, "nonce": nonce }))
        .await?;
    if let Some(t) = r.get("tile") {
        *tile = t.clone();
    }
    if !r.ok {
        logger::skip("forge", &format!("{} pack: {}", pack, r.reason()));
        return Ok(false);
    }
    boost.sync(tile.get("boost").unwrap_or(&Value::Null));
    logger::ok("forge", &format!("{} pack bought", pack));
    Ok(true)
}

fn nonce() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{:x}{:x}", ms, std::process::id())
}
