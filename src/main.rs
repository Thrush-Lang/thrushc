mod backend;
mod cli;
mod constants;
mod diagnostic;
mod error;
mod frontend;
mod logging;

use {
    backend::{
        apis::{debug, vector},
        builder::FileBuilder,
        compiler::Compiler,
        instruction::Instruction,
    },
    cli::Cli,
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
    lazy_static::lazy_static,
    std::{
        env,
        fs::{self, read_to_string},
        path::PathBuf,
        process,
        sync::Mutex,
        time::Instant,
    },
    stylic::{style, Color, Stylize},
};

lazy_static! {
    static ref HOME: Option<PathBuf> = {
        match env::consts::OS {
            "windows" => Some(PathBuf::from(env::var("APPDATA").unwrap())),
            "linux" => Some(PathBuf::from(env::var("HOME").unwrap())),
            _ => None,
        }
    };
    static ref LLVM_BACKEND_COMPILER: Option<PathBuf> = {
        if HOME.is_none() {
            return None;
        }

        if !HOME.as_ref().unwrap().join("thrushlang").exists()
            || !HOME.as_ref().unwrap().join("thrushlang/backends/").exists()
            || !HOME
                .as_ref()
                .unwrap()
                .join("thrushlang/backends/llvm")
                .exists()
            || !HOME
                .as_ref()
                .unwrap()
                .join("thrushlang/backends/llvm/backend")
                .exists()
            || !HOME
                .as_ref()
                .unwrap()
                .join("thrushlang/backends/llvm/backend/bin")
                .exists()
        {
            logging::log(
                logging::LogType::ERROR,
                &format!("LLVM Toolchain was corrupted from Thrush Toolchain, re-install the entire toolchain via \"thorium install {}\".", env::consts::OS),
            );

            return None;
        }

        if !HOME
            .as_ref()
            .unwrap()
            .join("thrushlang/backends/llvm/backend/bin/clang-17")
            .exists()
        {
            logging::log(
                logging::LogType::ERROR,
                &format!("Clang-17 don't exists in Thrush Toolchain, re-install the entire toolchain via \"thorium install {}\".", env::consts::OS),
            );

            return None;
        } else if !HOME
            .as_ref()
            .unwrap()
            .join("thrushlang/backends/llvm/backend/bin/opt")
            .exists()
        {
            logging::log(
                logging::LogType::ERROR,
                &format!("LLVM Optimizator don't exists in Thrush Toolchain, re-install the entire toolchain via \"thorium install {}\".", env::consts::OS),
            );

            return None;
        } else if !HOME
            .as_ref()
            .unwrap()
            .join("thrushlang/backends/llvm/backend/bin/llc")
            .exists()
        {
            logging::log(
                logging::LogType::ERROR,
                &format!("LLVM Static Compiler don't exists in Thrush Toolchain, re-install the entire toolchain via \"thorium install {}\".", env::consts::OS),
            );

            return None;
        } else if !HOME
            .as_ref()
            .unwrap()
            .join("thrushlang/backends/llvm/backend/bin/llvm-config")
            .exists()
        {
            logging::log(
                logging::LogType::ERROR,
                &format!("LLVM Configurator don't exists in Thrush Toolchain, re-install the entire toolchain via \"thorium install {}\".", env::consts::OS),
            );

            return None;
        }

        Some(
            HOME.as_ref()
                .unwrap()
                .join("thrushlang/backends/llvm/backend/bin/"),
        )
    };
}

pub static NAME: Mutex<String> = Mutex::new(String::new());

fn main() {
    if !["linux", "windows"].contains(&env::consts::OS) {
        logging::log(
            logging::LogType::ERROR,
            "Compilation from Unsopported Operating System. Only Linux and Windows are supported.",
        );

        process::exit(1);
    }

    Target::initialize_native(&InitializationConfig::default()).unwrap();

    let mut cli: Cli = Cli::parse(env::args().collect());

    println!(
        "{} {}",
        style("Compiling").bold().fg(Color::Rgb(141, 141, 142)),
        cli.options.file_path
    );

    if !cli.options.include_vector_api {
        vector::append_vector_api(&mut cli.options);
    }

    if !cli.options.include_debug_api {
        debug::append_debug_api(&mut cli.options);
    }

    let content: String = read_to_string(&cli.options.file_path).unwrap();

    let mut lexer: Lexer = Lexer::new(content.as_bytes(), &cli.options.file_path);
    let tokens: &[Token] = lexer.lex();

    let mut parser: Parser = Parser::new(&cli.options, tokens);
    let instructions: &[Instruction] = parser.start();

    let context: Context = Context::create();
    let builder: Builder<'_> = context.create_builder();
    let module: Module<'_> = context.create_module(&NAME.lock().unwrap());

    let start_time: Instant = Instant::now();

    // println!("{:?}", instructions);

    module.set_triple(&cli.options.target_triple);

    let opt: OptimizationLevel = cli.options.optimization.to_llvm_opt();

    let machine: TargetMachine = Target::from_triple(&cli.options.target_triple)
        .unwrap()
        .create_target_machine(
            &cli.options.target_triple,
            "",
            "",
            opt,
            cli.options.reloc_mode,
            cli.options.code_model,
        )
        .unwrap();

    module.set_data_layout(&machine.get_target_data().get_data_layout());

    Compiler::compile(&module, &builder, &context, &cli.options, instructions);

    FileBuilder::new(
        &cli.options,
        &module,
        &format!("{}.ll", &NAME.lock().unwrap()),
        &cli.options.output,
    )
    .build();

    if cli.options.delete_built_in_apis_after {
        let _ = fs::remove_file("vector.o");
        let _ = fs::remove_file("debug.o");
    }

    println!(
        "\r{} {} {}",
        style("Finished").bold().fg(Color::Rgb(141, 141, 142)),
        cli.options.file_path,
        style(&format!(
            "{}.{}s",
            start_time.elapsed().as_secs(),
            start_time.elapsed().as_millis()
        ))
        .bold()
        .fg(Color::Rgb(141, 141, 142))
    );
}
