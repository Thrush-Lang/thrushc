use {chrono::Local, colored::Colorize};

#[inline]
pub fn error(msg: &str) {
    eprintln!(
        "- {} - {}{}{} {}",
        Local::now().format("%H:%M:%S").to_string().bold(),
        "[".bold(),
        "ERROR".bold().bright_red(),
        "]".bold(),
        msg.bold()
    );
}

#[inline]
pub fn info(msg: &str) {
    println!(
        "- {} - {}{}{} {}",
        Local::now().format("%H:%M:%S").to_string().bold(),
        "[".bold(),
        "INFO".bold().bright_green(),
        "]".bold(),
        msg.bold()
    );
}
