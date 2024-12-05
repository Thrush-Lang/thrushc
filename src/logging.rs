use {
    std::io::{self, Write},
    stylic::{style, Styled, Stylize},
};

#[derive(PartialEq)]
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

#[inline]
pub fn log(ltype: LogType, msg: &str) {
    if ltype == LogType::ERROR {
        io::stderr()
            .write_all(format!("  \n{} {}\n", ltype.to_styled(), style(msg).bold()).as_bytes())
            .unwrap();

        return;
    }

    io::stdout()
        .write_all(format!("  {} {}", ltype.to_styled(), style(msg).bold()).as_bytes())
        .unwrap();
}
