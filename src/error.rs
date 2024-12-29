#[derive(Default, Debug)]
pub enum ThrushError {
    Parse(ThrushErrorKind, String, String, usize),
    Lex(ThrushErrorKind, String, String, usize),
    Scope(ThrushErrorKind, String, String, usize),
    #[default]
    None,
}

#[derive(Debug)]
pub enum ThrushErrorKind {
    SyntaxError,
    UnreachableNumber,
    ParsedNumber,
    UnknownChar,
    UnreachableVariable,
    ObjectNotDefined,
    VariableNotDefined,
    VariableNotDeclared,
}
