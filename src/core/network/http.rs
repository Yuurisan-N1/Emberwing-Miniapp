use anyhow::{anyhow, Result};
use reqwest::header::{
    HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, CONTENT_TYPE, ORIGIN, REFERER, USER_AGENT,
};
use reqwest::{Client, Method};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::core::constants::{
    API_PREFIX, APP_ORIGIN, BASE_URL, MAX_RATE_WAIT_SEC, READ_TIMEOUT_SEC, UA, WRITE_TIMEOUT_SEC,
};
use crate::core::logger::{server_reason, skip};
use crate::core::proxy::ProxyPool;
use crate::core::state::Snapshot;

#[derive(Debug, Clone)]
pub struct Res {
    pub status: u16,
    pub ok: bool,
    pub error: String,
    pub data: Value,
    pub ms: u64,
}

impl Res {
    pub fn snapshot(&self) -> Option<&Value> {
        self.data.get("snapshot").filter(|v| !v.is_null())
    }

    pub fn reason(&self) -> String {
        if !self.error.is_empty() {
            return server_reason(&self.error);
        }
        if let Some(e) = self.data.get("error").and_then(|v| v.as_str()) {
            return server_reason(e);
        }
        if self.ok {
            String::new()
        } else {
            format!("http {}", self.status)
        }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.data.get(key)
    }

    pub fn num_of(&self, key: &str) -> f64 {
        match self.data.get(key) {
            Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
            Some(Value::String(s)) => s.parse().unwrap_or(0.0),
            _ => 0.0,
        }
    }

    pub fn bool_of(&self, key: &str) -> bool {
        match self.data.get(key) {
            Some(Value::Bool(b)) => *b,
            Some(Value::Number(n)) => n.as_i64().unwrap_or(0) != 0,
            _ => false,
        }
    }

    pub fn arr(&self, key: &str) -> Vec<Value> {
        self.data
            .get(key)
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
    }

}

pub struct Api {
    http: RwLock<Client>,
    token: RwLock<Option<String>>,
    cfg_v: RwLock<Option<String>>,
    hold: RwLock<HashMap<String, Instant>>,
    maint_until: RwLock<Option<Instant>>,
    pub proxy_pool: ProxyPool,
    pub net_errors: RwLock<u32>,
}

impl Api {
    pub fn new(proxy_pool: ProxyPool) -> Result<Self> {
        let http = build_client(&proxy_pool)?;
        Ok(Api {
            http: RwLock::new(http),
            token: RwLock::new(None),
            cfg_v: RwLock::new(None),
            hold: RwLock::new(HashMap::new()),
            maint_until: RwLock::new(None),
            proxy_pool,
            net_errors: RwLock::new(0),
        })
    }

    pub fn set_token(&self, token: &str) {
        *self.token.write().unwrap() = Some(token.to_string());
    }

    pub fn token(&self) -> Option<String> {
        self.token.read().unwrap().clone()
    }

    pub fn clear_token(&self) {
        *self.token.write().unwrap() = None;
    }

    pub fn set_cfg_v(&self, v: &str) {
        if !v.is_empty() {
            *self.cfg_v.write().unwrap() = Some(v.to_string());
        }
    }

    pub fn maintenance_left(&self) -> u64 {
        match *self.maint_until.read().unwrap() {
            Some(t) => t.saturating_duration_since(Instant::now()).as_secs(),
            None => 0,
        }
    }

    pub fn hold_left(&self, path: &str) -> u64 {
        let m = self.hold.read().unwrap();
        match m.get(path) {
            Some(t) => t.saturating_duration_since(Instant::now()).as_secs(),
            None => 0,
        }
    }

    fn park(&self, path: &str, secs: u64) {
        let mut m = self.hold.write().unwrap();
        m.insert(
            path.to_string(),
            Instant::now() + Duration::from_secs(secs.max(1)),
        );
    }

    pub async fn auth(&self, init_data: &str) -> Result<Res> {
        self.call("/auth", Some(json!({ "initData": init_data })), true)
            .await
    }

    pub async fn get(&self, path: &str) -> Result<Res> {
        self.call(path, None, false).await
    }

    pub async fn post(&self, path: &str, body: Value) -> Result<Res> {
        self.call(path, Some(body), false).await
    }

    pub async fn act(&self, path: &str, body: Value, st: &mut Snapshot) -> Result<Res> {
        let res = self.call(path, Some(body), false).await?;
        if let Some(sn) = res.snapshot() {
            st.apply(sn);
            if !st.cfg_v.is_empty() {
                let v = st.cfg_v.clone();
                self.set_cfg_v(&v);
            }
        }
        Ok(res)
    }

    pub async fn call(&self, path: &str, body: Option<Value>, auth_call: bool) -> Result<Res> {
        if !auth_call {
            let left = self.hold_left(path);
            if left > 0 {
                return Ok(Res {
                    status: 429,
                    ok: false,
                    error: format!("cooldown {}s", left),
                    data: Value::Null,
                    ms: 0,
                });
            }
            let mleft = self.maintenance_left();
            if mleft > 0 {
                return Ok(Res {
                    status: 503,
                    ok: false,
                    error: format!("server maintenance {}s left", mleft),
                    data: Value::Null,
                    ms: 0,
                });
            }
        }

        let mut last = Res {
            status: 0,
            ok: false,
            error: "no attempt".to_string(),
            data: Value::Null,
            ms: 0,
        };

        for attempt in 0..2u32 {
            match self.once(path, body.clone()).await {
                Ok(res) => {
                    last = res;
                    if last.status == 429 {
                        let wait = cooldown_secs(&last.data);
                        self.park(path, wait);
                        if attempt == 0 && wait <= MAX_RATE_WAIT_SEC {
                            tokio::time::sleep(Duration::from_secs(wait.min(5))).await;
                            continue;
                        }
                    }
                    if last.status == 503 {
                        if let Some(min) = last.data.get("min").and_then(|v| v.as_f64()) {
                            *self.maint_until.write().unwrap() = Some(
                                Instant::now() + Duration::from_secs((min.max(1.0) as u64) * 60),
                            );
                            skip("maint", &format!("server maintenance: {} min", min));
                        }
                    }
                    break;
                }
                Err(e) => {
                    *self.net_errors.write().unwrap() += 1;
                    last = Res {
                        status: 0,
                        ok: false,
                        error: format!("{}", e),
                        data: Value::Null,
                        ms: 0,
                    };
                    if attempt == 0 {
                        tokio::time::sleep(Duration::from_millis(800)).await;
                        continue;
                    }
                }
            }
        }

        Ok(last)
    }

    async fn once(&self, path: &str, body: Option<Value>) -> Result<Res> {
        let url = format!("{}{}{}", BASE_URL, API_PREFIX, path);
        let is_write = body.is_some();
        let method = if is_write { Method::POST } else { Method::GET };

        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static(UA));
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/json, text/plain, */*"),
        );
        headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));
        headers.insert(ORIGIN, HeaderValue::from_static(APP_ORIGIN));
        headers.insert(REFERER, HeaderValue::from_static(APP_ORIGIN));
        if is_write {
            headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        }
        if let Some(t) = self.token() {
            if let Ok(v) = HeaderValue::from_str(&format!("Bearer {}", t)) {
                headers.insert("Authorization", v);
            }
        }
        if let Some(v) = self.cfg_v.read().unwrap().clone() {
            if let Ok(hv) = HeaderValue::from_str(&v) {
                headers.insert("X-Cfg-V", hv);
            }
        }

        let timeout = if is_write {
            WRITE_TIMEOUT_SEC
        } else {
            READ_TIMEOUT_SEC
        };
        let rb = self
            .http
            .read()
            .unwrap()
            .request(method, &url)
            .headers(headers)
            .timeout(Duration::from_secs(timeout));
        let rb = match body {
            Some(b) => rb.body(serde_json::to_vec(&b)?),
            None => rb,
        };

        let t0 = Instant::now();
        let resp = rb.send().await.map_err(|e| anyhow!("{}", e))?;
        let status = resp.status().as_u16();
        let text = resp.text().await.map_err(|e| anyhow!("{}", e))?;
        let ms = t0.elapsed().as_millis() as u64;
        let data: Value = serde_json::from_str(&text).unwrap_or(Value::Null);

        let error = if (200..300).contains(&status) {
            String::new()
        } else if let Some(e) = data.get("error").and_then(|v| v.as_str()) {
            e.to_string()
        } else {
            format!("http {}", status)
        };

        Ok(Res {
            status,
            ok: (200..300).contains(&status),
            error,
            data,
            ms,
        })
    }

    pub fn rotate_proxy(&self) -> bool {
        if !self.proxy_pool.rotate() {
            return false;
        }
        if let Ok(c) = build_client(&self.proxy_pool) {
            *self.http.write().unwrap() = c;
        }
        true
    }

    pub fn proxy_label(&self) -> String {
        self.proxy_pool.current_label()
    }
}

fn build_client(pool: &ProxyPool) -> Result<Client> {
    let mut b = Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .pool_max_idle_per_host(2);
    if let Some(url) = pool.current_url() {
        b = b.proxy(reqwest::Proxy::all(&url).map_err(|e| anyhow!("bad proxy: {}", e))?);
    }
    Ok(b.build()?)
}

fn cooldown_secs(data: &Value) -> u64 {
    if let Some(ms) = data.get("retryMs").and_then(|v| v.as_f64()) {
        return ((ms / 1000.0).ceil() as u64).max(1);
    }
    match data.get("error").and_then(|v| v.as_str()) {
        Some(e) if e.contains("slow down") => 60,
        _ => 60,
    }
}

pub fn server_now_ms(st: &Snapshot) -> i64 {
    if st.now > 0 {
        return st.now;
    }
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
