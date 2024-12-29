use {
    super::backend::compiler::options::ThrushFile,
    super::{
        error::{ThrushError, ThrushErrorKind},
        logging::LogType,
    },
    std::{
        fs::File,
        io::{BufRead, BufReader},
    },
    stylic::{style, Stylize},
};

#[derive(Debug)]
pub struct Diagnostic {
    pub thrush_file: ThrushFile,
    buffer: String,
    drawer: String,
    lines: Vec<String>,
}

impl Diagnostic {
    pub fn new(thrush_file: &ThrushFile) -> Self {
        let file: File = File::open(&thrush_file.path).unwrap();
        let lines: Vec<String> = BufReader::new(file)
            .lines()
            .map(|line| line.unwrap().to_string())
            .collect();

        Self {
            thrush_file: thrush_file.clone(),
            buffer: String::new(),
            drawer: String::new(),
            lines,
        }
    }

    pub fn report(&mut self, error: &ThrushError, log_type: LogType) {
        if let ThrushError::Parse(
            ThrushErrorKind::ParsedNumber
            | ThrushErrorKind::UnreachableNumber
            | ThrushErrorKind::SyntaxError
            | ThrushErrorKind::UnreachableVariable
            | ThrushErrorKind::ObjectNotDefined
            | ThrushErrorKind::VariableNotDeclared,
            title,
            help,
            line,
        ) = error
        {
            self.print_report(title, help, *line, log_type);
        } else if let ThrushError::Lex(
            ThrushErrorKind::SyntaxError
            | ThrushErrorKind::ParsedNumber
            | ThrushErrorKind::UnreachableNumber
            | ThrushErrorKind::UnknownChar,
            title,
            help,
            line,
        ) = error
        {
            self.print_report(title, help, *line, log_type);
        } else if let ThrushError::Scope(
            ThrushErrorKind::UnreachableVariable | ThrushErrorKind::ObjectNotDefined,
            title,
            help,
            line,
        ) = error
        {
            self.print_report(title, help, *line, log_type);
        }
    }

    fn print_report(&mut self, title: &str, help: &str, line: usize, log_type: LogType) {
        self.print_header(line, title, log_type);

        let content: &str = if line > self.lines.len() - 1 {
            self.lines.last().unwrap().trim()
        } else {
            self.lines[line - 1].trim()
        };

        self.buffer.push_str("  ");
        self.drawer.push_str(&format!("{} | ^ ", line));
        self.buffer.push_str(&format!("{}\n", content));

        println!("|\n|");

        for _ in 0..content.len() + 6 {
            self.drawer
                .push_str(style("─").bright_red().to_string().as_str());
        }

        self.buffer.push_str(&self.drawer);

        println!("{}", self.buffer);

        self.drawer.clear();
        self.buffer.clear();

        println!(
            "\n{}{} {}\n",
            style("Help").bold().bright_green(),
            style(":").bold(),
            style(help).bold()
        );
    }

    fn print_header(&mut self, line: usize, title: &str, log_type: LogType) {
        println!(
            "{} - {}\n",
            format_args!("{}", style(&self.thrush_file.name).bold().bright_red()),
            line
        );

        println!("{} {}\n", log_type.to_styled(), title);
    }
}

#[inline]
pub fn create_panic_message(subject: &str) -> String {
    format!(
        "{} {} {}",
        style("PANIC").bold().bright_red(),
        style("-").bold(),
        subject
    )
}

#[inline]
pub fn create_help_message(msg: &str) -> String {
    format!(
        "● {}{} {}",
        style("Help").bold().bright_green(),
        style(":").bold(),
        msg
    )
}
