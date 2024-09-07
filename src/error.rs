use super::frontend::lexer::TokenSpan;

#[derive(Default, Debug)]
pub enum ThrushError {
    Compile(String),
    Parse(ThrushErrorKind, String, String, String, TokenSpan, usize),
    Lex(ThrushErrorKind, String, String, String, TokenSpan, usize),
    #[default]
    None,
}

#[derive(Debug)]
pub enum ThrushErrorKind {
    TooManyArguments,
    SyntaxError,
    UnreachableNumber,
    ParsedNumber,
    UnknownChar,
}
