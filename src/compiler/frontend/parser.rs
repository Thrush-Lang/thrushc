use {
    super::{
        super::{ThrushError, ThrushErrorKind, ThrushFile},
        diagnostic::Diagnostic,
        lexer::{DataTypes, Token, TokenKind},
    },
    inkwell::values::PointerValue,
    std::mem,
};

#[derive(Debug, Clone)]
pub enum Instruction<'instr> {
    Puts(Box<Instruction<'instr>>),
    Println(Vec<Instruction<'instr>>),
    String(String),
    Integer(DataTypes, f64),
    Block(Vec<Instruction<'instr>>),
    EntryPoint {
        body: Box<Instruction<'instr>>,
    },
    PointerValue(PointerValue<'instr>),
    Param(String, DataTypes),
    Function {
        name: String,
        params: Vec<Instruction<'instr>>,
        body: Box<Instruction<'instr>>,
        return_kind: Option<DataTypes>,
        is_public: bool,
    },
    Return(Box<Instruction<'instr>>),
    Boolean(bool),
    Null,
    End,
}

pub struct Parser<'parser, 'instr> {
    stmts: Vec<Instruction<'instr>>,
    errors: Vec<ThrushError>,
    tokens: &'parser [Token],
    fun: u16,
    ret: Option<DataTypes>,
    current: usize,
    scope: usize,
    file: ThrushFile,
}

impl<'parser, 'instr> Parser<'parser, 'instr> {
    pub fn new(tokens: &'parser [Token], file: ThrushFile) -> Self {
        Self {
            stmts: Vec::new(),
            errors: Vec::with_capacity(10),
            tokens,
            current: 0,
            ret: None,
            fun: 0,
            scope: 0,
            file,
        }
    }

    pub fn start(&mut self) -> Result<&[Instruction<'instr>], ThrushError> {
        while !self.end() {
            match self.parse() {
                Ok(instr) => {
                    self.stmts.push(instr);
                }
                Err(e) => {
                    if self.errors.len() >= 10 {
                        break;
                    }

                    self.errors.push(e)
                }
            }
        }

        if !self.errors.is_empty() {
            for error in mem::take(&mut self.errors) {
                Diagnostic::new(error).report();
            }

            return Err(ThrushError::Compile(String::from(
                "Compilation proccess ended with errors.",
            )));
        }

        self.stmts.push(Instruction::End);

        Ok(self.stmts.as_slice())
    }

    fn parse(&mut self) -> Result<Instruction<'instr>, ThrushError> {
        match &self.peek().kind {
            TokenKind::Puts => Ok(self.puts()?),
            TokenKind::Println => Ok(self.println()?),
            TokenKind::Def => Ok(self.def(false)?),
            TokenKind::LBrace => Ok(self.block()?),
            TokenKind::Return => Ok(self.ret()?),
            TokenKind::Pub => Ok(self.pub_def()?),
            _ => Ok(self.expr()?),
        }
    }

    fn pub_def(&mut self) -> Result<Instruction<'instr>, ThrushError> {
        self.advance();

        match &self.peek().kind {
            TokenKind::Def => Ok(self.def(true)?),
            _ => todo!(),
        }
    }

    fn ret(&mut self) -> Result<Instruction<'instr>, ThrushError> {
        self.advance();

        if self.fun == 0 {
            return Err(ThrushError::Parse(
                ThrushErrorKind::SyntaxError,
                self.peek().lexeme.as_ref().unwrap().to_string(),
                String::from("Syntax Error"),
                String::from("Return statement outside of function. Invoke this keyword in scope of function definition."),
                self.peek().span,
                self.peek().line,
            ));
        }

        if self.peek().kind == TokenKind::SemiColon {
            self.consume(
                TokenKind::SemiColon,
                ThrushErrorKind::SyntaxError,
                String::from("Syntax Error"),
                String::from("Expected ';'."),
            )?;

            return Ok(Instruction::Return(Box::new(Instruction::Null)));
        }

        let value: Instruction<'instr> = self.parse()?;

        match &value {
            Instruction::Integer(kind, _) => match kind {
                DataTypes::U8 => self.ret = Some(DataTypes::U8),
                DataTypes::U16 => self.ret = Some(DataTypes::U16),
                DataTypes::U32 => self.ret = Some(DataTypes::U32),
                DataTypes::U64 => self.ret = Some(DataTypes::U64),

                DataTypes::I8 => self.ret = Some(DataTypes::I8),
                DataTypes::I16 => self.ret = Some(DataTypes::I16),
                DataTypes::I32 => self.ret = Some(DataTypes::I32),
                DataTypes::I64 => self.ret = Some(DataTypes::I64),

                DataTypes::F32 => self.ret = Some(DataTypes::F32),
                DataTypes::F64 => self.ret = Some(DataTypes::F64),

                _ => unreachable!(),
            },

            Instruction::String(_) => self.ret = Some(DataTypes::String),
            Instruction::Boolean(_) => self.ret = Some(DataTypes::Bool),

            _ => unreachable!(),
        }

        self.consume(
            TokenKind::SemiColon,
            ThrushErrorKind::SyntaxError,
            String::from("Syntax Error"),
            String::from("Expected ';'."),
        )?;

        Ok(Instruction::Return(Box::new(value)))
    }

    fn block(&mut self) -> Result<Instruction<'instr>, ThrushError> {
        self.advance();

        let mut stmts: Vec<Instruction> = Vec::new();

        while !self.match_token(TokenKind::RBrace) {
            stmts.push(self.parse()?)
        }

        Ok(Instruction::Block(stmts))
    }

    fn def(&mut self, is_public: bool) -> Result<Instruction<'instr>, ThrushError> {
        self.advance();

        self.begin_function();

        let name: Token = self.consume(
            TokenKind::Identifier,
            ThrushErrorKind::SyntaxError,
            String::from("Expected function name"),
            String::from("Expected def <name>."),
        )?;

        if name.lexeme.as_ref().unwrap() == "main" && self.scope == 0 && self.file.is_main {
            self.consume(
                TokenKind::LParen,
                ThrushErrorKind::SyntaxError,
                String::from("Syntax Error"),
                String::from("Expected '('."),
            )?;

            self.consume(
                TokenKind::RParen,
                ThrushErrorKind::SyntaxError,
                String::from("Syntax Error"),
                String::from("Expected ')'."),
            )?;

            self.consume(
                TokenKind::Colon,
                ThrushErrorKind::SyntaxError,
                String::from("Syntax Error"),
                String::from("Expected ':' for the return."),
            )?;

            match self.peek().kind {
                TokenKind::DataType(DataTypes::Void) => {
                    self.advance();

                    if self.peek().kind != TokenKind::LBrace {
                        return Err(ThrushError::Parse(
                            ThrushErrorKind::SyntaxError,
                            self.peek().lexeme.as_ref().unwrap().to_string(),
                            String::from("Syntax Error"),
                            String::from("Expected '{'."),
                            self.peek().span,
                            self.peek().line,
                        ));
                    }

                    return Ok(Instruction::EntryPoint {
                        body: Box::new(self.parse()?),
                    });
                }

                _ => {
                    return Err(ThrushError::Parse(
                        ThrushErrorKind::SyntaxError,
                        self.peek().lexeme.as_ref().unwrap().to_string(),
                        String::from("Syntax Error"),
                        String::from("Expected 'void' type return."),
                        self.peek().span,
                        self.peek().line,
                    ));
                }
            }
        }

        self.consume(
            TokenKind::LParen,
            ThrushErrorKind::SyntaxError,
            String::from("Syntax Error"),
            String::from("Expected '('."),
        )?;

        let mut params: Vec<Instruction> = Vec::with_capacity(8);

        while !self.match_token(TokenKind::RParen) {
            if self.match_token(TokenKind::Comma) {
                continue;
            }

            if params.len() >= 8 {
                return Err(ThrushError::Parse(
                    ThrushErrorKind::TooManyArguments,
                    self.peek().lexeme.as_ref().unwrap().to_string(),
                    String::from("Syntax Error"),
                    String::from("Too many arguments for the function. The maximum number of arguments is 8."),
                    self.peek().span,
                    self.peek().line,
                ));
            }

            if !self.match_token(TokenKind::Identifier) {
                return Err(ThrushError::Parse(
                    ThrushErrorKind::SyntaxError,
                    self.peek().lexeme.as_ref().unwrap().to_string(),
                    String::from("Syntax Error"),
                    String::from("Expected argument name."),
                    self.peek().span,
                    self.peek().line,
                ));
            }

            let ident: String = self.previous().lexeme.as_ref().unwrap().to_string();

            if !self.match_token(TokenKind::ColonColon) {
                return Err(ThrushError::Parse(
                    ThrushErrorKind::SyntaxError,
                    self.peek().lexeme.as_ref().unwrap().to_string(),
                    String::from("Syntax Error"),
                    String::from("Expected '::'."),
                    self.peek().span,
                    self.peek().line,
                ));
            }

            let kind: DataTypes = match &self.peek().kind {
                TokenKind::DataType(kind) => {
                    self.advance();

                    match kind {
                        DataTypes::I8 => DataTypes::I8,
                        DataTypes::I16 => DataTypes::I16,
                        DataTypes::I32 => DataTypes::I32,
                        DataTypes::I64 => DataTypes::I64,

                        DataTypes::U8 => DataTypes::U8,
                        DataTypes::U16 => DataTypes::U16,
                        DataTypes::U32 => DataTypes::U32,
                        DataTypes::U64 => DataTypes::U64,

                        DataTypes::F32 => DataTypes::F32,
                        DataTypes::F64 => DataTypes::F64,

                        DataTypes::Bool => DataTypes::Bool,

                        DataTypes::String => DataTypes::String,

                        DataTypes::Void => DataTypes::Void,
                    }
                }
                _ => {
                    return Err(ThrushError::Parse(
                        ThrushErrorKind::SyntaxError,
                        self.peek().lexeme.as_ref().unwrap().to_string(),
                        String::from("Syntax Error"),
                        String::from("Expected argument type."),
                        self.peek().span,
                        self.peek().line,
                    ));
                }
            };

            params.push(Instruction::Param(ident, kind))
        }

        if self.peek().kind == TokenKind::Colon {
            self.consume(
                TokenKind::Colon,
                ThrushErrorKind::SyntaxError,
                String::from("Syntax Error"),
                String::from("Missing return type. Expected ':' followed by return type."),
            )?;
        }

        let return_kind: Option<DataTypes> = match &self.peek().kind {
            TokenKind::DataType(kind) => {
                self.advance();

                match kind {
                    DataTypes::I8 => Some(DataTypes::I8),
                    DataTypes::I16 => Some(DataTypes::I16),
                    DataTypes::I32 => Some(DataTypes::I32),
                    DataTypes::I64 => Some(DataTypes::I64),

                    DataTypes::U8 => Some(DataTypes::U8),
                    DataTypes::U16 => Some(DataTypes::U16),
                    DataTypes::U32 => Some(DataTypes::U32),
                    DataTypes::U64 => Some(DataTypes::U64),

                    DataTypes::F32 => Some(DataTypes::F32),
                    DataTypes::F64 => Some(DataTypes::F64),

                    DataTypes::Bool => Some(DataTypes::Bool),

                    DataTypes::String => Some(DataTypes::String),

                    DataTypes::Void => Some(DataTypes::Void),
                }
            }
            _ => None,
        };

        let body: Box<Instruction> = Box::new(self.block()?);

        match &return_kind {
            Some(kind) => {
                if self.ret.is_none() {
                    return Err(ThrushError::Parse(
                        ThrushErrorKind::SyntaxError,
                        name.lexeme.as_ref().unwrap().to_string(),
                        String::from("Syntax Error"),
                        format!("Missing return statement with type '{}', you should add a return statement with type '{}'.", kind, kind),
                        name.span,
                        name.line,
                    ));
                }

                match kind != self.ret.as_ref().unwrap() {
                    true => {
                        return Err(ThrushError::Parse(
                            ThrushErrorKind::SyntaxError,
                            name.lexeme.as_ref().unwrap().to_string(),
                            String::from("Syntax Error"),
                            format!(
                                "Expected return type of '{}', found '{}'. You should write a return statement with type '{}'.",
                                kind,
                                self.ret.as_ref().unwrap(),
                                kind
                            ),
                            name.span,
                            name.line,
                        ))
                    }

                    false => {}
                }
            }

            None => {}
        }

        self.end_function();

        Ok(Instruction::Function {
            name: name.lexeme.as_ref().unwrap().to_string(),
            params,
            body,
            return_kind,
            is_public,
        })
    }

    fn println(&mut self) -> Result<Instruction<'instr>, ThrushError> {
        self.advance();

        self.consume(
            TokenKind::LParen,
            ThrushErrorKind::SyntaxError,
            String::from("Syntax Error"),
            String::from("Expected '('."),
        )?;

        let mut args: Vec<Instruction<'instr>> = Vec::with_capacity(24);

        while !self.match_token(TokenKind::RParen) {
            if self.match_token(TokenKind::Comma) {
                continue;
            }

            if args.len() >= 24 {
                return Err(ThrushError::Parse(
                    ThrushErrorKind::TooManyArguments,
                    self.peek().lexeme.as_ref().unwrap().to_string(),
                    String::from("Syntax Error"),
                    String::from("Expected ')'. Too many arguments. Max is 24."),
                    self.peek().span,
                    self.peek().line,
                ));
            }

            args.push(self.expr()?);
        }

        self.consume(
            TokenKind::SemiColon,
            ThrushErrorKind::SyntaxError,
            String::from("Syntax Error"),
            String::from("Expected ';'."),
        )?;

        Ok(Instruction::Println(args))
    }

    fn puts(&mut self) -> Result<Instruction<'instr>, ThrushError> {
        self.advance();

        self.consume(
            TokenKind::LParen,
            ThrushErrorKind::SyntaxError,
            String::from("Syntax Error"),
            String::from("Expected '('."),
        )?;

        let arg: Instruction<'instr> = self.parse()?;

        if arg != Instruction::String(String::from("")) {
            return Err(ThrushError::Parse(
                ThrushErrorKind::SyntaxError,
                self.peek().lexeme.as_ref().unwrap().to_string(),
                String::from("Syntax Error"),
                String::from("Expected string for 'puts' call."),
                self.peek().span,
                self.peek().line,
            ));
        }

        self.consume(
            TokenKind::RParen,
            ThrushErrorKind::SyntaxError,
            String::from("Syntax Error"),
            String::from("Expected ')'."),
        )?;

        self.consume(
            TokenKind::SemiColon,
            ThrushErrorKind::SyntaxError,
            String::from("Syntax Error"),
            String::from("Expected ';'."),
        )?;

        Ok(Instruction::Puts(Box::new(arg)))
    }

    fn string(&mut self) -> Result<String, ThrushError> {
        Ok(self.advance().lexeme.as_ref().unwrap().to_string())
    }

    fn expr(&mut self) -> Result<Instruction<'instr>, ThrushError> {
        self.expression()
    }

    fn expression(&mut self) -> Result<Instruction<'instr>, ThrushError> {
        let expr: Instruction = self.primary()?;

        Ok(expr)
    }

    fn primary(&mut self) -> Result<Instruction<'instr>, ThrushError> {
        let primary: Instruction = match &self.peek().kind {
            TokenKind::String => Instruction::String(self.string()?),
            kind => match kind {
                TokenKind::Integer(kind, num) => {
                    self.advance();

                    match kind {
                        DataTypes::I8 => Instruction::Integer(DataTypes::I8, (*num as i8).into()),
                        DataTypes::I16 => {
                            Instruction::Integer(DataTypes::I16, (*num as i16).into())
                        }
                        DataTypes::I32 => {
                            Instruction::Integer(DataTypes::I32, (*num as i32).into())
                        }
                        DataTypes::I64 => {
                            Instruction::Integer(DataTypes::I64, (*num as i64) as f64)
                        }

                        DataTypes::U8 => Instruction::Integer(DataTypes::U8, (*num as u8).into()),
                        DataTypes::U16 => Instruction::Integer(DataTypes::U8, (*num as u16).into()),
                        DataTypes::U32 => {
                            Instruction::Integer(DataTypes::U32, (*num as u32).into())
                        }
                        DataTypes::U64 => {
                            Instruction::Integer(DataTypes::U64, (*num as u64) as f64)
                        }

                        _ => unreachable!(),
                    }
                }

                TokenKind::True => {
                    self.advance();

                    Instruction::Boolean(true)
                }

                TokenKind::False => {
                    self.advance();

                    Instruction::Boolean(false)
                }

                TokenKind::RParen | TokenKind::RBrace => {
                    self.advance();

                    return Err(ThrushError::Parse(
                            ThrushErrorKind::SyntaxError,
                            self.peek().lexeme.as_ref().unwrap().to_string(),
                            String::from("Syntax Error"),
                            format!("Expected expression, found '{}'. Is this a function call or an function definition?", kind),
                            self.peek().span,
                            self.peek().line,
                        ));
                }

                kind => {
                    self.advance();

                    println!("{:?}", self.previous());

                    return Err(ThrushError::Parse(
                        ThrushErrorKind::SyntaxError,
                        self.peek().lexeme.as_ref().unwrap().to_string(),
                        String::from("Syntax Error"),
                        format!("Unexpected code '{}', check the code and review the syntax rules in the documentation.", kind),
                        self.peek().span,
                        self.peek().line,
                    ));
                }
            },
        };

        Ok(primary)
    }

    fn consume(
        &mut self,
        kind: TokenKind,
        error_kind: ThrushErrorKind,
        error_title: String,
        help: String,
    ) -> Result<Token, ThrushError> {
        if self.peek().kind == kind {
            return Ok(self.advance());
        }

        println!("{:?}", self.peek());

        Err(ThrushError::Parse(
            error_kind,
            self.peek().lexeme.as_ref().unwrap().to_string(),
            error_title,
            help,
            self.peek().span,
            self.peek().line,
        ))
    }

    #[inline]
    fn begin_function(&mut self) {
        self.fun += 1;
    }

    #[inline]
    fn end_function(&mut self) {
        self.fun -= 1;
    }

    fn check(&mut self, kind: TokenKind) -> bool {
        if self.end() {
            return false;
        }

        self.peek().kind == kind
    }

    fn match_token(&mut self, kind: TokenKind) -> bool {
        if self.end() {
            return false;
        } else if self.peek().kind == kind {
            self.advance();
            return true;
        }

        false
    }

    fn advance(&mut self) -> Token {
        if !self.end() {
            self.current += 1;
        }

        self.previous().clone()
    }

    fn peek(&self) -> &'parser Token {
        &self.tokens[self.current]
    }

    fn previous(&self) -> &'parser Token {
        &self.tokens[self.current - 1]
    }

    fn end(&self) -> bool {
        self.peek().kind == TokenKind::Eof
    }
}

impl PartialEq for Instruction<'_> {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Instruction::Integer(_, _) => {
                matches!(other, Instruction::Integer(_, _))
            }

            Instruction::String(_) => {
                matches!(other, Instruction::String(_))
            }

            _ => self == other,
        }
    }
}
