use stylic::{style, Stylize};

pub enum LogType {
    INFO,
    WARN,
    ERROR,
}

impl LogType {
    fn to_str(&self) -> &str {
        match self {
            LogType::INFO => "INFO",
            LogType::WARN => "WARN",
            LogType::ERROR => "ERROR",
        }
    }
}

/// Logs a message to the compiler standard output (CSO)
#[inline]
pub fn log(ltype: LogType, msg: &str) {
    println!(
        "{} {}",
        style(ltype.to_str()).bold().bright_red(),
        style(msg).bold()
    );
}
