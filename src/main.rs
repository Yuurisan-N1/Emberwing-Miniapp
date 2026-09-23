mod core;
mod features;

use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;

use crate::core::banner;
use crate::core::config;
use crate::core::http::Api;
use crate::core::logger::{self, lg, lr, ly};
use crate::core::proxy::ProxyPool;
use crate::core::state::Snapshot;
use crate::features::auth::account;
use crate::features::Ctx;

#[tokio::main]
async fn main() {
    tokio::select! {
        result = run() => {
            if let Err(e) = result {
                lr(&format!("Bot ended due to unexpected error: {}", logger::sanitize(&e.to_string())));
            }
        }
        _ = tokio::signal::ctrl_c() => {
            println!();
            lr("Script stopped by user");
        }
    }
}

async fn run() -> Result<()> {
    banner::print_banner();

    let cfg = match config::load_config() {
        Ok(c) => c,
        Err(e) => {
            ly(&e.to_string());
            return Ok(());
        }
    };

    let (acct, ignored) = account::load_account()?;
    let api = Arc::new(Api::new(ProxyPool::load())?);

    lg(&format!("Account {}", acct.label));
    lg(&format!("Telegram id {}", acct.user_id));
    if let Some(sp) = account::start_param(&acct.init_data) {
        lg(&format!("Start app {}", logger::clean_text(&sp)));
    }
    lg(&format!(
        "Mode single account, data.txt line {}",
        acct.source_line
    ));
    if ignored > 0 {
        ly(&format!(
            "data.txt carries {} extra initData lines, this bot runs one account",
            ignored
        ));
    }
    lg(&format!("Using proxy {}", api.proxy_label()));
    ly(&format!(
        "Cycle every {} with the loop {}",
        logger::clock(cfg.sleep_seconds() as i64),
        if cfg.loop_cycle.enabled { "on" } else { "off (single run)" }
    ));

    let mut st = Snapshot::default();
    let mut cycle: u64 = 0;

    features::presence::spawn(api.clone(), cfg.presence.enabled);

macro_rules! step {
    ($label:expr, $f:path) => {{
        let mut ctx = Ctx {
            api: api.clone(),
            st: &mut st,
            cfg: &cfg,
            init_data: &acct.init_data,
        };
        if let Err(e) = $f(&mut ctx).await {
            logger::fail($label, &format!("failed, {}", logger::sanitize(&e.to_string())));
        }
    }};
}

    loop {
        cycle += 1;
        ly(&format!("Cycle {} execution started", cycle));

        if api.maintenance_left() > 0 {
            let left = api.maintenance_left() as i64;
            ly(&format!(
                "Server maintenance: {} left, holding the cycle",
                logger::clock(left)
            ));
            sleep_countdown((left.min(300)) as u64).await;
            continue;
        }

        if api.token().is_none() && !login(&api, &mut st, &cfg, &acct).await {
            lr("Could not open a session, retrying next cycle");
            sleep_countdown(cfg.sleep_seconds()).await;
            continue;
        }

        refresh(&api, &mut st).await;

        step!("pending battle", features::arena::resume_pending);
        step!("notices", features::notices::run);
        step!("daily gift", features::daily_gift::run);
        step!("referrals", features::referrals::run);
        step!("quests", features::quests::run);
        step!("achievements", features::achievements::run);
        step!("festival and top up", features::event::run);
        step!("pass", features::pass::run);
        step!("island", features::island::run);
        step!("islanders", features::villagers::run);
        step!("hatchery", features::eggs::run);
        step!("dragons", features::dragons::run);
        step!("star fragments", features::dragons::convert_fragments);
        step!("forge", features::gear::run);
        step!("arena farm", features::arena::farm::run);
        step!("arena", features::arena::run);
        step!("expeditions", features::expeditions::run);
        step!("raids", features::raids::run);
        step!("hunt camps boss fog", features::hunts::run);
        step!("tavern", features::tavern::run);

        let closing = fetch_state(&api, &mut st).await;
        lg(&format!("Cycle {} execution completed", cycle));
        ly(&format!(
            "Holdings gold {}, emb {}, meat {}, trophies {}, dragons {}, eggs {}",
            logger::num(st.player.gold),
            logger::num(st.player.emb),
            logger::num(st.meat_now()),
            st.player.trophies,
            st.dragons.len(),
            st.eggs.len()
        ));
        if !closing.is_empty() {
            lg(&closing);
        }

        let net_errors = *api.net_errors.read().unwrap();
        if net_errors > 0 {
            ly(&format!("{} network error(s) this cycle", net_errors));
            if api.proxy_pool.len() > 0 && api.rotate_proxy() {
                ly(&format!("rotated to {}", api.proxy_label()));
            }
        }

        if !cfg.loop_cycle.enabled {
            lg("Cycle loop is off, exiting");
            break;
        }
        sleep_countdown(cfg.sleep_seconds()).await;
    }

    Ok(())
}

async fn login(api: &Arc<Api>, st: &mut Snapshot, cfg: &config::Config, acct: &account::Account) -> bool {
    for attempt in 1..=3u32 {
        let mut ctx = Ctx {
            api: api.clone(),
            st,
            cfg,
            init_data: &acct.init_data,
        };
        match features::session::run(&mut ctx).await {
            Ok(true) => return true,
            Ok(false) => ly(&format!("Auth refused, attempt {}/3", attempt)),
            Err(e) => lr(&format!("Auth error: {}", e)),
        }
        sleep_countdown(30).await;
    }
    false
}

async fn refresh(api: &Arc<Api>, st: &mut Snapshot) {
    let line = fetch_state(api, st).await;
    if !line.is_empty() {
        lg(&line);
    }
}

async fn fetch_state(api: &Arc<Api>, st: &mut Snapshot) -> String {
    match api.get("/me").await {
        Ok(res) if res.ok => {
            if let Some(sn) = res.snapshot() {
                st.apply(sn);
            }
            if !st.cfg_v.is_empty() {
                let v = st.cfg_v.clone();
                api.set_cfg_v(&v);
            }
            format!(
                "State {} with gold {}, logs {}, meat {}, trophies {}, league {}, island {}",
                st.label(),
                logger::num(st.player.gold),
                logger::num(st.player.logs),
                logger::num(st.meat_now()),
                st.player.trophies,
                st.league_idx() + 1,
                if st.island { "open" } else { "locked" }
            )
        }
        Ok(res) => {
            if res.status == 401 {
                lr("State session expired, signing in again");
                api.clear_token();
                String::new()
            } else {
                ly(&format!("State {}", res.reason()));
                String::new()
            }
        }
        Err(e) => {
            lr(&format!("State {}", e));
            String::new()
        }
    }
}

async fn sleep_countdown(seconds: u64) {
    let mut remaining = seconds;
    while remaining > 0 {
        let h = remaining / 3600;
        let m = (remaining % 3600) / 60;
        let s = remaining % 60;
        print!(
            "\r\x1b[1;33mNext cycle in {:02}:{:02}:{:02}\x1b[0m",
            h, m, s
        );
        let _ = std::io::Write::flush(&mut std::io::stdout());
        tokio::time::sleep(Duration::from_secs(1)).await;
        remaining -= 1;
    }
    println!();
}
