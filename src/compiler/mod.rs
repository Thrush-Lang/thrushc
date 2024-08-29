pub mod backend;
pub mod frontend;

use {
    super::Logging,
    backend::codegen::CodeGen,
    frontend::{
        lexer::{Lexer, Token, TokenSpan},
        parser::Parser,
    },
    inkwell::{
        context::Context,
        targets::{TargetMachine, TargetTriple},
    },
    std::{fs::read_to_string, mem, path::PathBuf, sync::Mutex},
};

pub static FILE_NAME_WITH_EXT: Mutex<String> = Mutex::new(String::new());
pub static FILE_PATH: Mutex<String> = Mutex::new(String::new());

#[derive(Default, Clone)]
pub struct ThrushFile {
    pub is_main: bool,
    pub name: String,
    pub path: PathBuf,
}

pub struct Compiler {
    file: ThrushFile,
    options: CompilerOptions,
}

#[derive(Default)]
pub enum ThrushError {
    Compile(String),
    Parse(ThrushErrorKind, String, String, String, TokenSpan, usize),
    Lex(ThrushErrorKind, String, String, String, TokenSpan, usize),
    #[default]
    None,
}

pub enum ThrushErrorKind {
    SyntaxError,
    UnreachableNumber,
    ParsedNumber,
    UnknownChar,
}

#[derive(Default, Debug)]
pub enum OptimizationLevel {
    #[default]
    None,
    Low,
    Mid,
    Mcqueen,
}

#[derive(Debug)]
pub struct CompilerOptions {
    pub name: String,
    pub target: TargetTriple,
    pub optimization: OptimizationLevel,
    pub interpret: bool,
    pub emit_llvm: bool,
    pub build: bool,
}

impl Default for CompilerOptions {
    fn default() -> Self {
        Self {
            name: String::from("main"),
            target: TargetMachine::get_default_triple(),
            optimization: OptimizationLevel::default(),
            interpret: false,
            emit_llvm: false,
            build: false,
        }
    }
}

impl Compiler {
    pub fn new(options: CompilerOptions, file: ThrushFile) -> Self {
        Self { options, file }
    }

    pub fn compile(&mut self) {
        if let Ok(content) = read_to_string(mem::take(&mut self.file.path)) {
            let mut lexer: Lexer = Lexer::new(content.as_bytes());
            let tokens: Result<&[Token], ThrushError> = lexer.lex();

            match tokens {
                Ok(tokens) => {
                    let mut parser: Parser = Parser::new(tokens, self.file.clone());

                    match parser.parse() {
                        Ok(instructions) => {
                            let context: Context = Context::create();

                            CodeGen::new(
                                &context,
                                context.create_module(&self.options.name),
                                context.create_builder(),
                                instructions,
                                mem::take(&mut self.file),
                                mem::take(&mut self.options),
                            )
                            .compile();
                        }

                        Err(error) => {
                            if let ThrushError::Compile(error) = error {
                                Logging::new(error).error();
                            }
                        }
                    }
                }

                Err(error) => {
                    if let ThrushError::Compile(error) = error {
                        Logging::new(error).error();
                    }
                }
            }
        }
    }
}
