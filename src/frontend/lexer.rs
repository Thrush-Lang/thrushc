use {
    super::super::{
        diagnostic::Diagnostic,
        error::{ThrushError, ThrushErrorKind},
        backend::compiler::options::ThrushFile,
    }, core::str, inkwell::{FloatPredicate, IntPredicate}, std::{num::ParseFloatError, path::PathBuf, process::exit}
};

pub struct Lexer<'a> {
    tokens: Vec<Token>,
    errors: Vec<ThrushError>,
    code: &'a [u8],
    start: usize,
    current: usize,
    line: usize,
    diagnostic: Diagnostic
}

impl<'a> Lexer<'a> {
    pub fn new(code: &'a [u8], file: &ThrushFile) -> Self {
        Self {
            tokens: Vec::new(),
            errors: Vec::new(),
            code,
            start: 0,
            current: 0,
            line: 1,
            diagnostic: Diagnostic::new(file)
        }
    }

    pub fn lex(&mut self) -> &[Token] {
        while !self.end() {
            self.start = self.current;

            match self.scan() {
                Ok(()) => {}
                Err(e) => self.errors.push(e),
            }
        }

        if !self.errors.is_empty() {
            self.errors.iter().for_each(|error| {
                self.diagnostic.report(error);
            });
       
            exit(1);
        };

        self.make(TokenKind::Eof);

        self.tokens.as_slice()
    }

    fn scan(&mut self) -> Result<(), ThrushError> {
        match self.advance() {
            b'[' => self.make(TokenKind::LeftBracket),
            b']' => self.make(TokenKind::RightBracket),
            b'(' => self.make(TokenKind::LParen),
            b')' => self.make(TokenKind::RParen),
            b'{' => self.make(TokenKind::LBrace),
            b'}' => self.make(TokenKind::RBrace),
            b',' => self.make(TokenKind::Comma),
            b'.' => self.make(TokenKind::Dot),
            b'%' => self.make(TokenKind::Arith),
            b'*' => self.make(TokenKind::Star),
            b'/' if self.char_match(b'/') => loop {
                if self.peek() == b'\n' || self.end() {
                    break;
                }

                self.advance();
            },
            b'/' if self.char_match(b'*') => loop {
                if self.char_match(b'*') && self.char_match(b'/') {
                    break;
                } else if self.end() {
                    return Err(ThrushError::Lex(
                        ThrushErrorKind::SyntaxError,
        
                        String::from("Syntax Error"),
                        String::from(
                            "Unterminated multiline comment. Did you forget to close the string with a '*/'?",
                        ),
                        self.line,
                    ));
                }

                self.advance();
            },
            b'/' => self.make(TokenKind::Slash),
            b';' => self.make(TokenKind::SemiColon),
            b'-' if self.char_match(b'-') => self.make(TokenKind::MinusMinus),
            b'-' => self.make(TokenKind::Minus),
            b'+' if self.char_match(b'+') => self.make(TokenKind::PlusPlus),
            b'+' => self.make(TokenKind::Plus),
            b':' if self.char_match(b':') => self.make(TokenKind::ColonColon),
            b':' => self.make(TokenKind::Colon),
            b'!' if self.char_match(b'=') => self.make(TokenKind::BangEq),
            b'!' => self.make(TokenKind::Bang),
            b'=' if self.char_match(b'=') => self.make(TokenKind::EqEq),
            b'=' => self.make(TokenKind::Eq),
            b'<' if self.char_match(b'=') => self.make(TokenKind::LessEq),
            b'<' => self.make(TokenKind::Less),
            b'>' if self.char_match(b'=') => self.make(TokenKind::GreaterEq),
            b'>' => self.make(TokenKind::Greater),
            b'|' if self.char_match(b'|') => self.make(TokenKind::Or),
            b'&' if self.char_match(b'&') => self.make(TokenKind::And),
            b' ' | b'\r' | b'\t' => {}
            b'\n' => self.line += 1,
            b'\'' => self.char()?,
            b'"' => self.string()?,
            b'0'..=b'9' => self.integer_or_float()?,
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => self.identifier()?,
            _ => {
                return Err(ThrushError::Lex(
                    ThrushErrorKind::UnknownChar,
                    String::from("Unknown character."),
                    String::from("Did you provide a valid character?"),
                    self.line,
                ));
            }
        }

        Ok(())
    }

    fn identifier(&mut self) -> Result<(), ThrushError> {

        while self.is_alpha(self.peek()) || self.peek().is_ascii_digit() {
            self.advance();
        }

        match str::from_utf8(&self.code[self.start..self.current]).unwrap() {
            "var" => self.make(TokenKind::Var),
            "fn" => self.make(TokenKind::Fn),
            "if" => self.make(TokenKind::If),
            "elif" => self.make(TokenKind::Elif),
            "else" => self.make(TokenKind::Else),
            "for" => self.make(TokenKind::For),
            "while" => self.make(TokenKind::While),
            "true" => self.make(TokenKind::True),
            "false" => self.make(TokenKind::False),
            "or" => self.make(TokenKind::Or),
            "and" => self.make(TokenKind::And),
            "const" => self.make(TokenKind::Const),
            "struct" => self.make(TokenKind::Struct),
            "return" => self.make(TokenKind::Return),
            "break" => self.make(TokenKind::Break),
            "continue" => self.make(TokenKind::Continue),
            "println" => self.make(TokenKind::Println),
            "print" => self.make(TokenKind::Print),
            "super" => self.make(TokenKind::Super),
            "this" => self.make(TokenKind::This),
            "extends" => self.make(TokenKind::Extends),
            "public" => self.make(TokenKind::Public),
            "null" => self.make(TokenKind::Null),

            "u8" => self.make(TokenKind::DataType(DataTypes::U8)),
            "u16" => self.make(TokenKind::DataType(DataTypes::U16)),
            "u32" => self.make(TokenKind::DataType(DataTypes::U32)),
            "u64" => self.make(TokenKind::DataType(DataTypes::U64)),

            "i8" => self.make(TokenKind::DataType(DataTypes::I8)),
            "i16" => self.make(TokenKind::DataType(DataTypes::I16)),
            "i32" => self.make(TokenKind::DataType(DataTypes::I32)),
            "i64" => self.make(TokenKind::DataType(DataTypes::I64)),

            "f32" => self.make(TokenKind::DataType(DataTypes::F32)),
            "f64" => self.make(TokenKind::DataType(DataTypes::F64)),

            "bool" => self.make(TokenKind::DataType(DataTypes::Bool)),

            "string" => self.make(TokenKind::DataType(DataTypes::String)),
            "char" => self.make(TokenKind::DataType(DataTypes::Char)),

            "void" => self.make(TokenKind::DataType(DataTypes::Void)),

            "float" => self.make(TokenKind::DataType(DataTypes::Float)),
            "integer" => self.make(TokenKind::DataType(DataTypes::Integer)),

            _ => {
                self.tokens.push(Token {
                    kind: TokenKind::Identifier,
                    lexeme: Some(self.lexeme()),
                    line: self.line,
                });
            }
        }

        Ok(())
    }

    fn integer_or_float(&mut self) -> Result<(), ThrushError> {
        while self.peek().is_ascii_digit()
            || self.peek() == b'_' && self.peek_next().is_ascii_digit()
            || self.peek() == b'.' && self.peek_next().is_ascii_digit()
        {
            self.advance();
        }

        let kind: DataTypes =
            self.eval_integer_type(self.lexeme())?;

        let num: Result<f64, ParseFloatError> = self.lexeme().parse::<f64>();

        if num.is_err() {
            return Err(ThrushError::Parse(
                ThrushErrorKind::ParsedNumber,
                String::from("The number is too big for an integer or float."),
                String::from(
                    "Did you provide a valid number with the correct format and not out of bounds?",
                ),
                self.line,
            ));
        }

        if kind == DataTypes::F32 || kind == DataTypes::F64 {
            self.tokens.push(Token {
                kind: TokenKind::Float(kind, *num.as_ref().unwrap()),
                lexeme: None,
                line: self.line
            });

            return Ok(());
        }

        self.tokens.push(Token {
            kind: TokenKind::Integer(kind, num.unwrap()),
            lexeme: None,
            line: self.line
        });

        Ok(())
    }

    fn char(&mut self) -> Result<(), ThrushError> {
        while self.peek() != b'\'' && !self.end() {
            self.advance();
        }

        if self.peek() != b'\'' {
            return Err(ThrushError::Lex(
                ThrushErrorKind::SyntaxError,
                String::from("Syntax Error"),
                String::from(
                    "Unterminated char. Did you forget to close the cjar with a '\''?",
                ),
                self.line,
            ));
        }

        self.advance();

        if self.code[self.start + 1..self.current - 1].len() > 1 {
            return Err(ThrushError::Lex(
                ThrushErrorKind::SyntaxError,
                String::from("Syntax Error"),
                String::from(
                    "A char data type only can contain one character.",
                ),
                self.line,
            ))
        }

        self.tokens.push(Token {
            kind: TokenKind::Char,
            lexeme: Some(String::from_utf8_lossy(&self.code[self.start + 1..self.current - 1]).to_string()),
            line: self.line,
        });

        Ok(())


    }

    fn string(&mut self) -> Result<(), ThrushError> {

        while self.peek() != b'"' && !self.end() {
            self.advance();
        }

        if self.peek() != b'"' {
            return Err(ThrushError::Lex(
                ThrushErrorKind::SyntaxError,
                String::from("Syntax Error"),
                String::from(
                    "Unterminated string. Did you forget to close the string with a '\"'?",
                ),
                self.line,
            ));
        }

        self.advance();

        let mut string: String =
            String::from_utf8_lossy(&self.code[self.start + 1..self.current - 1]).to_string();

        string.push('\0');

        string = string.replace("\\n", "\n");

        self.tokens.push(Token {
            kind: TokenKind::String,
            lexeme: Some(string),
            line: self.line,
        });

        Ok(())
    }

    pub fn eval_integer_type(
        &self,
        lexeme: String,
    ) -> Result<DataTypes, ThrushError> {

        if self.previous_token().kind == TokenKind::Minus && !lexeme.contains(".") && !self.tokens[self.tokens.len() - 2].kind.is_integer() {

            let lexeme: String = String::from("-") + &lexeme;

            return match lexeme.parse::<isize>() {
                Ok(num) => match num {
                    -128_isize..=127_isize => Ok(DataTypes::I8),
                    -32_768isize..=32_767_isize => Ok(DataTypes::I16),
                    -2147483648isize..=2147483647isize => Ok(DataTypes::I32),
                    -9223372036854775808isize..=9223372036854775807isize => Ok(DataTypes::I64),
                    _ => Err(ThrushError::Parse(
                        ThrushErrorKind::UnreachableNumber,
                        String::from("The number is out of bounds."),
                        String::from("The size is out of bounds of an isize (-n to n)."),
                        self.line,
                    )),
                },
                Err(_) => Err(ThrushError::Parse(
                    ThrushErrorKind::ParsedNumber,
                    String::from("The number is too long for an signed integer or float."),
                    String::from("Did you provide a valid number with the correct format and not out of bounds?"),
                    self.line,
                )),
            };
        } else if lexeme.contains(".") {

            if lexeme.chars().filter(|ch| *ch == '.').count() > 1 {
                return Err(ThrushError::Lex(
                    ThrushErrorKind::SyntaxError, 
                    String::from("Float Violated Syntax"), 
                    String::from("Float's values should be only contain one dot."), 
                    self.line
                ));
            } else if lexeme.parse::<f32>().is_ok() {
                return Ok(DataTypes::F32);
            } else if lexeme.parse::<f64>().is_ok() {
                return Ok(DataTypes::F64);
            } 

            return Err(ThrushError::Parse(
                ThrushErrorKind::ParsedNumber,
                String::from("The number is too big for an float."),
                String::from("Did you provide a valid number with the correct format and not out of bounds?"),
                self.line,
            ));
            
        }

        match lexeme.parse::<isize>() {
            Ok(num) => match num {
                0isize..=127isize => Ok(DataTypes::U8),
                128isize..=65_535isize => Ok(DataTypes::U16),
                65_536isize..=4_294_967_295isize => Ok(DataTypes::U32),
                4_294_967_296isize..= 9_223_372_036_854_775_807isize => Ok(DataTypes::U64),
                _ => Err(ThrushError::Parse(
                    ThrushErrorKind::UnreachableNumber,
                    String::from("The number is out of bounds."),
                    String::from("The size is out of bounds of an isize (0 to n)."),
                    self.line,
                )),
            },
            Err(_) => Err(ThrushError::Parse(
                ThrushErrorKind::ParsedNumber,
                String::from("The number is too long for an unsigned integer_or_float."),
                String::from(
                    "Did you provide a valid number with the correct format and not out of bounds?",
                ),
                self.line,
            )),
        }
    }

    fn previous_token(&self) -> &Token {
        &self.tokens[self.tokens.len() - 1]
    }

    fn advance(&mut self) -> u8 {
        let c: u8 = self.code[self.current];
        self.current += 1;

        c
    }

    fn peek_next(&self) -> u8 {
        if self.current + 1 >= self.code.len() {
            return b'\0';
        }

        self.code[self.current + 1]
    }

    fn peek(&self) -> u8 {
        if self.end() {
            return b'\0';
        }

        self.code[self.current]
    }

    fn char_match(&mut self, c: u8) -> bool {
        if !self.end() && self.code[self.current] == c {
            self.current += 1;
            return true;
        }

        false
    }

    #[inline]
    fn end(&self) -> bool {
        self.current >= self.code.len()
    }

    #[inline]
    fn is_alpha(&self, ch: u8) -> bool {
        ch.is_ascii_lowercase() || ch.is_ascii_uppercase() || ch == b'_'
    }

    #[inline]
    fn lexeme(&self) -> String {
        String::from_utf8_lossy(&self.code[self.start..self.current]).to_string()
    }

    fn make(&mut self, kind: TokenKind) {
        self.tokens.push(Token {
            kind,
            lexeme: Some(self.lexeme()),
            line: self.line,
        });
    }
}

#[derive(Debug, Clone)]
pub struct Token {
    pub lexeme: Option<String>,
    pub kind: TokenKind,
    pub line: usize,
}

#[derive(Debug, PartialEq, Clone)]
pub enum TokenKind {
    // --- Operators ---
    LParen,       // ' ( '
    RParen,       // ' ) '
    LBrace,       // ' { '
    RBrace,       // ' } '
    Comma,        // ' , '
    Dot,          // ' . '
    Minus,        // ' - '
    Plus,         // ' + '
    Slash,        // ' / '
    Star,         // ' * '
    Colon,        // ' : '
    SemiColon,    // ' ; '
    LeftBracket,  // ' ] '
    RightBracket, // ' [ '
    Arith,        // ' % ',
    Bang,         // ' ! '
    ColonColon,   // ' :: '
    BangEq,    // ' != '
    Eq,           // ' = '
    EqEq,         // ' == '
    Greater,      // ' > '
    GreaterEq, // ' >= '
    Less,         // ' < '
    LessEq,    // ' <= '
    PlusPlus,     // ' ++ '
    MinusMinus,   // ' -- '

    // --- Literals ---
    Identifier,
    Integer(DataTypes, f64),
    Float(DataTypes, f64),
    DataType(DataTypes),
    String,
    Char,

    // --- Keywords ---
    Public,
    And,
    Struct,
    Else,
    False,
    Fn,
    For,
    Continue,
    Break,
    If,
    Elif,
    Null,
    Or,
    Println,
    Print,
    Return,
    Super,
    This,
    True,
    Var,
    Const,
    While,
    Extends,

    Eof,
}

impl std::fmt::Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenKind::LParen => write!(f, "("),
            TokenKind::RParen => write!(f, ")"),
            TokenKind::LBrace => write!(f, "{{"),
            TokenKind::RBrace => write!(f, "}}"),
            TokenKind::Comma => write!(f, ","),
            TokenKind::Dot => write!(f, "."),
            TokenKind::Minus => write!(f, "-"),
            TokenKind::Plus => write!(f, "+"),
            TokenKind::Slash => write!(f, "/"),
            TokenKind::Star => write!(f, "*"),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::SemiColon => write!(f, ";"),
            TokenKind::LeftBracket => write!(f, "["),
            TokenKind::RightBracket => write!(f, "]"),
            TokenKind::Arith => write!(f, "%"),
            TokenKind::Bang => write!(f, "!"),
            TokenKind::ColonColon => write!(f, "::"),
            TokenKind::BangEq => write!(f, "!="),
            TokenKind::Eq => write!(f, "="),
            TokenKind::EqEq => write!(f, "=="),
            TokenKind::Greater => write!(f, ">"),
            TokenKind::GreaterEq => write!(f, ">="),
            TokenKind::Less => write!(f, "<"),
            TokenKind::LessEq => write!(f, "<="),
            TokenKind::PlusPlus => write!(f, "++"),
            TokenKind::MinusMinus => write!(f, "--"),
            TokenKind::Identifier => write!(f, "Identifier"),
            TokenKind::And => write!(f, "and"),
            TokenKind::Struct => write!(f, "struct"),
            TokenKind::Else => write!(f, "else"),
            TokenKind::False => write!(f, "false"),
            TokenKind::Fn => write!(f, "fn"),
            TokenKind::For => write!(f, "for"),
            TokenKind::Continue => write!(f, "continue"),
            TokenKind::Break => write!(f, "break"),
            TokenKind::If => write!(f, "if"),
            TokenKind::Elif => write!(f, "elif"),
            TokenKind::Public => write!(f, "public"),
            TokenKind::Null => write!(f, "null"),
            TokenKind::Or => write!(f, "or"),
            TokenKind::Println => write!(f, "println"),
            TokenKind::Print => write!(f, "print"),
            TokenKind::Return => write!(f, "return"),
            TokenKind::Super => write!(f, "super"),
            TokenKind::This => write!(f, "this"),
            TokenKind::True => write!(f, "true"),
            TokenKind::Var => write!(f, "var"),
            TokenKind::Const => write!(f, "const"),
            TokenKind::While => write!(f, "while"),
            TokenKind::Extends => write!(f, "extends"),
            TokenKind::Integer(_, _) => write!(f, "integer"),
            TokenKind::Float(_, _) => write!(f, "float"),
            TokenKind::String => write!(f, "string"),
            TokenKind::Char => write!(f, "char"),
            TokenKind::Eof => write!(f, "EOF"),
            TokenKind::DataType(datatype) => write!(f, "{}", datatype),
        }
    }
}

impl TokenKind {
    #[inline]
    pub fn is_integer(&self) -> bool {
        matches!(self, TokenKind::Integer(_, _))
    }

    #[inline]
    pub fn to_llvm_intrinsic_identifier(&self) -> &str {
        match self {
            TokenKind::Plus => "add",
            TokenKind::Minus => "sub",
            TokenKind::Star => "mul",
            _ => "",
        }
    }

    #[inline]
    pub fn to_int_predicate(&self) -> IntPredicate {
        match self {
            TokenKind::EqEq => IntPredicate::EQ,
            TokenKind::BangEq => IntPredicate::NE,
            TokenKind::Greater => IntPredicate::SGT,
            TokenKind::GreaterEq => IntPredicate::SGE,
            TokenKind::Less => IntPredicate::SLT,
            TokenKind::LessEq => IntPredicate::SLE,
            _ => unreachable!(),
        }
    }

    #[inline]
    pub fn to_float_predicate(&self) -> FloatPredicate {
        match self {
            TokenKind::EqEq => FloatPredicate::OEQ,
            TokenKind::BangEq => FloatPredicate::ONE,
            TokenKind::Greater => FloatPredicate::OGT,
            TokenKind::GreaterEq => FloatPredicate::OGE,
            TokenKind::Less => FloatPredicate::OLT,
            TokenKind::LessEq => FloatPredicate::OLE,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum DataTypes {
    // Integer DataTypes
    U8 = 0,
    U16 = 1,
    U32 = 2,
    U64 = 3,
    I8 = 4,
    I16 = 5,
    I32 = 6,
    I64 = 7,
    Integer = 8,

    // Floating Point DataTypes
    F32 = 9,
    F64 = 10,
    Float = 11,
    // Boolean DataTypes
    Bool = 12,

    // String DataTypes
    String = 13,
    Char = 14,

    // Void Type
    Void = 15,
}


impl std::fmt::Display for DataTypes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {

        match self {
            DataTypes::U8 => write!(f, "u8"),
            DataTypes::U16 => write!(f, "u16"),
            DataTypes::U32 => write!(f, "u32"),
            DataTypes::U64 => write!(f, "u64"),
            DataTypes::I8 => write!(f, "i8"),
            DataTypes::I16 => write!(f, "i16"),
            DataTypes::I32 => write!(f, "i32"),
            DataTypes::I64 => write!(f, "i64"),
            DataTypes::F32 => write!(f, "f32"),
            DataTypes::F64 => write!(f, "f64"),
            DataTypes::Bool => write!(f, "bool"),
            DataTypes::String => write!(f, "string"),
            DataTypes::Char => write!(f, "char"),
            DataTypes::Void => write!(f, "void"),
            DataTypes::Float => write!(f, "float"),
            DataTypes::Integer => write!(f, "integer")
        }
    }
}

impl DataTypes {

    pub fn need_cast(&self, value: &DataTypes) -> bool {
        match self {
            DataTypes::U8
                if value == &DataTypes::U16
                    || value == &DataTypes::U32
                    || value == &DataTypes::U64 =>
            {
                true
            }
            DataTypes::U16 if value == &DataTypes::U32 || value == &DataTypes::U64 => true,
            DataTypes::U32 if value == &DataTypes::U64 => true,

            DataTypes::I8
                if value == &DataTypes::I16
                    || value == &DataTypes::I32
                    || value == &DataTypes::I64 =>
            {
                true
            }
            DataTypes::I16 if value == &DataTypes::I32 || value == &DataTypes::I64 => true,
            DataTypes::I32 if value == &DataTypes::I64 => true,

            DataTypes::F32 if value == &DataTypes::F64 => true,
            DataTypes::F64 if value == &DataTypes::F32 => true,

            _ => false, 
        }
    }

    pub fn is_unreachable_cast(&self, value: &DataTypes) -> bool {
        matches!((self, value), (DataTypes::U8, DataTypes::I8 | DataTypes::I16 | DataTypes::I32 | DataTypes::I64 | DataTypes::U16 | DataTypes::U32 | DataTypes::U64)
        | (DataTypes::U16, DataTypes::I8 | DataTypes::I16 | DataTypes::I32 | DataTypes::I64 | DataTypes::U32 | DataTypes::U64)
        | (DataTypes::U32, DataTypes::I8 | DataTypes::I16 | DataTypes::I32 | DataTypes::I64 | DataTypes::U64)
        | (DataTypes::U64, DataTypes::I8 | DataTypes::I16 | DataTypes::I32 | DataTypes::I64)
        | (DataTypes::I8, DataTypes::U8 | DataTypes::U16 | DataTypes::U32 | DataTypes::U64 | DataTypes::I16 | DataTypes::I32 | DataTypes::I64)
        | (DataTypes::I16, DataTypes::U8 | DataTypes::U16 | DataTypes::U32 | DataTypes::U64 | DataTypes::I32 | DataTypes::I64)
        | (DataTypes::I32, DataTypes::U8 | DataTypes::U16 | DataTypes::U32 | DataTypes::U64 | DataTypes::I64)
        | (DataTypes::I64, DataTypes::U8 | DataTypes::U16 | DataTypes::U32 | DataTypes::U64
        ))
    }

    pub fn check(&self, value: &DataTypes) -> bool {
        self == value
            || matches!(
                (&self, value),
                (
                    DataTypes::U64 | DataTypes::U32 | DataTypes::U16 | DataTypes::U8,
                    DataTypes::U8 | DataTypes::U16 | DataTypes::U32 | DataTypes::U64
                ) | (
                    DataTypes::I64 | DataTypes::I32 | DataTypes::I16 | DataTypes::I8,
                    DataTypes::I8 | DataTypes::I16 | DataTypes::I32 | DataTypes::I64
                ) | (DataTypes::F64, DataTypes::F32)
                    | (DataTypes::F32, DataTypes::F64)
            )
    }

    pub fn as_llvm_identifier(&self) -> &str {
        match self {
            DataTypes::U8 | DataTypes::I8 => "i8",
            DataTypes::U16 | DataTypes::I16 => "i16",
            DataTypes::U32 | DataTypes::I32 => "i32",
            DataTypes::U64 | DataTypes::I64 => "i64",
            DataTypes::F32 => "f32",
            DataTypes::F64 => "f64",
            _ => unreachable!()
        }
    }
}
