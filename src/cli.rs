use {
    super::{
        backend::compiler::options::{CompilerOptions, Linking, Opt},
        constants::TARGETS,
        LLVM_BACKEND_COMPILER, NAME,
    },
    inkwell::targets::{CodeModel, RelocMode, TargetMachine, TargetTriple},
    std::{path::PathBuf, process},
    stylic::{style, Color, Stylize},
};

pub struct Cli {
    pub options: CompilerOptions,
    args: Vec<String>,
}

impl Cli {
    pub fn parse(args: Vec<String>) -> Cli {
        let mut args_parsed: Cli = Self {
            options: CompilerOptions::default(),
            args,
        };

        args_parsed._parse();

        args_parsed
    }

    fn _parse(&mut self) {
        self.args.remove(0);

        if self.args.is_empty() {
            self.help();
            return;
        }

        let mut depth: usize = 0;

        while depth != self.args.len() {
            self.analyze(self.args[depth].clone(), &mut depth);
        }
    }

    fn analyze(&mut self, arg: String, index: &mut usize) {
        match arg.trim() {
            "help" | "-h" | "--help" => {
                *index += 1;
                self.help();
            }

            "targets" => {
                *index += 1;
                TARGETS.iter().for_each(|target| println!("{}", target));
                process::exit(0);
            }

            "native-target" => {
                *index += 1;
                println!("{}", TargetMachine::get_default_triple());
                process::exit(0);
            }

            "version" | "-v" | "--version" => {
                *index += 1;
                println!("v{}", env!("CARGO_PKG_VERSION"));
            }

            "-o" | "--output" => {
                *index += 2;

                if *index > self.args.len() {
                    self.report_error(&format!("Missing argument for {}", arg));
                }

                self.options.output = self.args[self.extract_relative_index(*index)].to_string();
            }

            "--delete-built-in-apis-after" | "-delete-built-in-apis-after" => {
                *index += 1;

                self.options.delete_built_in_apis_after = true;
            }

            "-opt" | "--optimization" => {
                *index += 1;

                if *index > self.args.len() {
                    self.report_error(&format!("Missing argument for {}", arg));
                }

                self.options.optimization =
                    match self.args[self.extract_relative_index(*index)].as_str() {
                        "O0" => Opt::None,
                        "O1" => Opt::Low,
                        "O2" => Opt::Mid,
                        "O3" => Opt::Mcqueen,
                        _ => Opt::default(),
                    };

                *index += 1;
            }

            "-emit-only-llvm" | "--emit-only-llvm" => {
                *index += 1;

                if self.options.emit_asm {
                    self.report_error(&format!(
                        "You can't use \"{}\" and \"{}\" together.",
                        "--emit-only-llvm", "--emit-only-asm"
                    ));
                }

                self.options.emit_llvm = true;
            }

            "-emit-only-asm" | "--emit-only-asm" => {
                *index += 1;

                if self.options.emit_asm {
                    self.report_error(&format!(
                        "You can't use \"{}\" and \"{}\" together.",
                        "--emit-only-llvm", "--emit-only-asm"
                    ));
                }

                self.options.emit_asm = true;
            }

            "--library" | "-library" => {
                *index += 1;

                if self.options.executable {
                    self.report_error(&format!(
                        "You can't use \"{}\" and \"{}\" together.",
                        "--executable", "--library"
                    ));
                }

                self.options.library = true;
            }

            "-target" | "--target" => {
                *index += 1;

                if *index > self.args.len() {
                    self.report_error(&format!("Missing argument for {}", arg));
                }

                match self.args[self.extract_relative_index(*index)].as_str() {
                    target if TARGETS.contains(&target) => {
                        self.options.target_triple = TargetTriple::create(target);
                        *index += 1;
                    }

                    _ => {
                        self.report_error(&format!(
                            "Invalid target: {}",
                            self.args[self.extract_relative_index(*index)]
                        ));
                    }
                }
            }

            "--static" | "-static" => {
                *index += 1;

                if self.options.linking == Linking::Dynamic {
                    self.report_error(&format!(
                        "Can't use linking \"{}\" with \"{}\" linking.",
                        arg,
                        self.options.linking.to_str()
                    ));
                }

                self.options.linking = Linking::Static;
            }

            "--dynamic" | "-dynamic" => {
                *index += 1;

                if self.options.linking == Linking::Static {
                    self.report_error(&format!(
                        "Can't use linking \"{}\" with \"{}\" linking.",
                        arg,
                        self.options.linking.to_str()
                    ));
                }

                self.options.linking = Linking::Dynamic;
            }

            "--reloc" | "-reloc" => {
                *index += 1;

                if *index > self.args.len() {
                    self.report_error(&format!("Missing argument for \"{}\".", arg));
                }

                self.options.reloc_mode =
                    match self.args[self.extract_relative_index(*index)].as_str() {
                        "dynamic-no-pic" => RelocMode::DynamicNoPic,
                        "pic" => RelocMode::PIC,
                        "static" => RelocMode::Static,
                        _ => RelocMode::Default,
                    };

                *index += 1;
            }

            "--code-model" | "-code-model" => {
                *index += 1;

                if *index > self.args.len() {
                    self.report_error(&format!("Missing argument for \"{}\".", arg));
                }

                self.options.code_model =
                    match self.args[self.extract_relative_index(*index)].as_str() {
                        "small" => CodeModel::Small,
                        "medium" => CodeModel::Medium,
                        "large" => CodeModel::Large,
                        "kernel" => CodeModel::Kernel,
                        _ => CodeModel::Default,
                    };

                *index += 1;
            }

            "--include" | "-include" => {
                *index += 1;

                if *index > self.args.len() {
                    self.report_error(&format!(
                        "Missing built-in API specification for \"{}\".",
                        arg
                    ));
                }

                match self.args[self.extract_relative_index(*index)].as_str() {
                    "vector-api" => {
                        self.options.include_vector_api = true;
                        *index += 1;
                    }
                    "debug-api" => {
                        self.options.include_debug_api = true;
                        *index += 1;
                    }
                    _ => {
                        self.report_error(&format!(
                            "Unknown built-in API name: \"{}\".",
                            self.args[self.extract_relative_index(*index)]
                        ));
                    }
                }
            }

            "--executable" | "-executable" => {
                *index += 1;
                self.options.executable = true;
            }

            path if PathBuf::from(path).exists() => {
                *index += 1;

                let mut file: PathBuf = PathBuf::from(path);

                if file.is_dir() {
                    self.report_error(&format!("\"{}\" is a directory", path));
                } else if file.extension().is_none() {
                    self.report_error(&format!("\"{}\" does not have extension.", path));
                } else if file.extension().unwrap() != "th" {
                    self.report_error(&format!("\"{}\" is not a Thrush file.", path));
                } else if file.file_name().is_none() {
                    self.report_error(&format!("\"{}\" does not have a name.", path));
                }

                if path.chars().filter(|ch| *ch == '.').count() > 2 && file.canonicalize().is_ok() {
                    file = file.canonicalize().unwrap();
                }

                if file.file_name().unwrap() == "main.th" {
                    self.options.is_main = true;
                }

                self.options.output = file.file_name().unwrap().to_string_lossy().to_string();

                self.options.file_path = path.to_string();

                if LLVM_BACKEND_COMPILER.is_none() {
                    self.report_error("LLVM Compiler Toolchain is corrupted or deleted from Thrush Toolchain, follow the below instructions.");
                }

                NAME.lock().unwrap().clear();
                NAME.lock()
                    .unwrap()
                    .push_str(file.file_name().unwrap().to_str().unwrap());
            }

            "--args" | "-args" => {
                *index += 1;

                if *index > self.args.len() {
                    self.report_error(&format!("Missing argument for {}", arg));
                }

                self.options.another_args =
                    self.args[self.extract_relative_index(*index)].to_string();

                *index += 1;
            }

            _ => {
                self.help();
            }
        }
    }

    fn extract_relative_index(&self, index: usize) -> usize {
        if index == self.args.len() {
            return index - 1;
        }

        index
    }

    fn report_error(&self, msg: &str) {
        println!(
            "{} {}",
            style("ERROR").bold().fg(Color::Rgb(255, 51, 51)),
            style(msg).bold()
        );

        process::exit(1);
    }

    fn help(&self) {
        println!(
            "\n{}\n",
            style("The Thrush Compiler")
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
            "{} ({} | {} | {}) {}",
            style("•").bold(),
            style("version").bold().fg(Color::Rgb(141, 141, 142)),
            style("-v").bold().fg(Color::Rgb(141, 141, 142)),
            style("--version").bold().fg(Color::Rgb(141, 141, 142)),
            style("Show the version.").bold()
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
        );

        println!("{}", style("\nCompiler Flags:\n").bold());

        println!(
            "{} ({} | {}) {}",
            style("•").bold(),
            style("--output [str]").bold().fg(Color::Rgb(141, 141, 142)),
            style("-output [str]").bold().fg(Color::Rgb(141, 141, 142)),
            style("Name of the executable or object file.").bold()
        );

        println!(
            "{} ({} | {}) {}",
            style("•").bold(),
            style("--target [str]").bold().fg(Color::Rgb(141, 141, 142)),
            style("-t [str]").bold().fg(Color::Rgb(141, 141, 142)),
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
            style("Include a Native API Code in the IR.").bold()
        );

        println!(
            "{} ({} | {}) {}",
            style("•").bold(),
            style("--delete-built-in-apis-after")
                .bold()
                .fg(Color::Rgb(141, 141, 142)),
            style("-delete-built-in-apis-after")
                .bold()
                .fg(Color::Rgb(141, 141, 142)),
            style("Delete Compiler built-in API after the Compilation.").bold()
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
            style("--executable").bold().fg(Color::Rgb(141, 141, 142)),
            style("-executable").bold().fg(Color::Rgb(141, 141, 142)),
            style("Compile the code into native executable.").bold()
        );

        println!(
            "{} ({} | {}) {}",
            style("•").bold(),
            style("--library").bold().fg(Color::Rgb(141, 141, 142)),
            style("-library").bold().fg(Color::Rgb(141, 141, 142)),
            style("Compile to an object file.").bold()
        );

        println!(
            "{} ({} | {}) {}",
            style("•").bold(),
            style("--reloc [reloc-mode]")
                .bold()
                .fg(Color::Rgb(141, 141, 142)),
            style("-reloc [reloc-mode]")
                .bold()
                .fg(Color::Rgb(141, 141, 142)),
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
            style("Define how code is organized and accessed in the executable or object file.")
                .bold()
        );

        println!(
            "{} ({} | {}) {}",
            style("•").bold(),
            style("--args [str]").bold().fg(Color::Rgb(141, 141, 142)),
            style("-args [str]").bold().fg(Color::Rgb(141, 141, 142)),
            style("Pass more arguments to the Backend Compiler.").bold()
        );

        process::exit(0);
    }
}
