use {chrono::Local, colored::Colorize};

pub struct Logging {
    pub text: String,
}

impl Logging {
    pub fn new(text: String) -> Self {
        Self { text }
    }

    #[inline]
    pub fn error(&self) {
        eprintln!(
            "- {} - {}{}{} {}",
            Local::now().format("Y-%m-%d %H:%M:%S").to_string().bold(),
            "[".bold(),
            "ERROR".bold().bright_red(),
            "]".bold(),
            self.text.bold()
        );
    }

    #[inline]
    pub fn info(&self) {
        println!(
            "- {} - {}{}{} {}",
            Local::now().format("Y-%m-%d %H:%M:%S").to_string().bold(),
            "[".bold(),
            "INFO".bold().bright_green(),
            "]".bold(),
            self.text.bold()
        );
    }
}
