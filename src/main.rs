mod backend;
mod cli;
mod constants;
mod diagnostic;
mod error;
mod frontend;
mod logging;
mod utils;

use {
    backend::{
        builder::FileBuilder,
        compiler::{Compiler, CompilerOptions, Linking, Opt},
        infraestructures::vector::VectorAPI,
        instruction::Instruction,
    },
    cli::CLIParser,
    constants::TARGETS,
    frontend::{
        lexer::{Lexer, Token},
        parser::Parser,
    },
    inkwell::{
        builder::Builder,
        context::Context,
        module::Module,
        targets::{
            CodeModel, InitializationConfig, RelocMode, Target, TargetMachine, TargetTriple,
        },
        OptimizationLevel,
    },
    std::{env, fs::read_to_string, option, path::Path, sync::Mutex, time::Instant},
    stylic::{style, Color, Stylize},
};

pub static NAME: Mutex<String> = Mutex::new(String::new());
pub static PATH: Mutex<String> = Mutex::new(String::new());
pub static BACKEND_COMPILER: Mutex<String> = Mutex::new(String::new());

fn main() {
    Target::initialize_native(&InitializationConfig::default()).unwrap();

    let mut cli: CLIParser = CLIParser::new(env::args().collect());
    cli.parse();

    if BACKEND_COMPILER.lock().unwrap().is_empty() {
        logging::log(logging::LogType::ERROR, "The Backend Compiler is not set.");
        return;
    }

    if cli.compiler_options.restore_natives_apis {
        let context: Context = Context::create();
        let builder: Builder<'_> = context.create_builder();

        cli.compiler_options.output = if cli.compiler_options.restore_vector_natives {
            "vector.o".to_string()
        } else {
            "native".to_string()
        };

        let module: Module<'_> = context.create_module(&cli.compiler_options.output);

        module.set_triple(&cli.compiler_options.target_triple);

        let opt: OptimizationLevel = cli.compiler_options.optimization.to_llvm_opt();

        let machine: TargetMachine = Target::from_triple(&cli.compiler_options.target_triple)
            .unwrap()
            .create_target_machine(
                &cli.compiler_options.target_triple,
                "",
                "",
                opt,
                cli.compiler_options.reloc_mode,
                cli.compiler_options.code_model,
            )
            .unwrap();

        module.set_data_layout(&machine.get_target_data().get_data_layout());

        if cli.compiler_options.restore_vector_natives {
            VectorAPI::include(&module, &builder, &context);
        }

        if cli.compiler_options.emit_llvm {
            module
                .print_to_file(format!("{}.ll", cli.compiler_options.output))
                .unwrap();
            return;
        }

        FileBuilder::new(&cli.compiler_options, &module).build();

        return;
    }

    println!(
        "\n{} {}",
        style("Compiling").bold().fg(Color::Rgb(141, 141, 142)),
        PATH.lock().unwrap()
    );

    let origin_content: String =
        read_to_string(PATH.lock().unwrap().as_str()).unwrap_or_else(|error| {
            logging::log(logging::LogType::ERROR, error.to_string().as_str());
            panic!()
        });

    let content: &[u8] = origin_content.as_bytes();

    let mut lexer: Lexer = Lexer::new(content);
    let mut parser: Parser = Parser::new();

    let context: Context = Context::create();
    let builder: Builder<'_> = context.create_builder();
    let module: Module<'_> = context.create_module(&cli.compiler_options.output);

    let start_time: Instant = Instant::now();

    let tokens: Result<&[Token], String> = lexer.lex();

    match tokens {
        Ok(tokens) => {
            parser.tokens = Some(tokens);
            parser.options = Some(&cli.compiler_options);

            let instructions: Result<&[Instruction<'_>], String> = parser.start();

            match instructions {
                Ok(instructions) => {
                    module.set_triple(&cli.compiler_options.target_triple);

                    let opt: OptimizationLevel = cli.compiler_options.optimization.to_llvm_opt();

                    let machine: TargetMachine =
                        Target::from_triple(&cli.compiler_options.target_triple)
                            .unwrap()
                            .create_target_machine(
                                &cli.compiler_options.target_triple,
                                "",
                                "",
                                opt,
                                cli.compiler_options.reloc_mode,
                                cli.compiler_options.code_model,
                            )
                            .unwrap();

                    module.set_data_layout(&machine.get_target_data().get_data_layout());

                    Compiler::compile(
                        &module,
                        &builder,
                        &context,
                        &cli.compiler_options,
                        instructions,
                    );

                    FileBuilder::new(&cli.compiler_options, &module).build();
                }

                Err(msg) => {
                    logging::log(logging::LogType::ERROR, &msg);
                }
            }
        }

        Err(msg) => {
            logging::log(logging::LogType::ERROR, &msg);
        }
    }

    println!(
        "{} {} {}",
        style("Finished").bold().fg(Color::Rgb(141, 141, 142)),
        PATH.lock().unwrap(),
        style(&format!(
            "{}.{}s",
            start_time.elapsed().as_secs(),
            start_time.elapsed().as_millis()
        ))
        .bold()
        .fg(Color::Rgb(141, 141, 142))
    );
}
