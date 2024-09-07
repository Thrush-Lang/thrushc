use {
    super::super::{
        diagnostic::Diagnostic,
        error::{ThrushError, ThrushErrorKind},
    },
    core::str,
    std::{mem, num::ParseFloatError},
};

pub type TokenSpan = (usize, usize);

pub struct Lexer<'a> {
    pub tokens: Vec<Token>,
    pub code: &'a [u8],
    pub token_start: usize,
    pub token_end: usize,
    pub start: usize,
    pub current: usize,
    pub line: usize,
    pub errors: Vec<ThrushError>,
}

impl<'a> Lexer<'a> {
    pub fn new(code: &'a [u8]) -> Self {
        Self {
            tokens: Vec::new(),
            code,
            token_start: 0,
            token_end: 0,
            start: 0,
            current: 0,
            line: 1,
            errors: Vec::with_capacity(10),
        }
    }

    pub fn lex(&mut self) -> Result<&[Token], String> {
        while !self.end() {
            if self.errors.len() >= 10 {
                break;
            }

            self.start = self.current;

            match self.scan() {
                Ok(_) => {}
                Err(e) => self.errors.push(e),
            }
        }

        if !self.errors.is_empty() {
            for error in mem::take(&mut self.errors) {
                Diagnostic::new(error).report();
            }

            return Err(String::from("Compilation proccess ended with errors."));
        };

        self.make(TokenKind::Eof);

        Ok(self.tokens.as_slice())
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
                        self.lexeme(),
                        String::from("Syntax Error"),
                        String::from(
                            "Unterminated multiline comment. Did you forget to close the string with a '*/'?",
                        ),
                        (self.token_start, self.token_end),
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
            b'!' if self.char_match(b'=') => self.make(TokenKind::BangEqual),
            b'!' => self.make(TokenKind::Bang),
            b'=' if self.char_match(b'=') => self.make(TokenKind::EqEq),
            b'=' => self.make(TokenKind::Eq),
            b'<' if self.char_match(b'=') => self.make(TokenKind::LessEqual),
            b'<' => self.make(TokenKind::Less),
            b'>' if self.char_match(b'=') => self.make(TokenKind::GreaterEqual),
            b'>' => self.make(TokenKind::Greater),
            b'|' if self.char_match(b'|') => self.make(TokenKind::Or),
            b'&' if self.char_match(b'&') => self.make(TokenKind::And),
            b' ' | b'\r' | b'\t' => {}
            b'\n' => self.line += 1,
            b'"' => self.string()?,
            b'0'..=b'9' => self.integer()?,
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => self.identifier()?,
            _ => {
                return Err(ThrushError::Lex(
                    ThrushErrorKind::UnknownChar,
                    self.lexeme(),
                    String::from("Unknown character."),
                    String::from("Did you provide a valid character?"),
                    (self.token_start, self.token_end),
                    self.line,
                ));
            }
        }

        Ok(())
    }

    fn identifier(&mut self) -> Result<(), ThrushError> {
        self.begin_token();

        while self.is_alpha(self.peek()) || self.peek().is_ascii_digit() {
            self.advance();
        }

        match str::from_utf8(&self.code[self.start..self.current]).unwrap() {
            "let" => self.make(TokenKind::Let),
            "def" => self.make(TokenKind::Def),
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
            "print" => self.make(TokenKind::Print),
            "println" => self.make(TokenKind::Println),
            "puts" => self.make(TokenKind::Puts),
            "super" => self.make(TokenKind::Super),
            "this" => self.make(TokenKind::This),
            "extends" => self.make(TokenKind::Extends),
            "pub" => self.make(TokenKind::Pub),
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

            "void" => self.make(TokenKind::DataType(DataTypes::Void)),

            _ => {
                self.tokens.push(Token {
                    kind: TokenKind::Identifier,
                    lexeme: Some(self.lexeme()),
                    line: self.line,
                    span: (self.token_start, self.token_end),
                });
            }
        }

        self.end_token();

        Ok(())
    }

    fn integer(&mut self) -> Result<(), ThrushError> {
        self.begin_token();

        while self.peek().is_ascii_digit()
            || self.peek() == b'_' && self.peek_next().is_ascii_digit()
            || self.peek() == b'.' && self.peek_next().is_ascii_digit()
        {
            self.advance();
        }

        self.end_token();

        let kind: DataTypes =
            self.eval_integer_type(self.lexeme(), (self.token_start, self.token_end), self.line)?;

        let num: Result<f64, ParseFloatError> = self.lexeme().parse::<f64>();

        if num.is_err() {
            return Err(ThrushError::Parse(
                ThrushErrorKind::ParsedNumber,
                self.lexeme(),
                String::from("The number is too big for an integer."),
                String::from(
                    "Did you provide a valid number with the correct format and not out of bounds?",
                ),
                (self.token_start, self.token_end),
                self.line,
            ));
        }

        self.tokens.push(Token {
            kind: TokenKind::Integer(kind, num.unwrap()),
            lexeme: None,
            line: self.line,
            span: (self.token_start, self.token_end),
        });

        Ok(())
    }

    fn string(&mut self) -> Result<(), ThrushError> {
        self.begin_token();

        while self.peek() != b'"' && !self.end() {
            self.advance();
        }

        if self.peek() != b'"' {
            return Err(ThrushError::Lex(
                ThrushErrorKind::SyntaxError,
                self.lexeme(),
                String::from("Syntax Error"),
                String::from(
                    "Unterminated string. Did you forget to close the string with a '\"'?",
                ),
                (self.token_start, self.token_end),
                self.line,
            ));
        }

        self.advance();
        self.end_token();

        let mut string: String =
            String::from_utf8_lossy(&self.code[self.start + 1..self.current - 1]).to_string();

        string.push_str("\0A\x00");

        string = string.replace("\\n", "\n");

        self.tokens.push(Token {
            kind: TokenKind::String,
            lexeme: Some(string),
            line: self.line,
            span: (self.token_start, self.token_end),
        });

        Ok(())
    }

    pub fn eval_integer_type(
        &self,
        lexeme: String,
        span: TokenSpan,
        line: usize,
    ) -> Result<DataTypes, ThrushError> {
        if self.previous_token().kind == TokenKind::Minus && !lexeme.contains(".") {
            let lexeme: String = String::from("-") + &lexeme;

            return match lexeme.parse::<isize>() {
                Ok(num) => match num {
                    -128_isize..=127_isize => Ok(DataTypes::I8),
                    -32_768isize..=32_767_isize => Ok(DataTypes::I16),
                    -2147483648isize..=2147483647isize => Ok(DataTypes::I32),
                    -9223372036854775808isize..=9223372036854775807isize => Ok(DataTypes::I64),
                    _ => Err(ThrushError::Parse(
                        ThrushErrorKind::UnreachableNumber,
                        lexeme,
                        String::from("The number is out of bounds."),
                        String::from("The size is out of bounds of an isize (-n to n)."),
                        span,
                        line,
                    )),
                },
                Err(_) => Err(ThrushError::Parse(
                    ThrushErrorKind::ParsedNumber,
                    lexeme,
                    String::from("The number is too long for an signed integer."),
                    String::from("Did you provide a valid number with the correct format and not out of bounds?"),
                    span,
                    line,
                )),
            };
        } else if lexeme.contains(".") {
            return match lexeme.parse::<f64>() {
                Ok(_) => Ok(DataTypes::F64),
                Err(_) => Err(ThrushError::Parse(
                    ThrushErrorKind::ParsedNumber,
                    lexeme,
                    String::from("The number is too big for an float."),
                    String::from("Did you provide a valid number with the correct format and not out of bounds?"),
                    span,
                    line,
                )),
            };
        }

        match lexeme.parse::<usize>() {
            Ok(num) => match num {
                0usize..=255usize => Ok(DataTypes::U8),
                0usize..=65_535usize => Ok(DataTypes::U16),
                0usize..=4_294_967_295usize => Ok(DataTypes::U32),
                0usize..=18_446_744_073_709_551_615usize => Ok(DataTypes::U64),
                _ => Err(ThrushError::Parse(
                    ThrushErrorKind::UnreachableNumber,
                    lexeme,
                    String::from("The number is out of bounds."),
                    String::from("The size is out of bounds of an usize (0 to n)."),
                    span,
                    line,
                )),
            },
            Err(_) => Err(ThrushError::Parse(
                ThrushErrorKind::ParsedNumber,
                lexeme,
                String::from("The number is too long for an unsigned integer."),
                String::from(
                    "Did you provide a valid number with the correct format and not out of bounds?",
                ),
                span,
                line,
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

        *self.code.get(self.current + 1).unwrap()
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

    fn end(&self) -> bool {
        self.current >= self.code.len()
    }

    fn begin_token(&mut self) {
        self.token_start = self.current;
    }

    fn end_token(&mut self) {
        self.token_end = self.current;
    }

    fn is_alpha(&self, ch: u8) -> bool {
        ch.is_ascii_lowercase() || ch.is_ascii_uppercase() || ch == b'_'
    }

    fn lexeme(&self) -> String {
        String::from_utf8_lossy(&self.code[self.start..self.current]).to_string()
    }

    fn make(&mut self, kind: TokenKind) {
        self.begin_token();
        self.end_token();

        self.tokens.push(Token {
            kind,
            lexeme: Some(self.lexeme()),
            line: self.line,
            span: (self.token_start, self.token_end),
        });
    }
}

#[derive(Debug, Clone)]
pub struct Token {
    pub lexeme: Option<String>,
    pub kind: TokenKind,
    pub span: TokenSpan,
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
    BangEqual,    // ' != '
    Eq,           // ' = '
    EqEq,         // ' == '
    Greater,      // ' > '
    GreaterEqual, // ' >= '
    Less,         // ' < '
    LessEqual,    // ' <= '
    PlusPlus,     // ' ++ '
    MinusMinus,   // ' -- '

    // --- Literals ---
    Identifier,
    Integer(DataTypes, f64),
    DataType(DataTypes),
    String,

    // --- Keywords ---
    Pub,
    And,
    Struct,
    Else,
    False,
    Def,
    For,
    Continue,
    Break,
    If,
    Elif,
    Null,
    Or,
    Print,
    Println,
    Puts,
    Return,
    Super,
    This,
    True,
    Let,
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
            TokenKind::BangEqual => write!(f, "!="),
            TokenKind::Eq => write!(f, "="),
            TokenKind::EqEq => write!(f, "=="),
            TokenKind::Greater => write!(f, ">"),
            TokenKind::GreaterEqual => write!(f, ">="),
            TokenKind::Less => write!(f, "<"),
            TokenKind::LessEqual => write!(f, "<="),
            TokenKind::PlusPlus => write!(f, "++"),
            TokenKind::MinusMinus => write!(f, "--"),
            TokenKind::Identifier => write!(f, "Identifier"),
            TokenKind::And => write!(f, "and"),
            TokenKind::Struct => write!(f, "struct"),
            TokenKind::Else => write!(f, "else"),
            TokenKind::False => write!(f, "false"),
            TokenKind::Def => write!(f, "def"),
            TokenKind::For => write!(f, "for"),
            TokenKind::Continue => write!(f, "continue"),
            TokenKind::Break => write!(f, "break"),
            TokenKind::If => write!(f, "if"),
            TokenKind::Elif => write!(f, "elif"),
            TokenKind::Pub => write!(f, "pub"),
            TokenKind::Null => write!(f, "null"),
            TokenKind::Or => write!(f, "or"),
            TokenKind::Print => write!(f, "print"),
            TokenKind::Println => write!(f, "println"),
            TokenKind::Puts => write!(f, "puts"),
            TokenKind::Return => write!(f, "return"),
            TokenKind::Super => write!(f, "super"),
            TokenKind::This => write!(f, "this"),
            TokenKind::True => write!(f, "true"),
            TokenKind::Let => write!(f, "let"),
            TokenKind::Const => write!(f, "const"),
            TokenKind::While => write!(f, "while"),
            TokenKind::Extends => write!(f, "extends"),
            TokenKind::Integer(_, _) => write!(f, "Integer"),
            TokenKind::String => write!(f, "String"),
            TokenKind::Eof => write!(f, "EOF"),
            TokenKind::DataType(datatype) => write!(f, "{}", datatype),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum DataTypes {
    // Integer DataTypes
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,

    // Floating Point DataTypes
    F32,
    F64,

    // Boolean DataTypes
    Bool,

    // String DataTypes
    String,

    // Void Type
    Void,
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
            DataTypes::Void => write!(f, "void"),
        }
    }
}
