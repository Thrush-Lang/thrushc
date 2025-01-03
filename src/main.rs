mod backend;
mod cli;
mod constants;
mod diagnostic;
mod error;
mod frontend;
mod logging;

use {
    ahash::AHashMap as HashMap,
    backend::{
        apis::{debug, vector},
        builder::{Clang, LLVMOptimizator},
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
        env, fs,
        path::{Path, PathBuf},
        process,
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
    static ref CORE_LIBRARY_PATH: HashMap<&'static str, (String, String)> = {
        if HOME.is_none() {
            logging::log(
                logging::LogType::ERROR,
                &format!("Thrush Toolchain is unreacheable via path, re-install the entire toolchain via \"thorium install {}\".", env::consts::OS),
            );

            process::exit(1);
        }

        let mut imports: HashMap<&'static str, (String, String)> = HashMap::with_capacity(1);

        imports.insert(
            "core.fmt",
            (
                String::from("fmt.th"),
                HOME.as_ref()
                    .unwrap()
                    .join("thrushlang/core/fmt.th")
                    .to_string_lossy()
                    .to_string(),
            ),
        );

        imports
    };
    static ref LLVM_BACKEND_COMPILER: PathBuf = {
        if HOME.is_none() {
            logging::log(
                logging::LogType::ERROR,
                &format!("LLVM Toolchain was corrupted from Thrush Toolchain, re-install the entire toolchain via \"thorium install {}\".", env::consts::OS),
            );

            process::exit(1);
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

            process::exit(1);
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

            process::exit(1);
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

            process::exit(1);
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

            process::exit(1);
        } else if !HOME
            .as_ref()
            .unwrap()
            .join("thrushlang/backends/llvm/backend/bin/llvm-dis")
            .exists()
        {
            logging::log(
            logging::LogType::ERROR,
            &format!("LLVM Dissambler don't exists in Thrush Toolchain, re-install the entire toolchain via \"thorium install {}\".", env::consts::OS),
        );

            process::exit(1);
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

            process::exit(1);
        }

        HOME.as_ref()
            .unwrap()
            .join("thrushlang/backends/llvm/backend/bin/")
    };
}

fn main() {
    if !["linux", "windows"].contains(&env::consts::OS) {
        logging::log(
            logging::LogType::ERROR,
            "Compilation from Unsopported Operating System. Only Linux and Windows are supported.",
        );

        process::exit(1);
    }

    Target::initialize_all(&InitializationConfig::default());

    let mut cli: Cli = Cli::parse(env::args().collect());

    if !cli.options.include_vector_api {
        vector::compile_vector_api(&mut cli.options);
    }

    if !cli.options.include_debug_api {
        debug::compile_debug_api(&mut cli.options);
    }

    cli.options.files.sort_by_key(|file| file.name != "main.th");

    if cli.options.executable || cli.options.library || cli.options.static_library {
        cli.options
            .args
            .extend(["output/vector.o".to_string(), "output/debug.o".to_string()]);
    }

    let start_time: Instant = Instant::now();

    let mut compiled: Vec<PathBuf> = Vec::new();

    for file in cli.options.files.iter() {
        println!(
            "{} {}",
            style("Compiling").bold().fg(Color::Rgb(141, 141, 142)),
            &file.path.to_string_lossy()
        );

        let content: String = fs::read_to_string(&file.path).unwrap();

        let mut lexer: Lexer = Lexer::new(content.as_bytes(), file);
        let tokens: &[Token] = lexer.lex();

        let mut parser: Parser = Parser::new(tokens, file);
        let instructions: &[Instruction] = parser.start();

        let context: Context = Context::create();
        let builder: Builder<'_> = context.create_builder();
        let module: Module<'_> = context.create_module(&file.name);

        // println!("{:#?}", instructions);

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

        let compiled_path: &str = &format!("output/{}.bc", &file.name);

        module.write_bitcode_to_path(Path::new(compiled_path));

        LLVMOptimizator::optimize(
            compiled_path,
            cli.options.optimization.to_llvm_17_passes(),
            cli.options.optimization.to_str(true, false),
        );

        compiled.push(PathBuf::from(compiled_path));
    }

    compiled.sort_by_key(|path| *path != PathBuf::from("output/main.th.bc"));

    Clang::new(&compiled, &cli.options).compile();

    let _ = fs::copy(
        &cli.options.output,
        format!("output/{}", cli.options.output),
    );

    let _ = fs::remove_file(&cli.options.output);

    println!(
        "\r{} {}",
        style("Finished").bold().fg(Color::Rgb(141, 141, 142)),
        style(&format!(
            "{}.{}s",
            start_time.elapsed().as_secs(),
            start_time.elapsed().as_millis()
        ))
        .bold()
        .fg(Color::Rgb(141, 141, 142))
    );

    compiled.iter().for_each(|path| {
        let _ = fs::remove_file(path);
    });

    let _ = fs::remove_file("output/vector.o");
    let _ = fs::remove_file("output/debug.o");
}
