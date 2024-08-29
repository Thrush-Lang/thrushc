use {
    super::{
        super::{LarkError, LarkErrorKind, LarkFile},
        diagnostic::Diagnostic,
        lexer::{DataTypes, Token, TokenKind},
    },
    inkwell::values::PointerValue,
    std::mem,
};

#[derive(Debug, Clone)]
pub enum Instruction<'instr> {
    Puts(Box<Instruction<'instr>>),
    String(String),
    Block(Vec<Instruction<'instr>>),
    EntryPoint { body: Box<Instruction<'instr>> },
    PointerValue(PointerValue<'instr>),
    Null,
    End,
}

pub struct Parser<'parser, 'instr> {
    stmts: Vec<Instruction<'instr>>,
    errors: Vec<LarkError>,
    tokens: &'parser [Token],
    current: usize,
    scope: usize,
    file: LarkFile,
}

impl<'parser, 'instr> Parser<'parser, 'instr> {
    pub fn new(tokens: &'parser [Token], file: LarkFile) -> Self {
        Self {
            stmts: Vec::new(),
            errors: Vec::with_capacity(10),
            tokens,
            current: 0,
            scope: 0,
            file,
        }
    }

    pub fn parse(&mut self) -> Result<&[Instruction<'instr>], LarkError> {
        while !self.end() {
            match self.def() {
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

            return Err(LarkError::Compile(String::from(
                "Compilation proccess ended with errors.",
            )));
        }

        self.stmts.push(Instruction::End);

        Ok(self.stmts.as_slice())
    }

    fn def(&mut self) -> Result<Instruction<'instr>, LarkError> {
        match self.peek().kind {
            TokenKind::Puts => Ok(self.puts()?),
            TokenKind::Def => Ok(self.func()?),
            TokenKind::LBrace => Ok(self.block()?),
            _ => Ok(self.expr()?),
        }
    }

    fn block(&mut self) -> Result<Instruction<'instr>, LarkError> {
        self.advance();

        let mut stmts: Vec<Instruction> = Vec::new();

        while !self.match_token(TokenKind::RBrace) {
            stmts.push(self.def()?)
        }

        self.advance();

        Ok(Instruction::Block(stmts))
    }

    fn func(&mut self) -> Result<Instruction<'instr>, LarkError> {
        self.advance();

        let name: Token = self.consume(
            TokenKind::Identifier,
            LarkErrorKind::SyntaxError,
            String::from("Expected function name"),
            String::from("Expected def <name>."),
        )?;

        if name.lexeme.as_ref().unwrap() == "main" && self.scope == 0 && self.file.is_main {
            self.consume(
                TokenKind::LParen,
                LarkErrorKind::SyntaxError,
                String::from("Syntax Error"),
                String::from("Expected '('."),
            )?;

            self.consume(
                TokenKind::RParen,
                LarkErrorKind::SyntaxError,
                String::from("Syntax Error"),
                String::from("Expected ')'."),
            )?;

            self.consume(
                TokenKind::Colon,
                LarkErrorKind::SyntaxError,
                String::from("Syntax Error"),
                String::from("Expected ':'."),
            )?;

            match self.peek().kind {
                TokenKind::DataType(DataTypes::Void) => {
                    self.advance();

                    if self.peek().kind != TokenKind::LBrace {
                        return Err(LarkError::Parse(
                            LarkErrorKind::SyntaxError,
                            self.peek().lexeme.as_ref().unwrap().to_string(),
                            String::from("Syntax Error"),
                            String::from("Expected '{'."),
                            self.peek().span,
                            self.peek().line,
                        ));
                    }

                    return Ok(Instruction::EntryPoint {
                        body: Box::new(self.def()?),
                    });
                }

                _ => {
                    return Err(LarkError::Parse(
                        LarkErrorKind::SyntaxError,
                        self.peek().lexeme.as_ref().unwrap().to_string(),
                        String::from("Syntax Error"),
                        String::from("Expected type 'void' return."),
                        self.peek().span,
                        self.peek().line,
                    ));
                }
            }
        }

        todo!()
    }

    fn puts(&mut self) -> Result<Instruction<'instr>, LarkError> {
        self.advance();

        self.consume(
            TokenKind::LParen,
            LarkErrorKind::SyntaxError,
            String::from("Syntax Error"),
            String::from("Expected '('."),
        )?;

        let arg: Instruction<'instr> = self.def()?;

        self.consume(
            TokenKind::RParen,
            LarkErrorKind::SyntaxError,
            String::from("Syntax Error"),
            String::from("Expected ')'."),
        )?;

        self.consume(
            TokenKind::SemiColon,
            LarkErrorKind::SyntaxError,
            String::from("Syntax Error"),
            String::from("Expected ';'."),
        )?;

        Ok(Instruction::Puts(Box::new(arg)))
    }

    fn string(&mut self) -> Result<String, LarkError> {
        Ok(self.advance().lexeme.as_ref().unwrap().to_string())
    }

    fn expr(&mut self) -> Result<Instruction<'instr>, LarkError> {
        self.expression()
    }

    fn expression(&mut self) -> Result<Instruction<'instr>, LarkError> {
        let expr: Instruction = self.primary()?;

        Ok(expr)
    }

    fn primary(&mut self) -> Result<Instruction<'instr>, LarkError> {
        let primary: Instruction = match &self.peek().kind {
            TokenKind::String => Instruction::String(self.string()?),
            kind => match kind {
                TokenKind::RParen | TokenKind::RBrace => {
                    self.advance();

                    return Err(LarkError::Parse(
                            LarkErrorKind::SyntaxError,
                            self.peek().lexeme.as_ref().unwrap().to_string(),
                            String::from("Syntax Error"),
                            format!("Expected expression, found '{}'. Is this a function call or an function definition?", kind),
                            self.peek().span,
                            self.peek().line,
                        ));
                }
                kind => {
                    self.advance();

                    return Err(LarkError::Parse(
                        LarkErrorKind::SyntaxError,
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
        error_kind: LarkErrorKind,
        error_title: String,
        help: String,
    ) -> Result<Token, LarkError> {
        if self.peek().kind == kind {
            return Ok(self.advance());
        }

        Err(LarkError::Parse(
            error_kind,
            self.peek().lexeme.as_ref().unwrap().to_string(),
            error_title,
            help,
            self.peek().span,
            self.peek().line,
        ))
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
