use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

use crate::core::http::Api;
use crate::core::logger;

pub fn spawn(api: Arc<Api>, enabled: bool) {
    if !enabled {
        return;
    }
    tokio::spawn(async move {
        let mut ticks: u64 = 0;
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            if api.token().is_none() {
                continue;
            }
            match api.post("/presence", json!({})).await {
                Ok(res) if res.ok => {
                    ticks += 1;
                    if ticks == 1 || ticks % 30 == 0 {
                        logger::skip(
                            "presence",
                            &format!("online ping {}, keepers online are not raided", ticks),
                        );
                    }
                }
                Ok(res) => {
                    if !crate::features::is_hold(&res.reason()) && ticks % 30 == 0 {
                        logger::skip("presence", &format!("ping refused: {}", res.reason()));
                    }
                }
                Err(e) => {
                    logger::fail("presence", &format!("ping failed: {}", e));
                }
            }
        }
    });
}
