#[derive(Default, Debug)]
pub enum ThrushError {
    Compile(String),
    Parse(ThrushErrorKind, String, String, usize),
    Lex(ThrushErrorKind, String, String, usize),
    Scope(ThrushErrorKind, String, String, usize),
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
    UnreachableVariable,
    VariableNotDefined,
    VariableNotDeclared,
}
