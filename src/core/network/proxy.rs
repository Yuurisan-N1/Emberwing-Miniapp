use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::core::constants::PROXY_FILE;

#[derive(Debug, Clone)]
pub struct ProxyEntry {
    pub url: String,
    pub raw: String,
}

#[derive(Debug, Clone)]
pub struct ProxyPool {
    inner: Arc<Mutex<ProxyPoolInner>>,
}

#[derive(Debug)]
struct ProxyPoolInner {
    proxies: Vec<ProxyEntry>,
    idx: usize,
}

impl ProxyPool {
    pub fn load() -> Self {
        let proxies = load_proxies_from_file();
        ProxyPool {
            inner: Arc::new(Mutex::new(ProxyPoolInner { proxies, idx: 0 })),
        }
    }

    pub fn current_label(&self) -> String {
        let inner = self.inner.lock().unwrap();
        if inner.proxies.is_empty() {
            return "local".to_string();
        }
        mask_proxy(&inner.proxies[inner.idx % inner.proxies.len()].raw)
    }

    pub fn current_url(&self) -> Option<String> {
        let inner = self.inner.lock().unwrap();
        if inner.proxies.is_empty() {
            return None;
        }
        Some(inner.proxies[inner.idx % inner.proxies.len()].url.clone())
    }

    pub fn rotate(&self) -> bool {
        let mut inner = self.inner.lock().unwrap();
        if inner.proxies.is_empty() {
            return false;
        }
        inner.idx = (inner.idx + 1) % inner.proxies.len();
        true
    }

    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().proxies.len()
    }
}

fn load_proxies_from_file() -> Vec<ProxyEntry> {
    let path = proxy_path();
    if !path.exists() {
        return vec![];
    }
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            parse_proxy_line(line)
        })
        .collect()
}

fn parse_proxy_line(line: &str) -> Option<ProxyEntry> {
    let url = if line.starts_with("http://")
        || line.starts_with("https://")
        || line.starts_with("socks5://")
        || line.starts_with("socks5h://")
    {
        line.to_string()
    } else {
        let parts: Vec<&str> = line.split(':').collect();
        match parts.len() {
            2 => format!("http://{}:{}", parts[0].trim(), parts[1].trim()),
            4 => format!(
                "http://{}:{}@{}:{}",
                parts[2].trim(),
                parts[3].trim(),
                parts[0].trim(),
                parts[1].trim()
            ),
            _ => return None,
        }
    };
    Some(ProxyEntry {
        url: url.clone(),
        raw: url,
    })
}

fn mask_proxy(raw: &str) -> String {
    if let Ok(parsed) = url::Url::parse(raw) {
        let host = parsed.host_str().unwrap_or("");
        let port = parsed
            .port()
            .map(|p| format!(":{}", p))
            .unwrap_or_default();
        let masked = if host.len() > 4 {
            format!("{}***", &host[..4])
        } else {
            format!("{}***", host)
        };
        return format!("{}://{}{}", parsed.scheme(), masked, port);
    }
    "proxy***".to_string()
}

pub fn proxy_path() -> PathBuf {
    let mut p = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    p.pop();
    p.push(PROXY_FILE);
    if !p.exists() {
        let local = PathBuf::from(PROXY_FILE);
        if local.exists() {
            return local;
        }
    }
    p
}
