pub const W: usize = 96;

use std::io::{IsTerminal, Write};
use std::sync::Mutex;

pub const G: &str = "\x1b[1;32m";
pub const Y: &str = "\x1b[1;33m";
pub const R: &str = "\x1b[1;31m";
pub const Z: &str = "\x1b[0m";

static WIDE: Mutex<usize> = Mutex::new(0);

#[cfg(windows)]
pub fn vt_on() {
    const STD_OUTPUT_HANDLE: u32 = 0xFFFF_FFF5;
    const VT: u32 = 0x0004;
    extern "system" {
        fn GetStdHandle(n: u32) -> *mut core::ffi::c_void;
        fn GetConsoleMode(h: *mut core::ffi::c_void, m: *mut u32) -> i32;
        fn SetConsoleMode(h: *mut core::ffi::c_void, m: u32) -> i32;
    }
    unsafe {
        let h = GetStdHandle(STD_OUTPUT_HANDLE);
        let mut m: u32 = 0;
        if GetConsoleMode(h, &mut m) != 0 {
            SetConsoleMode(h, m | VT);
        }
    }
}

#[cfg(not(windows))]
pub fn vt_on() {}

fn wide() -> std::sync::MutexGuard<'static, usize> {
    match WIDE.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    }
}

pub fn soft(color: &str, msg: &str) {
    let m = clip(msg);
    let n = m.chars().count();
    let mut out = std::io::stdout();
    let mut last = wide();
    if std::io::stdout().is_terminal() {
        let pad = if *last > n { *last - n } else { 0 };
        let _ = write!(out, "\r{}{}{}", color, m, Z);
        for _ in 0..pad {
            let _ = out.write_all(b" ");
        }
        let _ = out.flush();
        *last = n;
    } else {
        let _ = writeln!(out, "{}{}{}", color, m, Z);
        let _ = out.flush();
        *last = 0;
    }
}

pub fn live(msg: &str) {
    soft(Y, msg);
}

pub fn rlive(msg: &str) {
    soft(R, msg);
}

pub fn live_end() {
    let mut last = wide();
    if *last == 0 {
        return;
    }
    if std::io::stdout().is_terminal() {
        let mut out = std::io::stdout();
        let _ = write!(out, "\r");
        for _ in 0..*last {
            let _ = out.write_all(b" ");
        }
        let _ = write!(out, "\r");
        let _ = out.flush();
    }
    *last = 0;
}

fn hard(color: &str, msg: &str) {
    let mut last = wide();
    let mut out = std::io::stdout();
    if *last > 0 {
        let _ = out.write_all(b"\n");
    }
    let _ = writeln!(out, "{}{}{}", color, msg, Z);
    let _ = out.flush();
    *last = 0;
}

pub fn lg(msg: &str) {
    hard(G, msg);
}

pub fn ly(msg: &str) {
    hard(Y, msg);
}

pub fn lr(msg: &str) {
    hard(R, msg);
}

pub fn ok(tag: &str, msg: &str) {
    lg(&format!("{} {}", head(tag), clip(msg)));
}

pub fn skip(tag: &str, msg: &str) {
    ly(&format!("{} {}", head(tag), clip(msg)));
}

pub fn fail(tag: &str, msg: &str) {
    lr(&format!("{} {}", head(tag), clip(msg)));
}

fn head(tag: &str) -> String {
    let t = clean_text(tag);
    match t.as_str() {
        "exp" => "Expedition".to_string(),
        "gift" => "Daily gift".to_string(),
        "ref" => "Referrals".to_string(),
        "achv" => "Achievement".to_string(),
        "egg" => "Hatchery".to_string(),
        "area" => "Island area".to_string(),
        "" => "Bot".to_string(),
        other => {
            let mut c = other.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => "Bot".to_string(),
            }
        }
    }
}

pub fn clip(msg: &str) -> String {
    let one = clean_text(msg);
    if one.chars().count() <= W {
        return one;
    }
    let mut out: String = one.chars().take(W.saturating_sub(3)).collect();
    out.push_str("...");
    out
}

pub fn sanitize(text: &str) -> String {
    text.replace('[', " ")
        .replace(']', " ")
        .replace('#', " ")
        .replace('-', " ")
}

pub fn clean_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut sp = false;
    for c in text.chars() {
        if c.is_control() || c == '\u{feff}' {
            if c == '\n' || c == '\r' || c == '\t' {
                sp = true;
            }
            continue;
        }
        if c.is_whitespace() {
            sp = true;
            continue;
        }
        if sp && !out.is_empty() {
            out.push(' ');
        }
        sp = false;
        out.push(c);
    }
    out
}

pub fn server_reason(err: &str) -> String {
    let e = clean_text(err);
    for p in [
        "Bad Request:",
        "Unauthorized:",
        "Internal Server Error:",
        "Conflict:",
    ] {
        if let Some(rest) = e.strip_prefix(p) {
            return rest.trim().to_string();
        }
    }
    e
}

pub fn shorten(text: &str, max: usize) -> String {
    let t = clean_text(text);
    if t.chars().count() <= max {
        return t;
    }
    let mut out: String = t.chars().take(max.saturating_sub(3)).collect();
    out.push_str("...");
    out
}

pub fn num(n: f64) -> String {
    let v = n.abs();
    if v >= 1_000_000_000.0 {
        format!("{:.2}B", n / 1_000_000_000.0)
    } else if v >= 1_000_000.0 {
        format!("{:.2}M", n / 1_000_000.0)
    } else if v >= 1_000.0 {
        format!("{:.2}K", n / 1_000.0)
    } else {
        format!("{}", n.round() as i64)
    }
}

pub fn clock(secs: i64) -> String {
    let s = secs.max(0);
    let d = s / 86400;
    let h = (s % 86400) / 3600;
    let m = (s % 3600) / 60;
    let sec = s % 60;
    if d > 0 {
        format!("{}d {:02}:{:02}:{:02}", d, h, m, sec)
    } else {
        format!("{:02}:{:02}:{:02}", h, m, sec)
    }
}
