use {
    super::error::{ThrushError, ThrushErrorKind},
    super::{FILE_NAME_WITH_EXT, FILE_PATH},
    colored::Colorize,
    std::{fs::read_to_string, mem},
};

pub struct Diagnostic {
    error: ThrushError,
    buffer: String,
    drawer: String,
    span: (usize, usize),
    line: usize,
    msg: String,
}

impl Diagnostic {
    pub fn new(error: ThrushError) -> Self {
        let span: (usize, usize) = match error {
            ThrushError::Parse(_, _, _, _, span, _) => span,
            ThrushError::Lex(_, _, _, _, span, _) => span,
            _ => (0, 0),
        };

        let line: usize = match error {
            ThrushError::Parse(_, _, _, _, _, line) => line,
            ThrushError::Lex(_, _, _, _, _, line) => line,
            _ => 0,
        };

        let msg: String = match error {
            ThrushError::Parse(_, _, ref msg, _, _, _) => msg.clone(),
            ThrushError::Lex(_, _, ref msg, _, _, _) => msg.clone(),
            _ => String::new(),
        };

        Self {
            error,
            buffer: String::new(),
            drawer: String::new(),
            span,
            line,
            msg,
        }
    }

    pub fn report(&mut self) {
        let content: String = read_to_string(FILE_PATH.lock().unwrap().as_str()).unwrap();

        match mem::take(&mut self.error) {
            ThrushError::Parse(kind, lexeme, _, help, _, _) => match kind {
                ThrushErrorKind::ParsedNumber => {
                    self.print_report(content, lexeme, help);
                }
                ThrushErrorKind::UnreachableNumber => {
                    self.print_report(content, lexeme, help);
                }
                ThrushErrorKind::SyntaxError => {
                    self.print_report(content, lexeme, help);
                }
                _ => {}
            },

            ThrushError::Lex(kind, lexeme, _, help, _, _) => match kind {
                ThrushErrorKind::SyntaxError => {
                    self.print_report(content, lexeme, help);
                }

                ThrushErrorKind::ParsedNumber => {
                    self.print_report(content, lexeme, help);
                }

                ThrushErrorKind::UnreachableNumber => {
                    self.print_report(content, lexeme, help);
                }
                _ => {}
            },

            _ => {}
        }
    }

    fn print_report(&mut self, content: String, lexeme: String, help: String) {
        self.print_header();

        let line: &str = content
            .lines()
            .find(|line| line.contains(&lexeme))
            .expect("line not found in file.")
            .trim();

        self.add_space_to_line();
        self.draw_line(line);
        self.draw_main_focus(line.len());

        self.push_drawer();
        self.print_line();
        self.reset_buffer();
        self.reset_drawer();

        self.add_space_to_line();
        self.draw_line(lexeme.as_str());
        self.draw_main_focus(lexeme.len());
        self.push_drawer();
        self.print_line();

        println!(
            "\n{}{} {}\n",
            "Help".bold().bright_green(),
            ":".bold(),
            help.bold()
        );
    }

    fn print_header(&mut self) {
        println!(
            "\n{} {}{}{}\n",
            FILE_NAME_WITH_EXT.lock().unwrap().bold().bright_red(),
            self.line,
            ":".bold(),
            format!("{}..{}", self.span.0, self.span.1).bold()
        );

        println!(
            "{} {}\n",
            "ERROR:".bold().bright_red().underline(),
            self.msg.bold()
        );
    }

    fn draw_line(&mut self, line: &str) {
        self.buffer.push_str(line);
        self.buffer.push('\n');
    }

    fn draw_main_focus(&mut self, limit: usize) {
        for _ in 0..limit + 4 {
            self.drawer
                .push_str("^".bold().bright_red().to_string().as_str());
        }
    }

    #[inline]
    fn push_drawer(&mut self) {
        self.buffer.push_str(&self.drawer);
    }

    #[inline]
    fn reset_drawer(&mut self) {
        self.drawer.clear();
    }

    #[inline]
    fn reset_buffer(&mut self) {
        self.buffer.clear();
    }

    #[inline]
    fn add_space_to_line(&mut self) {
        self.buffer.push_str("  ");
    }

    #[inline]
    fn print_line(&mut self) {
        println!("{}", self.buffer);
    }
}
