use {
    super::{
        super::{
            backend::{compiler::options::ThrushFile, instruction::Instruction},
            diagnostic::Diagnostic,
            error::{ThrushError, ThrushErrorKind},
            logging::{self, LogType},
        },
        lexer::{DataTypes, Token, TokenKind},
        scoper::ThrushScoper,
        type_checking,
    },
    ahash::AHashMap as HashMap,
    std::{mem, process},
};

type ParserGlobals<'instr> = HashMap<&'instr str, (DataTypes, bool, Vec<DataTypes>)>;
type ParserLocals<'instr> = Vec<HashMap<&'instr str, (DataTypes, bool, bool, bool, usize)>>;
type GetObjectResult = Result<(DataTypes, bool, bool, Vec<DataTypes>), ThrushError>;

pub struct Parser<'instr> {
    stmts: Vec<Instruction<'instr>>,
    errors: Vec<ThrushError>,
    tokens: &'instr [Token],
    in_function: bool,
    in_type_function: DataTypes,
    in_var_type: DataTypes,
    current: usize,
    globals: ParserGlobals<'instr>,
    locals: ParserLocals<'instr>,
    scope: usize,
    scoper: ThrushScoper<'instr>,
    diagnostic: Diagnostic,
    has_entry_point: bool,
    is_main: bool,
}

impl<'instr> Parser<'instr> {
    pub fn new(tokens: &'instr [Token], file: &ThrushFile) -> Self {
        Self {
            stmts: Vec::new(),
            errors: Vec::new(),
            tokens,
            current: 0,
            in_function: false,
            in_type_function: DataTypes::Void,
            in_var_type: DataTypes::Void,
            globals: HashMap::new(),
            locals: vec![HashMap::new()],
            scope: 0,
            scoper: ThrushScoper::new(file),
            diagnostic: Diagnostic::new(file),
            has_entry_point: false,
            is_main: file.is_main,
        }
    }

    pub fn start(&mut self) -> &[Instruction<'instr>] {
        self.predefine_functions();

        while !self.end() {
            match self.parse() {
                Ok(instr) => {
                    self.stmts.push(instr);
                }
                Err(e) => {
                    self.errors.push(e);
                    self.sync();
                }
            }
        }
        if !self.errors.is_empty() {
            self.errors.iter().for_each(|error| {
                self.diagnostic.report(error, LogType::ERROR);
            });

            process::exit(1);
        } else if self.is_main && !self.has_entry_point {
            logging::log(
                logging::LogType::ERROR,
                "Missing entrypoint \"fn main() {}\" in main.th file.",
            );

            process::exit(1);
        }

        self.scoper.analyze();

        self.stmts.as_slice()
    }

    fn parse(&mut self) -> Result<Instruction<'instr>, ThrushError> {
        match &self.peek().kind {
            TokenKind::Println => Ok(self.println()?),
            TokenKind::Print => Ok(self.print()?),
            TokenKind::Fn => Ok(self.function(false)?),
            TokenKind::LBrace => Ok(self.block(&mut [])?),
            TokenKind::Return => Ok(self.ret()?),
            TokenKind::Public => Ok(self.public()?),
            TokenKind::Var => Ok(self.variable(false)?),
            TokenKind::For => Ok(self.for_loop()?),
            _ => Ok(self.expression()?),
        }
    }

    fn for_loop(&mut self) -> Result<Instruction<'instr>, ThrushError> {
        self.only_advance()?;

        let start_line: usize = self.previous().line;

        let variable: Instruction<'instr> = self.variable(false)?;

        let cond: Instruction<'instr> = self.expression()?;

        self.consume(
            TokenKind::SemiColon,
            ThrushErrorKind::SyntaxError,
            String::from("Syntax Error"),
            String::from("Expected ';'."),
            start_line,
        )?;

        let actions: Instruction<'instr> = self.expression()?;

        let mut variable_clone: Instruction<'instr> = variable.clone();

        if let Instruction::Var { only_comptime, .. } = &mut variable_clone {
            *only_comptime = true;
        }

        let body: Instruction<'instr> = self.block(&mut [variable_clone])?;

        Ok(Instruction::ForLoop {
            variable: Some(Box::new(variable)),
            cond: Some(Box::new(cond)),
            actions: Some(Box::new(actions)),
            block: Box::new(body),
        })
    }

    fn variable(&mut self, only_comptime: bool) -> Result<Instruction<'instr>, ThrushError> {
        self.only_advance()?;

        let name: &Token = self.consume(
            TokenKind::Identifier,
            ThrushErrorKind::SyntaxError,
            String::from("Expected variable name"),
            String::from("Expected var (name)."),
            self.previous().line,
        )?;

        if self.peek().kind == TokenKind::SemiColon {
            self.only_advance()?;

            self.errors.push(ThrushError::Parse(
                ThrushErrorKind::SyntaxError,
                String::from("Syntax Error"),
                String::from("Expected type for the variable. You forget the `:`."),
                name.line,
            ));
        } else if self.peek().kind == TokenKind::Colon {
            self.consume(
                TokenKind::Colon,
                ThrushErrorKind::SyntaxError,
                String::from("Expected variable type indicator"),
                String::from("Expected `var name --> : <-- type = value;`."),
                name.line,
            )?;
        }

        let kind: DataTypes = match &self.peek().kind {
            TokenKind::DataType(kind) => {
                if self.previous().kind != TokenKind::Colon {
                    self.errors.push(ThrushError::Parse(
                        ThrushErrorKind::SyntaxError,
                        String::from("Expected variable type indicator"),
                        String::from("Expected `var name --> : <-- type = value;`."),
                        name.line,
                    ));
                }

                self.only_advance()?;

                *kind
            }

            _ => {
                self.errors.push(ThrushError::Parse(
                    ThrushErrorKind::SyntaxError,
                    String::from("Syntax Error"),
                    String::from("Expected type for the variable."),
                    name.line,
                ));

                DataTypes::Void
            }
        };

        if self.peek().kind == TokenKind::SemiColon && kind == DataTypes::Void {
            self.only_advance()?;

            self.errors.push(ThrushError::Parse(
                ThrushErrorKind::SyntaxError,
                String::from("Syntax Error"),
                String::from(
                    "Variable type is undefined. Did you forget to specify the variable type to undefined variable?",
                ),
                name.line,
            ));
        } else if self.peek().kind == TokenKind::SemiColon {
            self.consume(
                TokenKind::SemiColon,
                ThrushErrorKind::SyntaxError,
                String::from("Syntax Error"),
                String::from("Expected ';'."),
                name.line,
            )?;

            self.define_local(name.lexeme.as_ref().unwrap(), (kind, true, false, false, 0));

            return Ok(Instruction::Var {
                name: name.lexeme.as_ref().unwrap(),
                kind,
                value: Box::new(Instruction::Null),
                line: name.line,
                only_comptime,
            });
        }

        self.consume(
            TokenKind::Eq,
            ThrushErrorKind::SyntaxError,
            String::from("Syntax Error"),
            String::from("Expected '=' for the variable definition."),
            name.line,
        )?;

        self.in_var_type = kind;

        let value: Instruction<'instr> = self.expression()?;
        let value_type: DataTypes = value.get_data_type();

        if let Err(e) = type_checking::check_type(
            value_type,
            kind,
            name.line,
            String::from("Type Mismatch"),
            format!(
                "Type mismatch. Expected '{}' but found '{}'.",
                kind, value_type
            ),
        ) {
            self.errors.push(e);
        }

        self.define_local(
            name.lexeme.as_ref().unwrap(),
            (kind, false, false, false, 0),
        );

        if let Instruction::RefVar { kind, .. } = &value {
            if kind == &DataTypes::String {
                self.modify_deallocation(name.lexeme.as_ref().unwrap(), true, false);
            }
        }

        let var: Instruction<'_> = Instruction::Var {
            name: name.lexeme.as_ref().unwrap(),
            kind,
            value: Box::new(value),
            line: name.line,
            only_comptime,
        };

        self.consume(
            TokenKind::SemiColon,
            ThrushErrorKind::SyntaxError,
            String::from("Syntax Error"),
            String::from("Expected ';'."),
            name.line,
        )?;

        Ok(var)
    }

    fn public(&mut self) -> Result<Instruction<'instr>, ThrushError> {
        self.only_advance()?;

        match &self.peek().kind {
            TokenKind::Fn => Ok(self.function(true)?),
            _ => unimplemented!(),
        }
    }

    fn ret(&mut self) -> Result<Instruction<'instr>, ThrushError> {
        self.only_advance()?;

        let line: usize = self.previous().line;

        if !self.in_function {
            self.errors.push(ThrushError::Parse(
                ThrushErrorKind::SyntaxError,
                String::from("Syntax Error"),
                String::from("Return statement outside of function. Invoke this keyword in scope of function."),
                line
            ));
        }

        if self.peek().kind == TokenKind::SemiColon {
            self.consume(
                TokenKind::SemiColon,
                ThrushErrorKind::SyntaxError,
                String::from("Syntax Error"),
                String::from("Expected ';'."),
                line,
            )?;

            if self.in_type_function != DataTypes::Void {
                self.errors.push(ThrushError::Parse(
                    ThrushErrorKind::SyntaxError,
                    String::from("Syntax Error"),
                    format!("Missing return statement with correctly type '{}', you should rewrite for return with type '{}'.", self.in_type_function, self.in_type_function),
                    line,
                ));
            }

            self.emit_deallocators();

            return Ok(Instruction::Return(
                Box::new(Instruction::Null),
                DataTypes::Void,
            ));
        }

        let value: Instruction<'instr> = self.expression()?;

        if let Instruction::RefVar { name, kind, .. } = value {
            if kind == DataTypes::String {
                self.modify_deallocation(name, false, true);
            }
        }

        if self.in_type_function == DataTypes::Void && value.get_data_type() != DataTypes::Void {
            self.errors.push(ThrushError::Parse(
                ThrushErrorKind::SyntaxError,
                String::from("Syntax Error"),
                format!("Missing function type indicator with type '{}', you should add a correct function type indicator with type '{}'.", value.get_data_type(), value.get_data_type()),
                line,
            ));
        }

        type_checking::check_type(
            value.get_data_type(),
            self.in_type_function,
            line,
            String::from("Type Mismatch"),
            format!(
                "Type mismatch. Expected '{}' but found '{}'.",
                self.in_type_function,
                value.get_data_type(),
            ),
        )?;

        self.consume(
            TokenKind::SemiColon,
            ThrushErrorKind::SyntaxError,
            String::from("Syntax Error"),
            String::from("Expected ';'."),
            line,
        )?;

        Ok(Instruction::Return(Box::new(value), self.in_type_function))
    }

    fn block(
        &mut self,
        with_instrs: &mut [Instruction<'instr>],
    ) -> Result<Instruction<'instr>, ThrushError> {
        self.only_advance()?;

        self.begin_scope();

        let mut stmts: Vec<Instruction> = Vec::new();
        let mut was_emited_deallocators: bool = false;

        for instr in with_instrs.iter_mut() {
            stmts.push(mem::take(instr));
        }

        while !self.match_token(TokenKind::RBrace)? {
            let instr: Instruction<'instr> = self.parse()?;
            let line: usize = self.previous().line;

            if instr.is_return() {
                if instr.is_indexe_return_of_string() {
                    self.errors.push(ThrushError::Parse(
                        ThrushErrorKind::SyntaxError,
                        String::from("Unreacheable Deallocation"),
                        String::from("In this point the correctly deallocation is imposible. The char should be stored in a variable and pass it variable to the return."),
                        line,
                    ));
                }

                let deallocators: Vec<Instruction<'_>> = self.emit_deallocators();

                stmts.extend(deallocators);

                was_emited_deallocators = true;
            }

            stmts.push(instr)
        }

        if !was_emited_deallocators {
            stmts.extend(self.emit_deallocators());
        }

        self.end_scope();

        self.scoper.add_scope(stmts.clone());

        Ok(Instruction::Block { stmts })
    }

    fn function(&mut self, is_public: bool) -> Result<Instruction<'instr>, ThrushError> {
        self.only_advance()?;

        if self.scope != 0 {
            self.errors.push(ThrushError::Parse(
                ThrushErrorKind::SyntaxError,
                String::from("Syntax Error"),
                String::from(
                    "The functions must go in the global scope. Rewrite it in the global scope.",
                ),
                self.previous().line,
            ));
        }

        self.in_function = true;

        let name: &Token = self.consume(
            TokenKind::Identifier,
            ThrushErrorKind::SyntaxError,
            String::from("Expected function name"),
            String::from("Expected fn < name >."),
            self.previous().line,
        )?;

        if name.lexeme.as_ref().unwrap() == "main" && self.is_main {
            if self.has_entry_point {
                self.errors.push(ThrushError::Parse(
                    ThrushErrorKind::SyntaxError,
                    String::from("Duplicated EntryPoint"),
                    String::from("The language not support two entrypoints, remove one."),
                    name.line,
                ));
            }

            self.consume(
                TokenKind::LParen,
                ThrushErrorKind::SyntaxError,
                String::from("Syntax Error"),
                String::from("Expected '('."),
                name.line,
            )?;

            self.consume(
                TokenKind::RParen,
                ThrushErrorKind::SyntaxError,
                String::from("Syntax Error"),
                String::from("Expected ')'."),
                name.line,
            )?;

            if self.peek().kind != TokenKind::LBrace {
                self.errors.push(ThrushError::Parse(
                    ThrushErrorKind::SyntaxError,
                    String::from("Syntax Error"),
                    String::from("Expected '{'."),
                    self.peek().line,
                ));
            }

            if self.peek().kind == TokenKind::LBrace {
                self.has_entry_point = true;

                return Ok(Instruction::EntryPoint {
                    body: Box::new(self.block(&mut [])?),
                });
            } else {
                self.errors.push(ThrushError::Parse(
                    ThrushErrorKind::SyntaxError,
                    String::from("Syntax Error"),
                    String::from("Expected 'block ({ ... })' for the function body."),
                    self.peek().line,
                ));
            }
        }

        self.consume(
            TokenKind::LParen,
            ThrushErrorKind::SyntaxError,
            String::from("Syntax Error"),
            String::from("Expected '('."),
            name.line,
        )?;

        let mut params: Vec<Instruction<'instr>> = Vec::new();

        while !self.match_token(TokenKind::RParen)? {
            if self.match_token(TokenKind::Comma)? {
                continue;
            }

            if !self.match_token(TokenKind::Identifier)? {
                self.errors.push(ThrushError::Parse(
                    ThrushErrorKind::SyntaxError,
                    String::from("Syntax Error"),
                    String::from("Expected argument name."),
                    name.line,
                ));
            }

            let ident: &str = self.previous().lexeme.as_ref().unwrap();

            if !self.match_token(TokenKind::ColonColon)? {
                self.errors.push(ThrushError::Parse(
                    ThrushErrorKind::SyntaxError,
                    String::from("Syntax Error"),
                    String::from("Expected '::'."),
                    name.line,
                ));
            }

            let kind: DataTypes = match &self.peek().kind {
                TokenKind::DataType(kind) => {
                    self.only_advance()?;

                    *kind
                }
                _ => {
                    self.errors.push(ThrushError::Parse(
                        ThrushErrorKind::SyntaxError,
                        String::from("Syntax Error"),
                        String::from("Expected argument type."),
                        name.line,
                    ));

                    DataTypes::Void
                }
            };

            params.push(Instruction::Param { name: ident, kind })
        }

        if self.peek().kind == TokenKind::Colon {
            self.consume(
                TokenKind::Colon,
                ThrushErrorKind::SyntaxError,
                String::from("Syntax Error"),
                String::from("Missing return type. Expected ':' followed by return type."),
                name.line,
            )?;
        }

        let return_kind: Option<DataTypes> = match &self.peek().kind {
            TokenKind::DataType(kind) => {
                self.only_advance()?;
                Some(*kind)
            }
            _ => None,
        };

        self.in_type_function = if let Some(kind) = return_kind {
            kind
        } else {
            DataTypes::Void
        };

        let body: Box<Instruction> = Box::new(self.block(&mut [])?);

        self.in_function = false;

        Ok(Instruction::Function {
            name: name.lexeme.as_ref().unwrap(),
            params,
            body,
            return_kind,
            is_public,
        })
    }

    fn print(&mut self) -> Result<Instruction<'instr>, ThrushError> {
        self.only_advance()?;

        let start: &Token = self.consume(
            TokenKind::LParen,
            ThrushErrorKind::SyntaxError,
            String::from("Syntax Error"),
            String::from("Expected '('."),
            self.previous().line,
        )?;

        let mut args: Vec<Instruction<'instr>> = Vec::with_capacity(24);

        while !self.match_token(TokenKind::RParen)? {
            if self.match_token(TokenKind::Comma)? {
                continue;
            }

            args.push(self.expression()?);
        }

        self.parse_string_formatted(&args, start.line, true);

        self.consume(
            TokenKind::SemiColon,
            ThrushErrorKind::SyntaxError,
            String::from("Syntax Error"),
            String::from("Expected ';'."),
            start.line,
        )?;

        Ok(Instruction::Print(args))
    }

    fn println(&mut self) -> Result<Instruction<'instr>, ThrushError> {
        self.only_advance()?;

        let start: &Token = self.consume(
            TokenKind::LParen,
            ThrushErrorKind::SyntaxError,
            String::from("Syntax Error"),
            String::from("Expected '('."),
            self.previous().line,
        )?;

        let mut args: Vec<Instruction<'instr>> = Vec::new();

        while !self.match_token(TokenKind::RParen)? {
            if self.match_token(TokenKind::Comma)? {
                continue;
            }

            args.push(self.expression()?);
        }

        self.parse_string_formatted(&args, start.line, false);

        self.consume(
            TokenKind::SemiColon,
            ThrushErrorKind::SyntaxError,
            String::from("Syntax Error"),
            String::from("Expected ';'."),
            start.line,
        )?;

        Ok(Instruction::Println(args))
    }

    fn expression(&mut self) -> Result<Instruction<'instr>, ThrushError> {
        let instr: Instruction = self.or()?;

        self.locals.iter_mut().for_each(|scope| {
            scope.values_mut().for_each(|variable| {
                if variable.4 > 0 {
                    variable.4 -= 1;
                }
            });
        });

        Ok(instr)
    }

    fn or(&mut self) -> Result<Instruction<'instr>, ThrushError> {
        let mut instr: Instruction<'_> = self.and()?;

        while self.match_token(TokenKind::Or)? {
            let op: &TokenKind = &self.previous().kind;
            let right: Instruction<'instr> = self.and()?;

            type_checking::check_binary_instr(
                op,
                &instr.get_data_type(),
                &right.get_data_type(),
                self.previous().line,
            )?;

            instr = Instruction::Binary {
                left: Box::new(instr),
                op,
                right: Box::new(right),
                kind: DataTypes::Bool,
                line: self.previous().line,
            }
        }

        Ok(instr)
    }

    fn and(&mut self) -> Result<Instruction<'instr>, ThrushError> {
        let mut instr: Instruction<'_> = self.equality()?;

        while self.match_token(TokenKind::And)? {
            let op: &TokenKind = &self.previous().kind;
            let right: Instruction<'_> = self.equality()?;

            type_checking::check_binary_instr(
                op,
                &instr.get_data_type(),
                &right.get_data_type(),
                self.previous().line,
            )?;

            instr = Instruction::Binary {
                left: Box::new(instr),
                op,
                right: Box::new(right),
                kind: DataTypes::Bool,
                line: self.previous().line,
            }
        }

        Ok(instr)
    }

    fn equality(&mut self) -> Result<Instruction<'instr>, ThrushError> {
        let mut instr: Instruction<'_> = self.comparison()?;

        while self.match_token(TokenKind::BangEq)? || self.match_token(TokenKind::EqEq)? {
            let op: &TokenKind = &self.previous().kind;
            let right: Instruction<'_> = self.comparison()?;

            type_checking::check_binary_instr(
                op,
                &instr.get_data_type(),
                &right.get_data_type(),
                self.previous().line,
            )?;

            instr = Instruction::Binary {
                left: Box::from(instr),
                op,
                right: Box::from(right),
                kind: DataTypes::Bool,
                line: self.previous().line,
            }
        }

        Ok(instr)
    }

    fn comparison(&mut self) -> Result<Instruction<'instr>, ThrushError> {
        let mut instr: Instruction<'_> = self.term()?;

        while self.match_token(TokenKind::Greater)?
            || self.match_token(TokenKind::GreaterEq)?
            || self.match_token(TokenKind::Less)?
            || self.match_token(TokenKind::LessEq)?
        {
            let op: &TokenKind = &self.previous().kind;
            let right: Instruction<'_> = self.term()?;

            type_checking::check_binary_instr(
                op,
                &instr.get_data_type(),
                &right.get_data_type(),
                self.previous().line,
            )?;

            instr = Instruction::Binary {
                left: Box::from(instr),
                op,
                right: Box::from(right),
                kind: DataTypes::Bool,
                line: self.previous().line,
            };
        }

        Ok(instr)
    }

    fn term(&mut self) -> Result<Instruction<'instr>, ThrushError> {
        let mut instr: Instruction<'_> = self.unary()?;

        while self.match_token(TokenKind::Plus)?
            || self.match_token(TokenKind::Minus)?
            || self.match_token(TokenKind::Slash)?
            || self.match_token(TokenKind::Star)?
        {
            let op: &TokenKind = &self.previous().kind;
            let right: Instruction<'_> = self.unary()?;

            let left_type: DataTypes = instr.get_data_type();
            let right_type: DataTypes = right.get_data_type();

            let kind: DataTypes = if left_type.is_integer() && right_type.is_integer() {
                left_type.determinate_integer_datatype(right_type)
            } else {
                self.in_var_type
            };

            type_checking::check_binary_instr(
                op,
                &instr.get_data_type(),
                &right.get_data_type(),
                self.previous().line,
            )?;

            instr = Instruction::Binary {
                left: Box::from(instr),
                op,
                right: Box::from(right),
                kind,
                line: self.previous().line,
            };
        }

        Ok(instr)
    }

    fn unary(&mut self) -> Result<Instruction<'instr>, ThrushError> {
        if self.match_token(TokenKind::Bang)? {
            let line: usize = self.previous().line;

            let op: &TokenKind = &self.previous().kind;
            let value: Instruction<'instr> = self.primary()?;

            type_checking::check_unary_instr(op, &value.get_data_type(), self.previous().line)?;

            return Ok(Instruction::Unary {
                op,
                value: Box::from(value),
                kind: DataTypes::Bool,
                line,
            });
        } else if self.match_token(TokenKind::PlusPlus)?
            | self.match_token(TokenKind::MinusMinus)?
            | self.match_token(TokenKind::Minus)?
        {
            let line: usize = self.previous().line;

            let op: &TokenKind = &self.previous().kind;
            let mut value: Instruction<'instr> = self.primary()?;

            if let Instruction::Integer(_, _, is_signed) = &mut value {
                if *op == TokenKind::Minus {
                    *is_signed = true;
                    return Ok(value);
                }
            }

            let value_type: &DataTypes = &value.get_data_type();

            type_checking::check_unary_instr(op, value_type, self.previous().line)?;

            return Ok(Instruction::Unary {
                op,
                value: Box::from(value),
                kind: *value_type,
                line,
            });
        }

        let instr: Instruction<'_> = self.primary()?;

        Ok(instr)
    }

    fn primary(&mut self) -> Result<Instruction<'instr>, ThrushError> {
        let primary: Instruction = match &self.peek().kind {
            TokenKind::LParen => {
                let line: usize = self.peek().line;

                self.only_advance()?;

                let instr: Instruction<'instr> = self.expression()?;
                let kind: DataTypes = instr.get_data_type();

                if !instr.is_binary() {
                    self.errors.push(ThrushError::Parse(
                        ThrushErrorKind::SyntaxError,
                        String::from("Syntax Error"),
                        String::from(
                            "Group the expressions \"(...)\" is only allowed if contain binary expressions.",
                        ),
                        line,
                    ));
                }

                self.consume(
                    TokenKind::RParen,
                    ThrushErrorKind::SyntaxError,
                    String::from("Syntax Error"),
                    String::from("Expected ')'."),
                    line,
                )?;

                return Ok(Instruction::Group {
                    instr: Box::new(instr),
                    kind,
                });
            }

            TokenKind::String => {
                let current: &Token = self.advance()?;

                Instruction::String(
                    current.lexeme.as_ref().unwrap().to_string(),
                    current.lexeme.as_ref().unwrap().contains("{}"),
                )
            }
            TokenKind::Char => {
                Instruction::Char(self.advance()?.lexeme.as_ref().unwrap().as_bytes()[0])
            }

            kind => match kind {
                TokenKind::Integer(kind, num, is_signed) => {
                    self.only_advance()?;

                    let instr: Instruction<'instr> = match kind {
                        DataTypes::I8 => Instruction::Integer(DataTypes::I8, *num, *is_signed),
                        DataTypes::I16 => Instruction::Integer(DataTypes::I16, *num, *is_signed),
                        DataTypes::I32 => Instruction::Integer(DataTypes::I32, *num, *is_signed),
                        DataTypes::I64 => Instruction::Integer(DataTypes::I64, *num, *is_signed),
                        _ => unreachable!(),
                    };

                    if self.match_token(TokenKind::PlusPlus)?
                        | self.match_token(TokenKind::MinusMinus)?
                    {
                        type_checking::check_unary_instr(
                            &self.previous().kind,
                            kind,
                            self.previous().line,
                        )?;

                        return Ok(Instruction::Unary {
                            op: &self.previous().kind,
                            value: Box::from(instr),
                            kind: *kind,
                            line: self.previous().line,
                        });
                    }

                    instr
                }

                TokenKind::Float(kind, num, is_signed) => {
                    self.only_advance()?;

                    let instr: Instruction<'instr> = match kind {
                        DataTypes::F32 => Instruction::Float(DataTypes::F32, *num, *is_signed),
                        DataTypes::F64 => Instruction::Float(DataTypes::F64, *num, *is_signed),
                        _ => unreachable!(),
                    };

                    if self.match_token(TokenKind::PlusPlus)?
                        | self.match_token(TokenKind::MinusMinus)?
                    {
                        type_checking::check_unary_instr(
                            &self.previous().kind,
                            kind,
                            self.previous().line,
                        )?;

                        return Ok(Instruction::Unary {
                            op: &self.previous().kind,
                            value: Box::from(instr),
                            kind: *kind,
                            line: self.previous().line,
                        });
                    }

                    instr
                }

                TokenKind::Identifier => {
                    let current: &Token = self.peek();
                    let line: usize = self.peek().line;

                    // type is_null, is_function, ?params
                    let var: (DataTypes, bool, bool, Vec<DataTypes>) =
                        self.get_object(current.lexeme.as_ref().unwrap())?;

                    let name: &str = current.lexeme.as_ref().unwrap();

                    self.only_advance()?;

                    if self.peek().kind == TokenKind::LeftBracket {
                        self.consume(
                            TokenKind::LeftBracket,
                            ThrushErrorKind::SyntaxError,
                            String::from("Syntax Error"),
                            String::from("Expected '['."),
                            line,
                        )?;

                        let expr: Instruction<'instr> = self.primary()?;

                        self.consume(
                            TokenKind::RightBracket,
                            ThrushErrorKind::SyntaxError,
                            String::from("Syntax Error"),
                            String::from("Expected ']'."),
                            line,
                        )?;

                        if var.1 {
                            self.errors.push(ThrushError::Parse(
                                ThrushErrorKind::VariableNotDeclared,
                                String::from("Variable Not Declared"),
                                format!(
                                    "Variable `{}` is not declared for are use it. Declare the variable before of the use.",
                                    self.previous().lexeme.as_ref().unwrap(),
                                ),
                                line,
                            ));
                        }

                        let kind: DataTypes = if var.0 == DataTypes::String {
                            DataTypes::Char
                        } else {
                            todo!()
                        };

                        if let Instruction::Integer(_, num, _) = expr {
                            return Ok(Instruction::Indexe {
                                origin: name,
                                index: num as u64,
                                kind,
                            });
                        }

                        self.errors.push(ThrushError::Parse(
                            ThrushErrorKind::SyntaxError,
                            String::from("Syntax Error"),
                            String::from("Expected unsigned number for the build an indexe."),
                            self.previous().line,
                        ));
                    } else if self.peek().kind == TokenKind::Eq {
                        self.only_advance()?;

                        let expr: Instruction<'instr> = self.expression()?;

                        if let Err(err) = type_checking::check_type(
                            expr.get_data_type(),
                            var.0,
                            line,
                            String::from("Type Mismatch"),
                            format!(
                                "Type mismatch. Expected '{}' but found '{}'.",
                                var.0,
                                expr.get_data_type()
                            ),
                        ) {
                            self.errors.push(err);
                        }

                        self.locals[self.scope].insert(name, (var.0, false, false, false, 0));

                        self.consume(
                            TokenKind::SemiColon,
                            ThrushErrorKind::SyntaxError,
                            String::from("Syntax Error"),
                            String::from("Expected ';'."),
                            line,
                        )?;

                        return Ok(Instruction::MutVar {
                            name,
                            value: Box::new(expr),
                            kind: var.0,
                        });
                    } else if self.peek().kind == TokenKind::LParen {
                        self.only_advance()?;

                        return self.call(name, var, line);
                    }

                    if var.1 {
                        self.errors.push(ThrushError::Parse(
                            ThrushErrorKind::VariableNotDeclared,
                            String::from("Variable Not Declared"),
                            format!(
                                "Variable `{}` is not declared for are use it. Declare the variable before of the use.",
                                name,
                            ),
                            line
                        ));
                    }

                    let refvar: Instruction<'_> = Instruction::RefVar {
                        name,
                        line,
                        kind: var.0,
                    };

                    if self.match_token(TokenKind::PlusPlus)?
                        | self.match_token(TokenKind::MinusMinus)?
                    {
                        type_checking::check_unary_instr(
                            &current.kind,
                            &refvar.get_data_type(),
                            line,
                        )?;

                        let expr: Instruction<'_> = Instruction::Unary {
                            op: &current.kind,
                            value: Box::from(refvar),
                            kind: DataTypes::I64,
                            line,
                        };

                        self.consume(
                            TokenKind::SemiColon,
                            ThrushErrorKind::SyntaxError,
                            String::from("Syntax Error"),
                            String::from("Expected ';'."),
                            line,
                        )?;

                        return Ok(expr);
                    }

                    refvar
                }

                TokenKind::True => {
                    self.only_advance()?;
                    Instruction::Boolean(true)
                }

                TokenKind::False => {
                    self.only_advance()?;
                    Instruction::Boolean(false)
                }

                _ => {
                    self.only_advance()?;

                    self.errors.push(ThrushError::Parse(
                        ThrushErrorKind::SyntaxError,
                        String::from("Syntax Error"),
                        format!(
                            "Statement `{}` don't allowed.",
                            self.previous().lexeme.as_ref().unwrap(),
                        ),
                        self.previous().line,
                    ));

                    Instruction::Null
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
        line: usize,
    ) -> Result<&'instr Token, ThrushError> {
        if self.peek().kind == kind {
            return self.advance();
        }

        Err(ThrushError::Parse(error_kind, error_title, help, line))
    }

    fn call(
        &mut self,
        name: &'instr str,
        object: (DataTypes, bool, bool, Vec<DataTypes>),
        line: usize,
    ) -> Result<Instruction<'instr>, ThrushError> {
        if !object.2 {
            return Err(ThrushError::Parse(
                ThrushErrorKind::SyntaxError,
                String::from("Syntax Error"),
                String::from("The object called is don't a function. Call only functions."),
                line,
            ));
        }

        let mut args: Vec<Instruction<'instr>> = Vec::new();

        while self.peek().kind != TokenKind::RParen {
            if self.match_token(TokenKind::Comma)? {
                continue;
            }

            args.push(self.expression()?);
        }

        self.consume(
            TokenKind::RParen,
            ThrushErrorKind::SyntaxError,
            String::from("Syntax Error"),
            String::from("Expected ')'."),
            line,
        )?;

        if object.3.len() != args.len() {
            let args_types: String = if !args.is_empty() {
                args.iter()
                    .map(|param| param.get_data_type().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            } else {
                DataTypes::Void.to_string()
            };

            self.errors.push(ThrushError::Parse(
                ThrushErrorKind::SyntaxError,
                String::from("Syntax Error"),
                format!(
                    "Function called expected all arguments with types '{}' don't '{}'.",
                    object
                        .3
                        .iter()
                        .map(|param| param.to_string())
                        .collect::<Vec<_>>()
                        .join(", "),
                    args_types,
                ),
                line,
            ));
        }

        let mut index: usize = 0;

        args.iter().for_each(|arg| {
            let arg_kind: DataTypes = arg.get_data_type();

            if object.3.len() >= index && object.3[index] != arg_kind {
                self.errors.push(ThrushError::Parse(
                    ThrushErrorKind::SyntaxError,
                    String::from("Syntax Error"),
                    format!(
                        "Function called, expected '{}' argument type in position {} don't '{}' type.",
                        object.3[index], index, arg_kind
                    ),
                    line,
                ));
            }

            index += 1;
        });

        Ok(Instruction::Call {
            name,
            args,
            kind: object.0,
        })
    }

    fn parse_string_formatted(&mut self, args: &[Instruction], line: usize, scan_spaces: bool) {
        if args.is_empty() {
            self.errors.push(ThrushError::Parse(
                ThrushErrorKind::SyntaxError,
                String::from("Syntax Error"),
                String::from(
                    "Expected at least 1 argument for 'println' call. Like 'println(\"Hi!\");'",
                ),
                line,
            ));
        } else if let Instruction::String(str, _) = &args[0] {
            let mut formats: usize = 0;

            str.split_inclusive("{}").for_each(|substr| {
                if substr.contains("{}") {
                    formats += 1;
                }
            });

            if formats != args.iter().skip(1).collect::<Vec<_>>().len() {
                self.errors.push(ThrushError::Parse(
                    ThrushErrorKind::SyntaxError,
                    String::from("Expected format"),
                    String::from("Missing format for argument or an argument. Should be like this println(\"{}\", arguments.size() == formatters.size());"),
                    line,
                ));
            }
        }

        if scan_spaces {
            args.iter().for_each(|arg| {
                if let Instruction::String(str, _) = arg {
                    if str.contains("\n") {
                        self.errors.push(ThrushError::Parse(
                            ThrushErrorKind::SyntaxError,
                            String::from("Syntax Error"),
                            String::from(
                                "You can't print strings that contain newlines. Use 'println' instead.",
                            ),
                            self.peek().line,
                        ));
                    }
                }
            });
        }
    }
    #[inline]
    fn get_object(&mut self, name: &'instr str) -> GetObjectResult {
        for scope in self.locals.iter_mut().rev() {
            if scope.contains_key(name) {
                // DataTypes, bool <- (is_null), bool <- (is_freeded), usize <- (number of references)
                let mut var: (DataTypes, bool, bool, bool, usize) = *scope.get(name).unwrap();

                var.4 += 1;

                scope.insert(name, var);

                return Ok((var.0, var.1, false, Vec::new()));
            }
        }

        if self.globals.contains_key(name) {
            let global: &(DataTypes, bool, Vec<DataTypes>) = self.globals.get(name).unwrap();

            let mut possible_params: Vec<DataTypes> = Vec::new();

            possible_params.clone_from(&global.2);

            // type, //is null, //is_function, //params
            return Ok((global.0, false, global.1, possible_params));
        }

        Err(ThrushError::Parse(
            ThrushErrorKind::ObjectNotDefined,
            String::from("Object Not Defined"),
            format!(
                "The object with name `{}` is not defined in this scope or global.",
                name
            ),
            self.previous().line,
        ))
    }

    fn predefine_functions(&mut self) {
        let mut functions_positions: Vec<usize> = Vec::new();
        let mut current_pos: usize = 0;

        self.tokens.iter().for_each(|tok| match tok.kind {
            TokenKind::Fn => {
                functions_positions.push(current_pos);

                current_pos += 1;
            }
            _ => {
                current_pos += 1;
            }
        });

        functions_positions.iter().for_each(|index| {
            let _ = self.predefine_function(*index);
        });
    }

    fn predefine_function(&mut self, index: usize) -> Result<(), ThrushError> {
        self.current = index;

        self.only_advance()?;

        let name: &Token = self.consume(
            TokenKind::Identifier,
            ThrushErrorKind::SyntaxError,
            String::from("Expected function name"),
            String::from("Expected fn < name >."),
            self.previous().line,
        )?;

        self.consume(
            TokenKind::LParen,
            ThrushErrorKind::SyntaxError,
            String::from("Syntax Error"),
            String::from("Expected '('."),
            name.line,
        )?;

        let mut params: Vec<DataTypes> = Vec::new();

        while !self.match_token(TokenKind::RParen)? {
            if self.match_token(TokenKind::Comma)? {
                continue;
            }

            if !self.match_token(TokenKind::Identifier)? {
                self.errors.push(ThrushError::Parse(
                    ThrushErrorKind::SyntaxError,
                    String::from("Syntax Error"),
                    String::from("Expected argument name."),
                    name.line,
                ));
            }

            if !self.match_token(TokenKind::ColonColon)? {
                self.errors.push(ThrushError::Parse(
                    ThrushErrorKind::SyntaxError,
                    String::from("Syntax Error"),
                    String::from("Expected '::'."),
                    name.line,
                ));
            }

            let kind: DataTypes = match &self.peek().kind {
                TokenKind::DataType(kind) => {
                    self.only_advance()?;

                    *kind
                }
                _ => {
                    self.errors.push(ThrushError::Parse(
                        ThrushErrorKind::SyntaxError,
                        String::from("Syntax Error"),
                        String::from("Expected argument type."),
                        name.line,
                    ));

                    DataTypes::Void
                }
            };

            params.push(kind)
        }

        if self.peek().kind == TokenKind::Colon {
            self.consume(
                TokenKind::Colon,
                ThrushErrorKind::SyntaxError,
                String::from("Syntax Error"),
                String::from("Missing return type. Expected ':' followed by return type."),
                name.line,
            )?;
        }

        let return_kind: Option<DataTypes> = match &self.peek().kind {
            TokenKind::DataType(kind) => {
                self.only_advance()?;
                Some(*kind)
            }
            _ => None,
        };

        self.current = 0;

        if let Some(kind) = &return_kind {
            self.define_global(name.lexeme.as_ref().unwrap(), *kind, true, params);

            return Ok(());
        }

        self.define_global(name.lexeme.as_ref().unwrap(), DataTypes::Void, true, params);

        Ok(())
    }

    fn emit_deallocators(&mut self) -> Vec<Instruction<'instr>> {
        let mut frees: Vec<Instruction> = Vec::new();

        for stmt in self.locals[self.scope].iter_mut() {
            if let (_, (DataTypes::String, false, false, free_only, 0)) = stmt {
                frees.push(Instruction::Free {
                    name: stmt.0,
                    is_string: true,
                    free_only: *free_only,
                });

                stmt.1 .2 = true;
            }
        }

        frees
    }

    #[inline]
    fn modify_deallocation(&mut self, name: &'instr str, free_only: bool, freeded: bool) {
        for scope in self.locals.iter_mut().rev() {
            if scope.contains_key(name) {
                // DataTypes, bool <- (is_null), bool <- (is_freeded), bool <- (free_only), usize <- (number of references)

                let mut var: (DataTypes, bool, bool, bool, usize) = *scope.get(name).unwrap();

                var.2 = freeded;
                var.3 = free_only;

                scope.insert(name, var);

                return;
            }
        }
    }

    #[inline]
    fn define_global(
        &mut self,
        name: &'instr str,
        kind: DataTypes,
        is_function: bool,
        params: Vec<DataTypes>,
    ) {
        self.globals.insert(name, (kind, is_function, params));
    }

    #[inline]
    fn define_local(&mut self, name: &'instr str, value: (DataTypes, bool, bool, bool, usize)) {
        self.locals[self.scope].insert(name, value);
    }

    #[inline]
    fn begin_scope(&mut self) {
        self.scope += 1;
        self.locals.push(HashMap::new());
    }

    #[inline]
    fn end_scope(&mut self) {
        self.scope -= 1;
        self.locals.pop();
    }

    fn match_token(&mut self, kind: TokenKind) -> Result<bool, ThrushError> {
        if self.end() {
            return Ok(false);
        } else if self.peek().kind == kind {
            self.only_advance()?;

            return Ok(true);
        }

        Ok(false)
    }

    fn only_advance(&mut self) -> Result<(), ThrushError> {
        if !self.end() {
            self.current += 1;
            return Ok(());
        }

        Err(ThrushError::Parse(
            ThrushErrorKind::SyntaxError,
            String::from("Undeterminated Code"),
            String::from("The code has ended abruptly and without any order, review the code and write the syntax correctly."),

            self.previous().line,
        ))
    }

    fn advance(&mut self) -> Result<&'instr Token, ThrushError> {
        if !self.end() {
            self.current += 1;
            return Ok(self.previous());
        }

        Err(ThrushError::Parse(
            ThrushErrorKind::SyntaxError,
            String::from("Undeterminated Code"),
            String::from("The code has ended abruptly and without any order, review the code and write the syntax correctly."),

            self.previous().line,
        ))
    }

    fn sync(&mut self) {
        while !self.end() {
            match self.peek().kind {
                TokenKind::Var | TokenKind::Fn => return,
                _ => {}
            }

            self.current += 1;
        }
    }

    #[inline]
    fn peek(&self) -> &'instr Token {
        &self.tokens[self.current]
    }

    #[inline]
    fn previous(&self) -> &'instr Token {
        &self.tokens[self.current - 1]
    }

    #[inline]
    fn end(&self) -> bool {
        self.peek().kind == TokenKind::Eof
    }
}
