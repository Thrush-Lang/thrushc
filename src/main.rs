mod backend;
mod constants;
mod diagnostic;
mod error;
mod frontend;
mod logging;

use {
    backend::{
        builder::FileBuilder,
        compiler::{Compiler, CompilerOptions, Linking, Opt},
        infraestructures::vector::VectorAPI,
        instruction::Instruction,
    },
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
    std::{env, fs::read_to_string, path::Path, sync::Mutex, time::Instant},
    stylic::{style, Color, Stylize},
};

pub static NAME: Mutex<String> = Mutex::new(String::new());
pub static PATH: Mutex<String> = Mutex::new(String::new());

fn main() {
    let mut parameters: Vec<String> = env::args().collect();
    let mut options: CompilerOptions = CompilerOptions::default();
    let mut backend: String = String::new();
    let mut compile: bool = false;

    let mut rebuild_vec_api: bool = false;

    parameters.remove(0);

    for parameter in parameters.iter() {
        match parameter.as_str() {
            "-h" | "help" | "--help" | "-help" => {
                help();

                return;
            }

            "-t" | "targets" => {
                TARGETS.iter().for_each(|target| {
                    println!("{}", style(target).bold());
                });

                return;
            }

            "-nt" | "native-target" => {
                println!(
                    "{}",
                    style(TargetMachine::get_default_triple().to_string()).bold()
                );

                return;
            }

            "-v" | "--version" => {
                println!("{}", style(env!("CARGO_PKG_VERSION")).bold());

                return;
            }

            "-api" | "api" => {
                if parameters.len() < 3 {
                    apis_help();
                    return;
                }

                match parameters[1].as_str() {
                    "vector" => {
                        match parameters[2].as_str() {
                            "rebuild" => {
                                for (index, parameter) in parameters.iter().skip(3).enumerate() {
                                    if parameter == "--backend" || parameter == "-backend" {
                                        if parameters.get(3 + index + 1).is_none() {
                                            vector_api_help();
                                            return;
                                        }

                                        let backend_path: &Path =
                                            Path::new(&parameters[3 + index + 1]);

                                        if !backend_path.exists() {
                                            logging::log(
                                                logging::LogType::ERROR,
                                                "The backend path does not exists.",
                                            );

                                            return;
                                        } else if !backend_path.is_dir() {
                                            logging::log(
                                                logging::LogType::ERROR,
                                                "The backend path don't terminate with type folder.",
                                            );

                                            return;
                                        } else if !backend_path.ends_with("bin") {
                                            logging::log(
                                                logging::LogType::ERROR,
                                                "The backend path don't terminate in bin folder.",
                                            );

                                            return;
                                        } else if !backend_path.join("clang-18").is_file() {
                                            logging::log(
                                                logging::LogType::ERROR,
                                                "The backend path don't contain a valid executable of clang-18.",
                                            );

                                            return;
                                        } else if !backend_path.join("llvm-config").is_file() {
                                            logging::log(
                                                logging::LogType::ERROR,
                                                "The backend path don't contain a valid executable for llvm-config.",
                                            );

                                            return;
                                        } else if !backend_path.join("lld").is_file() {
                                            logging::log(
                                                logging::LogType::ERROR,
                                                "The backend path don't contain a valid executable for linker of llvm (lld).",
                                            );

                                            return;
                                        } else if !backend_path.join("opt").is_file() {
                                            logging::log(
                                                logging::LogType::ERROR,
                                                "The backend path don't contain a valid executable for optimizer of llvm (opt).",
                                            );

                                            return;
                                        }

                                        backend.push_str(&parameters[3 + index + 1]);
                                    } else if parameter == "--target" || parameter == "-t" {
                                        if TARGETS.contains(&parameters[3 + index + 1].as_str()) {
                                            options.target_triple =
                                                TargetTriple::create(&parameters[3 + index + 1]);

                                            continue;
                                        }

                                        logging::log(logging::LogType::ERROR, &format!(
                                            "The target '{}' is not supported, see the list with Thrushr --print-targets.",
                                            &parameters[3 + index + 1]
                                        ));

                                        return;
                                    } else if parameter == "--emit-only-llvm"
                                        || parameter == "-emit-only-llvm"
                                    {
                                        options.emit_llvm = true;
                                    } else if parameter == "--codemodel" || parameter == "-codemd" {
                                        match parameters[3 + index + 1].as_str() {
                                            "jit" => {
                                                options.code_model = CodeModel::JITDefault;
                                            }
                                            "sys" => {
                                                options.code_model = CodeModel::Kernel;
                                            }
                                            "medium" => {
                                                options.code_model = CodeModel::Medium;
                                            }
                                            "large" => options.code_model = CodeModel::Large,
                                            _ => (),
                                        }
                                    } else if parameter == "--optimization" || parameter == "-opt" {
                                        match parameters[3 + index + 1].as_str() {
                                            "low" => {
                                                options.optimization = Opt::Low;
                                            }
                                            "mid" => {
                                                options.optimization = Opt::Mid;
                                            }
                                            "mcqueen" => {
                                                options.optimization = Opt::Mcqueen;
                                            }
                                            _ => (),
                                        }
                                    } else if parameter == "--reloc" || parameter == "-reloc" {
                                        match parameters[3 + index + 1].as_str() {
                                            "dynamic" => {
                                                options.reloc_mode = RelocMode::DynamicNoPic;
                                            }
                                            "pic" => {
                                                options.reloc_mode = RelocMode::PIC;
                                            }
                                            "static" => {
                                                options.reloc_mode = RelocMode::Static;
                                            }
                                            _ => (),
                                        }
                                    }
                                }

                                rebuild_vec_api = true;
                                options.rebuild_apis = true;
                                options.emit_object = true;
                            }
                            _ => {
                                vector_api_help();
                                return;
                            }
                        }

                        break;
                    }

                    _ => {
                        apis_help();
                    }
                }

                return;
            }

            "-c" | "compile" => {
                if parameters.len() == 1 {
                    compile_help();
                    return;
                }

                let index: usize = parameters.len() - 1;
                let path: &Path = Path::new(&parameters[index]);

                if !path.exists() {
                    logging::log(
                        logging::LogType::ERROR,
                        &format!("The path '{}' cannot be accessed.", &parameters[index]),
                    );

                    return;
                }

                if !path.is_file() {
                    logging::log(
                        logging::LogType::ERROR,
                        &format!("The path '{}' ended with not a file.", &parameters[index]),
                    );

                    return;
                }

                if path.extension().is_none() {
                    logging::log(
                        logging::LogType::ERROR,
                        &format!(
                            "The file in path '{}' does not have an extension.",
                            &parameters[index]
                        ),
                    );

                    return;
                }

                if path.extension().unwrap() != "th" {
                    logging::log(
                        logging::LogType::ERROR,
                        &format!(
                            "The file in path '{}' does not have the extension '.th'.",
                            &parameters[index]
                        ),
                    );

                    return;
                }

                for i in 1..parameters.len() - 1 {
                    match parameters[i].as_str() {
                        "--backend" | "-backend" => {
                            if !Path::new(&parameters[i + 1]).exists() {
                                logging::log(
                                    logging::LogType::ERROR,
                                    "The backend path does not exists.",
                                );

                                return;
                            } else if !Path::new(&parameters[i + 1]).is_dir() {
                                logging::log(
                                    logging::LogType::ERROR,
                                    "The backend path don't terminate with type folder.",
                                );

                                return;
                            } else if !Path::new(&parameters[i + 1]).ends_with("bin") {
                                logging::log(
                                    logging::LogType::ERROR,
                                    "The backend path don't terminate in bin folder.",
                                );

                                return;
                            } else if !Path::new(&parameters[i + 1]).join("clang-18").is_file() {
                                logging::log(
                                    logging::LogType::ERROR,
                                    "The backend path don't contain a valid executable of clang-18.",
                                );

                                return;
                            } else if !Path::new(&parameters[i + 1]).join("llvm-config").is_file() {
                                logging::log(
                                    logging::LogType::ERROR,
                                    "The backend path don't contain a valid executable for llvm-config.",
                                );

                                return;
                            } else if !Path::new(&parameters[i + 1]).join("lld").is_file() {
                                logging::log(
                                    logging::LogType::ERROR,
                                    "The backend path don't contain a valid executable for linker of llvm (lld).",
                                );

                                return;
                            } else if !Path::new(&parameters[i + 1]).join("opt").is_file() {
                                logging::log(
                                    logging::LogType::ERROR,
                                    "The backend path don't contain a valid executable for optimizer of llvm (opt).",
                                );

                                return;
                            }

                            backend.push_str(&parameters[i + 1]);
                        }
                        "--include" | "-include" => match parameters[i + 1].as_str() {
                            "vec_api" => {
                                options.include_vec_api = true;
                            }
                            _ => {
                                logging::log(
                                    logging::LogType::ERROR,
                                    "The include api's is not specified correctly.",
                                );

                                return;
                            }
                        },
                        "--lib" | "-lib" => {
                            options.emit_object = true;
                        }
                        "--name" | "-n" => {
                            options.name = parameters[i + 1].clone();
                        }
                        "--target" | "-t" => {
                            if TARGETS.contains(&parameters[i + 1].as_str()) {
                                options.target_triple = TargetTriple::create(&parameters[i + 1]);

                                continue;
                            }

                            logging::log(logging::LogType::ERROR, &format!(
                                "The target '{}' is not supported, see the list with Thrushr --print-targets.",
                                &parameters[i + 1]
                            ));

                            return;
                        }
                        "--optimization" | "-opt" => match parameters[i + 1].as_str() {
                            "low" => {
                                options.optimization = Opt::Low;
                            }
                            "mid" => {
                                options.optimization = Opt::Mid;
                            }
                            "mcqueen" => {
                                options.optimization = Opt::Mcqueen;
                            }
                            _ => (),
                        },

                        "--codemodel" | "-codemd" => match parameters[i + 1].as_str() {
                            "jit" => {
                                options.code_model = CodeModel::JITDefault;
                            }
                            "sys" => {
                                options.code_model = CodeModel::Kernel;
                            }
                            "medium" => {
                                options.code_model = CodeModel::Medium;
                            }
                            "large" => options.code_model = CodeModel::Large,
                            _ => (),
                        },

                        "--reloc" | "-reloc" => match parameters[i + 1].as_str() {
                            "dynamic" => {
                                options.reloc_mode = RelocMode::DynamicNoPic;
                            }
                            "pic" => {
                                options.reloc_mode = RelocMode::PIC;
                            }
                            "static" => {
                                options.reloc_mode = RelocMode::Static;
                            }
                            _ => (),
                        },

                        "--emit-only-llvm" | "-emit-only-llvm" => {
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

                NAME.lock().unwrap().push_str(
                    Path::new(&parameters[index])
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .split(".")
                        .collect::<Vec<_>>()[0],
                );

                PATH.lock().unwrap().push_str(&parameters[index]);

                compile = true;

                break;
            }

            _ => {
                help();
            }
        }
    }

    if !compile && !options.rebuild_apis {
        return;
    } else if backend.is_empty() {
        logging::log(
            logging::LogType::ERROR,
            "Cannot compile if don't specified the Backend path for the Compiler.",
        );

        return;
    }

    Target::initialize_all(&InitializationConfig::default());

    if options.rebuild_apis {
        let context: Context = Context::create();
        let builder: Builder<'_> = context.create_builder();

        options.name = if rebuild_vec_api {
            "vectorapi".to_string()
        } else {
            "genericapi".to_string()
        };

        let module: Module<'_> = context.create_module(&options.name);

        module.set_triple(&options.target_triple);

        let opt: OptimizationLevel = options.optimization.to_llvm_opt();

        let machine: TargetMachine = Target::from_triple(&options.target_triple)
            .unwrap()
            .create_target_machine(
                &options.target_triple,
                "",
                "",
                opt,
                options.reloc_mode,
                options.code_model,
            )
            .unwrap();

        module.set_data_layout(&machine.get_target_data().get_data_layout());

        if rebuild_vec_api {
            VectorAPI::include(&module, &builder, &context);
        }

        if options.emit_llvm {
            module
                .print_to_file(format!("{}.ll", options.name))
                .unwrap();
            return;
        }

        FileBuilder::new(&options, &module, &backend).build();

        return;
    } else if compile && format!("{}.th", NAME.lock().unwrap().as_str()) == "main.th" {
        options.is_main = true;
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
    let module: Module<'_> = context.create_module(&options.name);

    let start_time: Instant = Instant::now();

    let tokens: Result<&[Token], String> = lexer.lex();

    match tokens {
        Ok(tokens) => {
            parser.tokens = Some(tokens);
            parser.options = Some(&options);

            let instructions: Result<&[Instruction<'_>], String> = parser.start();

            match instructions {
                Ok(instructions) => {
                    module.set_triple(&options.target_triple);

                    let opt: OptimizationLevel = options.optimization.to_llvm_opt();

                    let machine: TargetMachine = Target::from_triple(&options.target_triple)
                        .unwrap()
                        .create_target_machine(
                            &options.target_triple,
                            "",
                            "",
                            opt,
                            options.reloc_mode,
                            options.code_model,
                        )
                        .unwrap();

                    module.set_data_layout(&machine.get_target_data().get_data_layout());

                    Compiler::compile(&module, &builder, &context, &options, instructions);

                    FileBuilder::new(&options, &module, &backend).build();
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
        "  {} {} {}",
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

fn help() {
    println!(
        "\n{}\n",
        style("Thrush Lang Compiler")
            .bold()
            .fg(Color::Rgb(141, 141, 142))
    );

    println!(
        "{} {} {}\n",
        style("Usage:").bold(),
        style("thrushc").bold().fg(Color::Rgb(141, 141, 142)),
        style("[--flags] [file]").bold()
    );

    println!("{}", style("Available Commands:\n").bold());

    println!(
        "{} ({} | {}) {}",
        style("•").bold(),
        style("help").bold().fg(Color::Rgb(141, 141, 142)),
        style("-h").bold().fg(Color::Rgb(141, 141, 142)),
        style("Show help message.").bold()
    );

    println!(
        "{} ({} | {}) {}",
        style("•").bold(),
        style("version").bold().fg(Color::Rgb(141, 141, 142)),
        style("-v").bold().fg(Color::Rgb(141, 141, 142)),
        style("Show the version.").bold()
    );

    println!(
        "{} ({} | {}) {}",
        style("•").bold(),
        style("compile [--flags] [file]")
            .bold()
            .fg(Color::Rgb(141, 141, 142)),
        style("-c [--flags] [file]")
            .bold()
            .fg(Color::Rgb(141, 141, 142)),
        style("Compile the code provided into executable, object file, LLVM IR, LLVM Bitcode or Native Assembly.").bold()
    );

    println!(
        "{} ({} | {}) {}",
        style("•").bold(),
        style("targets").bold().fg(Color::Rgb(141, 141, 142)),
        style("-t").bold().fg(Color::Rgb(141, 141, 142)),
        style("Print the list of supported targets machines.").bold()
    );

    println!(
        "{} ({} | {}) {}",
        style("•").bold(),
        style("native-target").bold().fg(Color::Rgb(141, 141, 142)),
        style("-nt").bold().fg(Color::Rgb(141, 141, 142)),
        style("Print the native target of this machine.").bold()
    )
}

fn compile_help() {
    println!(
        "{} {} {} {}\n",
        style("Usage:").bold(),
        style("thrushc").bold().fg(Color::Rgb(141, 141, 142)),
        style("compile").bold(),
        style("[--flags] [file]")
            .bold()
            .fg(Color::Rgb(141, 141, 142)),
    );

    println!("{}", style("Available Flags:\n").bold());

    println!(
        "{} ({} | {}) {}",
        style("•").bold(),
        style("--backend").bold().fg(Color::Rgb(141, 141, 142)),
        style("-backend").bold().fg(Color::Rgb(141, 141, 142)),
        style("Specific the path to the backend compiler to use it (Clang && LLVM).").bold()
    );

    println!(
        "{} ({} | {}) {}",
        style("•").bold(),
        style("--name [name]").bold().fg(Color::Rgb(141, 141, 142)),
        style("-n [name]").bold().fg(Color::Rgb(141, 141, 142)),
        style("Name of the executable or object file.").bold()
    );

    println!(
        "{} ({} | {}) {}",
        style("•").bold(),
        style("--target [target]")
            .bold()
            .fg(Color::Rgb(141, 141, 142)),
        style("-t [target]").bold().fg(Color::Rgb(141, 141, 142)),
        style("Target architecture for the executable or object file.").bold()
    );

    println!(
        "{} ({} | {}) {}",
        style("•").bold(),
        style("--optimization [opt-level]")
            .bold()
            .fg(Color::Rgb(141, 141, 142)),
        style("-opt [opt-level]")
            .bold()
            .fg(Color::Rgb(141, 141, 142)),
        style("Optimization level for the executable to emit or object file.").bold()
    );

    println!(
        "{} ({} | {}) {}",
        style("•").bold(),
        style("--emit-only-llvm")
            .bold()
            .fg(Color::Rgb(141, 141, 142)),
        style("-emit-only-llvm")
            .bold()
            .fg(Color::Rgb(141, 141, 142)),
        style("Compile the code only to LLVM IR.").bold()
    );

    println!(
        "{} ({} | {}) {}",
        style("•").bold(),
        style("--include").bold().fg(Color::Rgb(141, 141, 142)),
        style("-include").bold().fg(Color::Rgb(141, 141, 142)),
        style("Includes an internal API in the LLVM IR to be emited.").bold()
    );

    println!(
        "{} ({} | {}) {}",
        style("•").bold(),
        style("--static").bold().fg(Color::Rgb(141, 141, 142)),
        style("-s").bold().fg(Color::Rgb(141, 141, 142)),
        style("Link the executable statically.").bold()
    );

    println!(
        "{} ({} | {}) {}",
        style("•").bold(),
        style("--dynamic").bold().fg(Color::Rgb(141, 141, 142)),
        style("-s").bold().fg(Color::Rgb(141, 141, 142)),
        style("Link the executable dynamically.").bold()
    );

    println!(
        "{} ({} | {}) {}",
        style("•").bold(),
        style("--build").bold().fg(Color::Rgb(141, 141, 142)),
        style("-b").bold().fg(Color::Rgb(141, 141, 142)),
        style("Compile the code into executable.").bold()
    );

    println!(
        "{} ({} | {}) {}",
        style("•").bold(),
        style("--lib").bold().fg(Color::Rgb(141, 141, 142)),
        style("-lib").bold().fg(Color::Rgb(141, 141, 142)),
        style("Compile to an object file.").bold()
    );

    println!(
        "{} ({} | {}) {}",
        style("•").bold(),
        style("--reloc [mode]").bold().fg(Color::Rgb(141, 141, 142)),
        style("-reloc [mode]").bold().fg(Color::Rgb(141, 141, 142)),
        style("Indicate how references to memory addresses are handled.").bold()
    );

    println!(
        "{} ({} | {}) {}",
        style("•").bold(),
        style("--codemodel [model]")
            .bold()
            .fg(Color::Rgb(141, 141, 142)),
        style("-codemd [model]")
            .bold()
            .fg(Color::Rgb(141, 141, 142)),
        style("Define how code is organized and accessed in the executable or object file.").bold()
    );
}

fn apis_help() {
    println!(
        "{} {} {} {}\n",
        style("Usage:").bold(),
        style("thrushc").bold().fg(Color::Rgb(141, 141, 142)),
        style("api").bold(),
        style("[name] [command] --flags")
            .bold()
            .fg(Color::Rgb(141, 141, 142)),
    );

    println!("{}", style("Available APIs:\n").bold());

    println!(
        "{} ({} | {}) {}",
        style("•").bold(),
        style("vector").bold().fg(Color::Rgb(141, 141, 142)),
        style("vector").bold().fg(Color::Rgb(141, 141, 142)),
        style("Select the Backend Vector API.").bold()
    );
}

fn vector_api_help() {
    println!(
        "{} {} {} {}\n",
        style("Usage:").bold(),
        style("thrushc").bold().fg(Color::Rgb(141, 141, 142)),
        style("api").bold(),
        style("vector [command] --flags")
            .bold()
            .fg(Color::Rgb(141, 141, 142)),
    );

    println!("{}", style("Available Commands:\n").bold());

    println!(
        "{} ({} | {}) {}",
        style("•").bold(),
        style("rebuild").bold().fg(Color::Rgb(141, 141, 142)),
        style("rb").bold().fg(Color::Rgb(141, 141, 142)),
        style("Rebuild the internal Backend Vector API.").bold()
    );

    println!("{}", style("Available Flags:\n").bold());

    println!(
        "{} ({} | {}) {}",
        style("•").bold(),
        style("--backend").bold().fg(Color::Rgb(141, 141, 142)),
        style("-backend").bold().fg(Color::Rgb(141, 141, 142)),
        style("Specific the path to the Backend Compiler to use it (Clang && LLVM).").bold()
    );

    println!(
        "{} ({} | {}) {}",
        style("•").bold(),
        style("--target [target]")
            .bold()
            .fg(Color::Rgb(141, 141, 142)),
        style("-t [target]").bold().fg(Color::Rgb(141, 141, 142)),
        style("Target architecture for the executable or object file.").bold()
    );

    println!(
        "{} ({} | {}) {}",
        style("•").bold(),
        style("--optimization [opt-level]")
            .bold()
            .fg(Color::Rgb(141, 141, 142)),
        style("-opt [opt-level]")
            .bold()
            .fg(Color::Rgb(141, 141, 142)),
        style("Optimization level for the executable to emit or object file.").bold()
    );

    println!(
        "{} ({} | {}) {}",
        style("•").bold(),
        style("--emit-only-llvm")
            .bold()
            .fg(Color::Rgb(141, 141, 142)),
        style("-emit-only-llvm")
            .bold()
            .fg(Color::Rgb(141, 141, 142)),
        style("Compile the code only to LLVM IR.").bold()
    );

    println!(
        "{} ({} | {}) {}",
        style("•").bold(),
        style("--reloc [mode]").bold().fg(Color::Rgb(141, 141, 142)),
        style("-reloc [mode]").bold().fg(Color::Rgb(141, 141, 142)),
        style("Indicate how references to memory addresses are handled.").bold()
    );

    println!(
        "{} ({} | {}) {}",
        style("•").bold(),
        style("--codemodel [model]")
            .bold()
            .fg(Color::Rgb(141, 141, 142)),
        style("-codemd [model]")
            .bold()
            .fg(Color::Rgb(141, 141, 142)),
        style("Define how code is organized and accessed in the executable or object file.").bold()
    );
}
