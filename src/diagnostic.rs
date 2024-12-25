use {
    super::backend::compiler::options::ThrushFile,
    super::error::{ThrushError, ThrushErrorKind},
    std::{
        fs::File,
        io::{BufRead, BufReader},
    },
    stylic::{style, Stylize},
};

#[derive(Debug)]
pub struct Diagnostic {
    pub file_name: String,
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
            buffer: String::new(),
            drawer: String::new(),
            lines,
            file_name: thrush_file.name.clone(),
        }
    }

    pub fn report(&mut self, error: &ThrushError) {
        if let ThrushError::Parse(
            ThrushErrorKind::ParsedNumber
            | ThrushErrorKind::UnreachableNumber
            | ThrushErrorKind::SyntaxError
            | ThrushErrorKind::UnreachableVariable
            | ThrushErrorKind::VariableNotDefined
            | ThrushErrorKind::VariableNotDeclared,
            title,
            help,
            line,
        ) = error
        {
            self.print_report(title, help, *line);
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
            self.print_report(title, help, *line);
        } else if let ThrushError::Scope(
            ThrushErrorKind::UnreachableVariable | ThrushErrorKind::VariableNotDefined,
            title,
            help,
            line,
        ) = error
        {
            self.print_report(title, help, *line);
        }
    }

    pub fn draw_only_line(&mut self, line: usize) -> &str {
        let content: &str = if line > self.lines.len() - 1 {
            self.lines.last().unwrap().trim()
        } else {
            self.lines[line - 1].trim()
        };

        self.drawer.push_str(&format!("{} | ", line));
        self.buffer.push_str(&format!("{}\n", content));

        for _ in 0..content.len() + 10 {
            self.drawer
                .push_str(style("─").bright_red().to_string().as_str());
        }

        self.buffer.push_str(&self.drawer);

        &self.buffer
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.drawer.clear();
    }

    fn print_report(&mut self, title: &str, help: &str, line: usize) {
        self.print_header(line, title);

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

    fn print_header(&mut self, line: usize, title: &str) {
        println!(
            "{} - {}\n",
            format_args!("{}", style(&self.file_name).bold().bright_red()),
            line
        );

        println!(
            "{} {}\n",
            style("ERROR").bold().underline().bright_red(),
            title
        );
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
