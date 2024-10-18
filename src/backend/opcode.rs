#[derive(Debug, Clone)]
pub enum OpCode {
    Sub, // -
    Add, // +
    Div, // /
    Mul, // *

    And,       // &&
    Or,        // ||
    Greater,   // >
    GreaterEq, // >=
    Less,      // <
    LessEq,    // <=
    BangEq,    // !=
    EqEq,      // ==
}
