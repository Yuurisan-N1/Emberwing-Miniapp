use chrono::Local;

pub fn print_banner() {
    clear_screen();
    set_title();

    let ascii = r#"__   __                _                     _
\ \ / /   _ _   _ _ __(_)___  __ _ _ __   __| | ___  ___ _   _ 
 \ V / | | | | | | '__| / __|/ _  | '_ \ / _  |/ _ \/ __| | | |
  | || |_| | |_| | |  | \__ \ (_| | | | | (_| |  __/\__ \ |_| |
  |_| \__,_|\__,_|_|  |_|___/\__,_|_| |_|\_,__|\___||___/\__,_|"#;

    println!("\x1b[1;36m{}\x1b[0m", ascii);
    println!();
    println!("\x1b[1;35mWelcome to Yuuri, Emberwing Miniapp\x1b[0m");
    println!("\x1b[1;32mJoin our community: https://t.me/Y3YuYuYo\x1b[0m");
    println!("\x1b[1;33mCurrent time: {}\x1b[0m\n", Local::now().format("%d-%m-%Y %H:%M:%S"));
}

fn clear_screen() {
    if cfg!(target_os = "windows") {
        let _ = std::process::Command::new("cmd").args(["/c", "cls"]).status();
    } else {
        print!("\x1b[2J\x1b[1;1H");
    }
}

fn set_title() {
    print!("\x1b]2;Emberwing Miniapp by : 佐賀県産 (YUURI)\x1b\\");
}
