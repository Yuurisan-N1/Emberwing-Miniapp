use anyhow::{anyhow, Result};
use serde_json::Value;
use std::path::PathBuf;

use crate::core::constants::DATA_FILE;
use crate::core::logger::{clean_text, shorten};

#[derive(Debug, Clone)]
pub struct Account {
    pub init_data: String,
    pub user_id: String,
    pub label: String,
    pub source_line: usize,
}

pub fn load_account() -> Result<(Account, usize)> {
    let path = data_path();
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| anyhow!("{} could not be read: {}", path.display(), e))?;

    let mut lines = raw.lines().enumerate().filter(|(_, l)| is_init_data(l));
    let (idx, first) = match lines.next() {
        Some(v) => v,
        None => return Err(anyhow!("{} holds no initData line", path.display())),
    };
    let ignored = lines.count();

    let init_data = first.trim().to_string();
    validate(&init_data)?;

    let acct = Account {
        user_id: user_id(&init_data),
        label: label(&init_data),
        init_data,
        source_line: idx + 1,
    };
    Ok((acct, ignored))
}

fn is_init_data(line: &str) -> bool {
    let l = line.trim();
    !l.is_empty() && !l.starts_with('#') && l.contains("hash=") && l.contains("user=")
}

fn validate(init_data: &str) -> Result<()> {
    for need in ["hash=", "user=", "auth_date="] {
        if !init_data.contains(need) {
            return Err(anyhow!(
                "initData is missing its {} part",
                need.trim_end_matches('=')
            ));
        }
    }
    Ok(())
}

pub fn user_id(init_data: &str) -> String {
    let raw = field(init_data, "user").unwrap_or_default();
    let decoded = pct_decode(&raw);
    if let Ok(v) = serde_json::from_str::<Value>(&decoded) {
        if let Some(id) = v.get("id") {
            return id_to_string(id);
        }
    }
    String::new()
}

pub fn label(init_data: &str) -> String {
    let raw = field(init_data, "user").unwrap_or_default();
    let decoded = pct_decode(&raw);
    if let Ok(v) = serde_json::from_str::<Value>(&decoded) {
        if let Some(u) = v.get("username").and_then(|x| x.as_str()) {
            if !u.trim().is_empty() {
                return shorten(&clean_text(u), 20);
            }
        }
        if let Some(f) = v.get("first_name").and_then(|x| x.as_str()) {
            if !f.trim().is_empty() {
                return shorten(&clean_text(f), 20);
            }
        }
    }
    let id = user_id(init_data);
    if id.is_empty() {
        "account".to_string()
    } else {
        shorten(&id, 20)
    }
}

fn id_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}

fn field(init_data: &str, key: &str) -> Option<String> {
    let needle = format!("{}=", key);
    for part in init_data.split('&') {
        if let Some(rest) = part.strip_prefix(&needle) {
            return Some(rest.to_string());
        }
    }
    None
}

pub fn start_param(init_data: &str) -> Option<String> {
    field(init_data, "start_param").map(|v| pct_decode(&v))
}

pub fn pct_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'%' if i + 2 < b.len() => {
                let hi = hex(b[i + 1]);
                let lo = hex(b[i + 2]);
                match (hi, lo) {
                    (Some(h), Some(l)) => {
                        out.push(h * 16 + l);
                        i += 3;
                    }
                    _ => {
                        out.push(b[i]);
                        i += 1;
                    }
                }
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).to_string()
}

fn hex(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

pub fn data_path() -> PathBuf {
    let mut p = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    p.pop();
    p.push(DATA_FILE);
    if !p.exists() {
        let local = PathBuf::from(DATA_FILE);
        if local.exists() {
            return local;
        }
    }
    p
}
