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
        builder::FileBuilder, compiler::Compiler, infraestructures::vector::VectorAPI,
        instruction::Instruction,
    },
    cli::CLIParser,
    frontend::{
        lexer::{Lexer, Token},
        parser::Parser,
    },
    inkwell::{
        builder::Builder,
        context::Context,
        module::Module,
        targets::{InitializationConfig, Target, TargetMachine},
        OptimizationLevel,
    },
    std::{env, fs::read_to_string, sync::Mutex, time::Instant},
    stylic::{style, Color, Stylize},
};

pub static NAME: Mutex<String> = Mutex::new(String::new());
pub static BACKEND_COMPILER: Mutex<String> = Mutex::new(String::new());

fn main() {
    Target::initialize_native(&InitializationConfig::default()).unwrap();

    let mut cli: CLIParser = CLIParser::new(env::args().collect());
    cli.parse();

    if BACKEND_COMPILER.lock().unwrap().is_empty() {
        logging::log(
            logging::LogType::ERROR,
            "The Backend Compiler is not set. Use the flag `thrushc --backend \"PATH\"` to set it.",
        );

        return;
    }

    if cli.compiler_options.restore_natives_apis {
        let context: Context = Context::create();
        let builder: Builder<'_> = context.create_builder();

        cli.compiler_options.output = if cli.compiler_options.restore_vector_natives {
            "vector.o".to_string()
        } else {
            "native.o".to_string()
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

        FileBuilder::new(
            &cli.compiler_options,
            &module,
            &format!("{}.ll", utils::extract_file_name(&NAME.lock().unwrap())),
        )
        .build();

        return;
    }

    println!(
        "\n{} {}",
        style("Compiling").bold().fg(Color::Rgb(141, 141, 142)),
        cli.compiler_options.file_path
    );

    let content: String = read_to_string(&cli.compiler_options.file_path).unwrap();

    let mut lexer: Lexer = Lexer::new(content.as_bytes(), &cli.compiler_options.file_path);

    let tokens: &[Token] = lexer.lex().unwrap_or_else(|msg| unreachable!());

    let mut parser: Parser = Parser::new(&cli.compiler_options, tokens);

    let instructions: &[Instruction] = parser.start().unwrap_or_else(|msg| unreachable!());

    let context: Context = Context::create();
    let builder: Builder<'_> = context.create_builder();
    let module: Module<'_> = context.create_module(&cli.compiler_options.output);

    let start_time: Instant = Instant::now();

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

    Compiler::compile(
        &module,
        &builder,
        &context,
        &cli.compiler_options,
        instructions,
    );

    FileBuilder::new(
        &cli.compiler_options,
        &module,
        &format!("{}.ll", utils::extract_file_name(&NAME.lock().unwrap())),
    )
    .build();

    println!(
        "{} {} {}",
        style("Finished").bold().fg(Color::Rgb(141, 141, 142)),
        cli.compiler_options.file_path,
        style(&format!(
            "{}.{}s",
            start_time.elapsed().as_secs(),
            start_time.elapsed().as_millis()
        ))
        .bold()
        .fg(Color::Rgb(141, 141, 142))
    );
}
