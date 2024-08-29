use {chrono::Local, colored::Colorize};

pub struct Logging {
    pub text: String,
}

impl Logging {
    pub fn new(text: String) -> Self {
        Self { text }
    }

    #[inline]
    pub fn log(&self) {
        eprintln!(
            "- {} - {}{}{} {}",
            Local::now().to_string().bold(),
            "[".bold(),
            "LOG".bold().bright_cyan(),
            "]".bold(),
            self.text.bold()
        );
    }

    #[inline]
    pub fn error(&self) {
        eprintln!(
            "- {} - {}{}{} {}",
            Local::now().to_string().bold(),
            "[".bold(),
            "ERROR".bold().bright_red(),
            "]".bold(),
            self.text.bold()
        );
    }

    #[inline]
    pub fn warning(&self) {
        eprintln!(
            "- {} - {}{}{} {}",
            Local::now().to_string().bold(),
            "[".bold(),
            "WARN".bold().bright_yellow(),
            "]".bold(),
            self.text.bold()
        );
    }

    #[inline]
    pub fn info(&self) {
        println!(
            "- {} - {}{}{} {}",
            Local::now().to_string().bold(),
            "[".bold(),
            "INFO".bold().bright_black(),
            "]".bold(),
            self.text.bold()
        );
    }
}
