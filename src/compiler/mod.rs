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
        targets::{InitializationConfig, Target, TargetMachine, TargetTriple},
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
    TooManyArguments,
    SyntaxError,
    UnreachableNumber,
    ParsedNumber,
    UnknownChar,
}

#[derive(Default, Debug)]
pub enum Optimization {
    #[default]
    None,
    Low,
    Mid,
    Mcqueen,
}

#[derive(Default, Debug)]
pub enum Linking {
    #[default]
    Static,
    Dynamic,
}

#[derive(Debug)]
pub struct CompilerOptions {
    pub name: String,
    pub target_triple: TargetTriple,
    pub optimization: Optimization,
    pub interpret: bool,
    pub emit_llvm: bool,
    pub build: bool,
    pub linking: Linking,
}

impl Default for CompilerOptions {
    fn default() -> Self {
        Self {
            name: String::from("main"),
            target_triple: TargetMachine::get_default_triple(),
            optimization: Optimization::default(),
            interpret: false,
            emit_llvm: false,
            build: false,
            linking: Linking::default(),
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
                    match parser.start() {
                        Ok(instructions) => {
                            Target::initialize_all(&InitializationConfig::default());

                            println!("{:?}", instructions);

                            /* let context: Context = Context::create();

                            CodeGen::new(
                                &context,
                                context.create_module(&self.options.name),
                                context.create_builder(),
                                instructions,
                                mem::take(&mut self.file),
                                mem::take(&mut self.options),
                            )
                            .compile(); */
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
