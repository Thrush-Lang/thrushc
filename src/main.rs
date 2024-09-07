mod backend;
mod constants;
mod diagnostic;
mod error;
mod frontend;
mod logging;

use {
    backend::compiler::{Compiler, FileBuilder, Instruction, Linking, Optimization, Options},
    colored::Colorize,
    constants::TARGETS,
    frontend::{
        lexer::{Lexer, Token},
        parser::Parser,
    },
    inkwell::{
        builder::Builder,
        context::Context,
        module::Module,
        targets::{InitializationConfig, Target, TargetMachine, TargetTriple},
    },
    logging::Logging,
    std::{env, fs::read_to_string, path::Path, sync::Mutex},
};

pub static FILE_NAME_WITH_EXT: Mutex<String> = Mutex::new(String::new());
pub static FILE_PATH: Mutex<String> = Mutex::new(String::new());

/*

HOW TO FIX NUMBER PRINT ERROR

; ModuleID = 'main'
source_filename = "main"
target triple = "x86_64-pc-linux-gnu"

@fmt = private unnamed_addr constant [5 x i8] c"%d\00A\00", align 1

declare i32 @printf(ptr, ...)

define i32 @main() {
  %1 = alloca i8, align 4

  store i8 10, ptr %1, align 4

  %val = load i8, ptr %1, align 4

  %fmt_ptr = bitcast [5 x i8]* @fmt to ptr

  call i32 @printf(ptr %fmt_ptr, i8 %val)

  ret i32 0
}

*/

fn main() {
    let mut parameters: Vec<String> = env::args().collect();

    parameters.remove(0);

    let mut options: Options = Options::default();
    let mut compile: bool = false;

    for parameter in parameters.iter() {
        match parameter.as_str() {
            "-h" | "--help" => {
                help();
                break;
            }

            "-t" | "targets" => {
                TARGETS.iter().for_each(|target| {
                    println!("{}", target.bold());
                });
                break;
            }

            "-nt" | "native-target" => {
                println!("{}", TargetMachine::get_default_triple().to_string().bold());
                break;
            }

            "-v" | "--version" => {
                println!("{}", env!("CARGO_PKG_VERSION").bold());
                break;
            }

            "-c" | "compile" => {
                if parameters.len() == 1 {
                    compile_help();
                    return;
                }

                let index: usize = parameters.len() - 1;
                let path: &Path = Path::new(&parameters[index]);

                if !path.exists() {
                    Logging::new(format!(
                        "The path or file '{}' cannot be accessed.",
                        &parameters[index]
                    ))
                    .error();
                    return;
                }

                if !path.is_file() {
                    Logging::new(format!(
                        "The path '{}' ended with not a file.",
                        &parameters[index]
                    ))
                    .error();
                    return;
                }

                if path.extension().is_none() {
                    Logging::new(format!(
                        "The file in path '{}' does not have an extension.",
                        &parameters[index]
                    ))
                    .error();
                    return;
                }

                if path.extension().unwrap() != "th" {
                    Logging::new(format!(
                        "The file in path '{}' does not have the extension '.th'.",
                        &parameters[index]
                    ))
                    .error();
                    return;
                }

                for i in 1..parameters.len() - 1 {
                    match parameters[i].as_str() {
                        "--name" | "-n" => {
                            options.name = parameters[index].clone();
                        }
                        "--target" | "-t" => {
                            if TARGETS.contains(&parameters[i + 1].as_str()) {
                                options.target_triple = TargetTriple::create(&parameters[i + 1]);

                                continue;
                            }

                            Logging::new(format!(
                                "The target '{}' is not supported, see the list with Thrushr --print-targets.",
                                &parameters[index]
                            ))
                            .error();
                            return;
                        }
                        "--optimization" | "-opt" => match parameters[i + 1].as_str() {
                            "none" => {
                                options.optimization = Optimization::None;
                            }
                            "low" => {
                                options.optimization = Optimization::Low;
                            }
                            "mid" => {
                                options.optimization = Optimization::Mid;
                            }
                            "mcqueen" => {
                                options.optimization = Optimization::Mcqueen;
                            }
                            _ => {
                                options.optimization = Optimization::None;
                            }
                        },
                        "--emit-llvm" | "-emit-llvm" => {
                            options.emit_llvm = true;
                        }
                        "--static" | "-s" => {
                            options.linking = Linking::Static;
                        }
                        "--dynamic" | "-d" => {
                            options.linking = Linking::Dynamic;
                        }
                        "--build" | "-b" => {
                            options.build = true;
                        }

                        _ => continue,
                    }
                }

                FILE_NAME_WITH_EXT.lock().unwrap().push_str(
                    Path::new(&parameters[index])
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap(),
                );

                options.path = Path::new(&parameters[index]).to_path_buf();

                compile = true;

                break;
            }

            "-i" | "interpret" => {}

            _ => help(),
        }
    }

    if &options.name == "main" {
        options.name = FILE_NAME_WITH_EXT
            .lock()
            .unwrap()
            .as_str()
            .split(".")
            .next()
            .unwrap()
            .to_string();
    }

    match &options.name == "main" {
        true => {
            options.is_main = true;
        }
        false => {
            options.is_main = false;
        }
    }

    let extract_content: String = read_to_string(&options.path).unwrap_or_else(|error| {
        Logging::new(error.to_string()).error();
        panic!()
    });

    let content: &[u8] = extract_content.as_bytes();

    let mut lexer: Lexer = Lexer::new(content);
    let mut parser: Parser = Parser::new();

    Target::initialize_all(&InitializationConfig::default());

    let context: Context = Context::create();
    let builder: Builder<'_> = context.create_builder();
    let module: Module<'_> = context.create_module(&options.name);

    let tokens: Result<&[Token], String> = lexer.lex();

    match tokens {
        Ok(tokens) => {
            parser.tokens = Some(tokens);
            parser.options = Some(&options);

            let instructions: Result<&[Instruction<'_>], String> = parser.start();

            match instructions {
                Ok(instructions) => {
                    Compiler::compile(&module, &builder, &context, instructions);
                }

                Err(msg) => {
                    Logging::new(msg).error();
                }
            }
        }

        Err(msg) => {
            Logging::new(msg).error();
        }
    }

    module.set_triple(&options.target_triple);
    module.strip_debug_info();

    if !compile {
        todo!()
    }

    FileBuilder::new(&options, &module).build();
}

fn help() {
    println!(
        "\n{} {}\n",
        "Thrush".bright_cyan().bold(),
        "Programming Language".bold()
    );

    println!(
        "{} {} {}\n",
        "The".bold(),
        "Bootstrap Compiler"
            .bright_cyan()
            .bold()
            .italic()
            .underline(),
        "for the Thrush programming language.".bold(),
    );

    println!(
        "{} {} {}\n",
        "Usage:".bold(),
        "thrush".bright_cyan().bold(),
        "[--flags]".bold()
    );

    println!("{}", "Available Commands:\n".bold());

    println!(
        "{} ({} | {}) {}",
        "•".bold(),
        "help".bold().bright_cyan(),
        "-h".bold().bright_cyan(),
        "Show help message.".bold()
    );

    println!(
        "{} ({} | {}) {}",
        "•".bold(),
        "version".bold().bright_cyan(),
        "-v".bold().bright_cyan(),
        "Show the version.".bold()
    );

    println!(
        "{} ({} | {}) {}",
        "•".bold(),
        "interpret [--flags] [file]".bold().bright_cyan(),
        "-i [--flags] [file]".bold().bright_cyan(),
        "Interpret the code provided using the JIT compiler.".bold()
    );

    println!(
        "{} ({} | {}) {}",
        "•".bold(),
        "compile [--flags] [file]".bold().bright_cyan(),
        "-c [--flags] [file]".bold().bright_cyan(),
        "Compile the code provided into executable.".bold()
    );

    println!(
        "{} ({} | {}) {}",
        "•".bold(),
        "targets".bold().bright_cyan(),
        "-t".bold().bright_cyan(),
        "Print the list of supported targets machines.".bold()
    );

    println!(
        "{} ({} | {}) {}",
        "•".bold(),
        "native-target".bold().bright_cyan(),
        "-nt".bold().bright_cyan(),
        "Print the native target of this machine.".bold()
    )
}

fn compile_help() {
    println!(
        "{} {} {} {}\n",
        "Usage:".bold(),
        "thrush".bold().bright_cyan(),
        "compile".bold(),
        "[--flags values]".bold().bright_cyan()
    );

    println!("{}", "Available Flags:\n".bold());

    println!(
        "{} ({} | {}) {}",
        "•".bold(),
        "--name [name]".bold().bright_cyan(),
        "-n [name]".bold().bright_cyan(),
        "Name of the executable (Compiler dispatches it).".bold()
    );

    println!(
        "{} ({} | {}) {}",
        "•".bold(),
        "--target [x86_64]".bold().bright_cyan(),
        "-t [x86_64]".bold().bright_cyan(),
        "Target architecture for the Compiler or JIT compiler.".bold()
    );

    println!(
        "{} ({} | {}) {}",
        "•".bold(),
        "--optimization [opt-level]".bold().bright_cyan(),
        "-opt [opt-level]".bold().bright_cyan(),
        "Optimization level for the JIT compiler or the Compiler.".bold()
    );

    println!(
        "{} ({} | {}) {}",
        "•".bold(),
        "--emit-llvm".bold().bright_cyan(),
        "-emit-llvm".bold().bright_cyan(),
        "Compile the code to LLVM IR.".bold()
    );

    println!(
        "{} ({} | {}) {}",
        "•".bold(),
        "--static".bold().bright_cyan(),
        "-s".bold().bright_cyan(),
        "Link the executable statically.".bold()
    );

    println!(
        "{} ({} | {}) {}",
        "•".bold(),
        "--dynamic".bold().bright_cyan(),
        "-s".bold().bright_cyan(),
        "Link the executable dynamically.".bold()
    );

    println!(
        "{} ({} | {}) {}",
        "•".bold(),
        "--build".bold().bright_cyan(),
        "-b".bold().bright_cyan(),
        "Compile the code into executable.".bold()
    );
}
