use stylic::{style, Styled, Stylize};

pub enum LogType {
    INFO,
    WARN,
    ERROR,
}

impl LogType {
    fn to_styled(&self) -> Styled<&str> {
        match self {
            LogType::INFO => style("INFO").bold().bright_green(),
            LogType::WARN => style("WARN").bold().bright_yellow(),
            LogType::ERROR => style("ERROR").bold().bright_red(),
        }
    }
}

/// Logs a message to the compiler standard output (CSO)
#[inline]
pub fn log(ltype: LogType, msg: &str) {
    println!("  {} {}", ltype.to_styled(), style(msg).bold());
}
