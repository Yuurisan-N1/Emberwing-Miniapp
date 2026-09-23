pub const W: usize = 63;

pub fn lg(msg: &str) {
    println!("\x1b[1;32m{}\x1b[0m", msg);
}

pub fn ly(msg: &str) {
    println!("\x1b[1;33m{}\x1b[0m", msg);
}

pub fn lr(msg: &str) {
    println!("\x1b[1;31m{}\x1b[0m", msg);
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
    for p in ["Bad Request:", "Unauthorized:", "Internal Server Error:", "Conflict:"] {
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
