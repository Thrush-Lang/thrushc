mod backend;
mod diagnostic;
mod error;
mod frontend;
mod logging;

use {
    backend::compiler::{Compiler, FileBuilder, Instruction, Linking, Opt, Options},
    colored::{Colorize, CustomColor},
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
    std::{env, fs::read_to_string, path::Path, sync::Mutex},
};

pub static NAME: Mutex<String> = Mutex::new(String::new());
pub static PATH: Mutex<String> = Mutex::new(String::new());

fn main() {
    let mut parameters: Vec<String> = env::args().collect();
    let mut options: Options = Options::default();
    let mut compile: bool = false;

    parameters.remove(0);

    for parameter in parameters.iter() {
        match parameter.as_str() {
            "-h" | "--help" => {
                help();
                return;
            }

            "-t" | "targets" => {
                TARGETS.iter().for_each(|target| {
                    println!("{}", target.bold());
                });
                return;
            }

            "-nt" | "native-target" => {
                println!("{}", TargetMachine::get_default_triple().to_string().bold());
                return;
            }

            "-v" | "--version" => {
                println!("{}", env!("CARGO_PKG_VERSION").bold());
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
                            "none" => {
                                options.optimization = Opt::None;
                            }
                            "low" => {
                                options.optimization = Opt::Low;
                            }
                            "mid" => {
                                options.optimization = Opt::Mid;
                            }
                            "mcqueen" => {
                                options.optimization = Opt::Mcqueen;
                            }
                            _ => {
                                options.optimization = Opt::None;
                            }
                        },

                        "--codemodel" | "-codemd" => match parameters[i + 1].as_str() {
                            "default" => {
                                options.code_model = CodeModel::Default;
                            }
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
                            _ => {
                                options.code_model = CodeModel::Default;
                            }
                        },

                        "--reloc" | "-reloc" => match parameters[i + 1].as_str() {
                            "default" => {
                                options.reloc_mode = RelocMode::Default;
                            }
                            "dynamic" => {
                                options.reloc_mode = RelocMode::DynamicNoPic;
                            }
                            "pic" => {
                                options.reloc_mode = RelocMode::PIC;
                            }
                            "static" => {
                                options.reloc_mode = RelocMode::Static;
                            }
                            _ => {
                                options.reloc_mode = RelocMode::Default;
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

                options.path = Path::new(&parameters[index]).to_path_buf();

                compile = true;

                break;
            }

            "-i" | "interpret" => {}

            _ => {
                help();
                return;
            }
        }
    }

    if options.interpret && options.emit_llvm {
        logging::log(
            logging::LogType::ERROR,
            "Cannot issue llvm ir when interpreting. Use `thrushc compile --emit-llvm file.th` instead.",
        );
        return;
    } else if options.interpret && options.emit_object {
        logging::log(
            logging::LogType::ERROR,
            "Cannot emit object file when interpreting. Use `thrushc compile --lib file.th` instead.",
        );
        return;
    }

    if &format!("{}.th", NAME.lock().unwrap().as_str()) == "main.th" {
        options.is_main = true;
    }

    let origin_content: String = read_to_string(&options.path).unwrap_or_else(|error| {
        logging::log(logging::LogType::ERROR, error.to_string().as_str());
        panic!()
    });

    let content: &[u8] = origin_content.as_bytes();

    let mut lexer: Lexer = Lexer::new(content);
    let mut parser: Parser = Parser::new();

    Target::initialize_all(&InitializationConfig::default());

    let context: Context = Context::create();
    let builder: Builder<'_> = context.create_builder();
    let module: Module<'_> = context.create_module(&options.name);

    println!(
        "\n{} {}",
        "Compiling"
            .custom_color(CustomColor::new(141, 141, 142))
            .bold(),
        PATH.lock().unwrap()
    );

    let tokens: Result<&[Token], String> = lexer.lex();

    match tokens {
        Ok(tokens) => {
            parser.tokens = Some(tokens);
            parser.options = Some(&options);

            let instructions: Result<&[Instruction<'_>], String> = parser.start();

            match instructions {
                Ok(instructions) => {
                    module.set_triple(&options.target_triple);

                    let opt: OptimizationLevel = match &options.optimization {
                        Opt::None => OptimizationLevel::None,
                        Opt::Low => OptimizationLevel::Default,
                        Opt::Mid => OptimizationLevel::Less,
                        Opt::Mcqueen => OptimizationLevel::Aggressive,
                    };

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

                    Compiler::compile(&module, &builder, &context, instructions);

                    if compile {
                        FileBuilder::new(&options, &module).build();
                    } else {
                        todo!()
                    }

                    println!(
                        "  {} {}",
                        "Finished"
                            .custom_color(CustomColor::new(141, 141, 142))
                            .bold(),
                        PATH.lock().unwrap()
                    );
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
}

fn help() {
    println!(
        "\n{}\n",
        "Thrush Compiler"
            .custom_color(CustomColor::new(141, 141, 142))
            .bold()
    );

    println!(
        "{} {} {}\n",
        "Usage:".bold(),
        "thrushc"
            .custom_color(CustomColor::new(141, 141, 142))
            .bold(),
        "[--flags] [file]".bold()
    );

    println!("{}", "Available Commands:\n".bold());

    println!(
        "{} ({} | {}) {}",
        "•".bold(),
        "help".custom_color(CustomColor::new(141, 141, 142)).bold(),
        "-h".custom_color(CustomColor::new(141, 141, 142)).bold(),
        "Show help message.".bold()
    );

    println!(
        "{} ({} | {}) {}",
        "•".bold(),
        "version"
            .custom_color(CustomColor::new(141, 141, 142))
            .bold(),
        "-v".custom_color(CustomColor::new(141, 141, 142)).bold(),
        "Show the version.".bold()
    );

    println!(
        "{} ({} | {}) {}",
        "•".bold(),
        "interpret [--flags] [file]"
            .custom_color(CustomColor::new(141, 141, 142))
            .bold(),
        "-i [--flags] [file]"
            .custom_color(CustomColor::new(141, 141, 142))
            .bold(),
        "Interpret the code provided using the Interpreter.".bold()
    );

    println!(
        "{} ({} | {}) {}",
        "•".bold(),
        "compile [--flags] [file]"
            .custom_color(CustomColor::new(141, 141, 142))
            .bold(),
        "-c [--flags] [file]"
            .custom_color(CustomColor::new(141, 141, 142))
            .bold(),
        "Compile the code provided into executable.".bold()
    );

    println!(
        "{} ({} | {}) {}",
        "•".bold(),
        "targets"
            .custom_color(CustomColor::new(141, 141, 142))
            .bold(),
        "-t".custom_color(CustomColor::new(141, 141, 142)).bold(),
        "Print the list of supported targets machines.".bold()
    );

    println!(
        "{} ({} | {}) {}",
        "•".bold(),
        "native-target"
            .custom_color(CustomColor::new(141, 141, 142))
            .bold(),
        "-nt".custom_color(CustomColor::new(141, 141, 142)).bold(),
        "Print the native target of this machine.".bold()
    )
}

fn compile_help() {
    println!(
        "{} {} {} {}\n",
        "Usage:".bold(),
        "thrushc"
            .custom_color(CustomColor::new(141, 141, 142))
            .bold(),
        "compile".bold(),
        "[--flags] [file]"
            .bold()
            .custom_color(CustomColor::new(141, 141, 142))
            .bold(),
    );

    println!("{}", "Available Flags:\n".bold());

    println!(
        "{} ({} | {}) {}",
        "•".bold(),
        "--name [name]"
            .custom_color(CustomColor::new(141, 141, 142))
            .bold(),
        "-n [name]"
            .custom_color(CustomColor::new(141, 141, 142))
            .bold(),
        "Name of the executable (Compiler dispatches it).".bold()
    );

    println!(
        "{} ({} | {}) {}",
        "•".bold(),
        "--target [target]"
            .custom_color(CustomColor::new(141, 141, 142))
            .bold(),
        "-t [target]"
            .custom_color(CustomColor::new(141, 141, 142))
            .bold(),
        "Target architecture for the Compiler or Interpreter.".bold()
    );

    println!(
        "{} ({} | {}) {}",
        "•".bold(),
        "--optimization [opt-level]"
            .custom_color(CustomColor::new(141, 141, 142))
            .bold(),
        "-opt [opt-level]"
            .custom_color(CustomColor::new(141, 141, 142))
            .bold(),
        "Optimization level for the JIT compiler or the Compiler.".bold()
    );

    println!(
        "{} ({} | {}) {}",
        "•".bold(),
        "--emit-llvm"
            .custom_color(CustomColor::new(141, 141, 142))
            .bold(),
        "-emit-llvm"
            .custom_color(CustomColor::new(141, 141, 142))
            .bold(),
        "Compile the code to LLVM IR.".bold()
    );

    println!(
        "{} ({} | {}) {}",
        "•".bold(),
        "--static"
            .custom_color(CustomColor::new(141, 141, 142))
            .bold(),
        "-s".custom_color(CustomColor::new(141, 141, 142)).bold(),
        "Link the executable statically.".bold()
    );

    println!(
        "{} ({} | {}) {}",
        "•".bold(),
        "--dynamic"
            .custom_color(CustomColor::new(141, 141, 142))
            .bold(),
        "-s".custom_color(CustomColor::new(141, 141, 142)).bold(),
        "Link the executable dynamically.".bold()
    );

    println!(
        "{} ({} | {}) {}",
        "•".bold(),
        "--build"
            .custom_color(CustomColor::new(141, 141, 142))
            .bold(),
        "-b".custom_color(CustomColor::new(141, 141, 142)).bold(),
        "Compile the code into executable.".bold()
    );

    println!(
        "{} ({} | {}) {}",
        "•".bold(),
        "--lib".custom_color(CustomColor::new(141, 141, 142)).bold(),
        "-lib".custom_color(CustomColor::new(141, 141, 142)).bold(),
        "Compile the file to an object and then link it to an executable.".bold()
    );

    println!(
        "{} ({} | {}) {}",
        "•".bold(),
        "--reloc [mode]"
            .custom_color(CustomColor::new(141, 141, 142))
            .bold(),
        "-reloc [mode]"
            .custom_color(CustomColor::new(141, 141, 142))
            .bold(),
        "Indicate how references to memory addresses are handled.".bold()
    );

    println!(
        "{} ({} | {}) {}",
        "•".bold(),
        "--codemodel [model]"
            .custom_color(CustomColor::new(141, 141, 142))
            .bold(),
        "-codemd [model]"
            .custom_color(CustomColor::new(141, 141, 142))
            .bold(),
        "Define how code is organized and accessed in the executable.".bold()
    );
}

pub const TARGETS: [&str; 240] = [
    "aarch64-apple-darwin",
    "aarch64-apple-ios",
    "aarch64-apple-ios-macabi",
    "aarch64-apple-ios-sim",
    "aarch64-apple-tvos",
    "aarch64-apple-tvos-sim",
    "aarch64-apple-visionos",
    "aarch64-apple-visionos-sim",
    "aarch64-apple-watchos",
    "aarch64-apple-watchos-sim",
    "aarch64-fuchsia",
    "aarch64-kmc-solid_asp3",
    "aarch64-linux-android",
    "aarch64-nintendo-switch-freestanding",
    "aarch64-pc-windows-gnullvm",
    "aarch64-pc-windows-msvc",
    "aarch64-unknown-freebsd",
    "aarch64-unknown-fuchsia",
    "aarch64-unknown-hermit",
    "aarch64-unknown-illumos",
    "aarch64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu_ilp32",
    "aarch64-unknown-linux-musl",
    "aarch64-unknown-linux-ohos",
    "aarch64-unknown-netbsd",
    "aarch64-unknown-none",
    "aarch64-unknown-none-softfloat",
    "aarch64-unknown-nto-qnx710",
    "aarch64-unknown-openbsd",
    "aarch64-unknown-redox",
    "aarch64-unknown-teeos",
    "aarch64-unknown-uefi",
    "aarch64-uwp-windows-msvc",
    "aarch64-wrs-vxworks",
    "aarch64_be-unknown-linux-gnu",
    "aarch64_be-unknown-linux-gnu_ilp32",
    "aarch64_be-unknown-netbsd",
    "arm-linux-androideabi",
    "arm-unknown-linux-gnueabi",
    "arm-unknown-linux-gnueabihf",
    "arm-unknown-linux-musleabi",
    "arm-unknown-linux-musleabihf",
    "arm64_32-apple-watchos",
    "arm64e-apple-darwin",
    "arm64e-apple-ios",
    "arm64ec-pc-windows-msvc",
    "armeb-unknown-linux-gnueabi",
    "armebv7r-none-eabi",
    "armebv7r-none-eabihf",
    "armv4t-none-eabi",
    "armv4t-unknown-linux-gnueabi",
    "armv5te-none-eabi",
    "armv5te-unknown-linux-gnueabi",
    "armv5te-unknown-linux-musleabi",
    "armv5te-unknown-linux-uclibceabi",
    "armv6-unknown-freebsd",
    "armv6-unknown-netbsd-eabihf",
    "armv6k-nintendo-3ds",
    "armv7-linux-androideabi",
    "armv7-sony-vita-newlibeabihf",
    "armv7-unknown-freebsd",
    "armv7-unknown-linux-gnueabi",
    "armv7-unknown-linux-gnueabihf",
    "armv7-unknown-linux-musleabi",
    "armv7-unknown-linux-musleabihf",
    "armv7-unknown-linux-ohos",
    "armv7-unknown-linux-uclibceabi",
    "armv7-unknown-linux-uclibceabihf",
    "armv7-unknown-netbsd-eabihf",
    "armv7-wrs-vxworks-eabihf",
    "armv7a-kmc-solid_asp3-eabi",
    "armv7a-kmc-solid_asp3-eabihf",
    "armv7a-none-eabi",
    "armv7a-none-eabihf",
    "armv7k-apple-watchos",
    "armv7r-none-eabi",
    "armv7r-none-eabihf",
    "armv7s-apple-ios",
    "armv8r-none-eabihf",
    "avr-unknown-gnu-atmega328",
    "bpfeb-unknown-none",
    "bpfel-unknown-none",
    "csky-unknown-linux-gnuabiv2",
    "csky-unknown-linux-gnuabiv2hf",
    "hexagon-unknown-linux-musl",
    "hexagon-unknown-none-elf",
    "i386-apple-ios",
    "i586-pc-nto-qnx700",
    "i586-pc-windows-msvc",
    "i586-unknown-linux-gnu",
    "i586-unknown-linux-musl",
    "i586-unknown-netbsd",
    "i686-apple-darwin",
    "i686-linux-android",
    "i686-pc-windows-gnu",
    "i686-pc-windows-gnullvm",
    "i686-pc-windows-msvc",
    "i686-unknown-freebsd",
    "i686-unknown-haiku",
    "i686-unknown-hurd-gnu",
    "i686-unknown-linux-gnu",
    "i686-unknown-linux-musl",
    "i686-unknown-netbsd",
    "i686-unknown-openbsd",
    "i686-unknown-uefi",
    "i686-uwp-windows-gnu",
    "i686-uwp-windows-msvc",
    "i686-win7-windows-msvc",
    "i686-wrs-vxworks",
    "loongarch64-unknown-linux-gnu",
    "loongarch64-unknown-linux-musl",
    "loongarch64-unknown-none",
    "loongarch64-unknown-none-softfloat",
    "m68k-unknown-linux-gnu",
    "mips-unknown-linux-gnu",
    "mips-unknown-linux-musl",
    "mips-unknown-linux-uclibc",
    "mips64-openwrt-linux-musl",
    "mips64-unknown-linux-gnuabi64",
    "mips64-unknown-linux-muslabi64",
    "mips64el-unknown-linux-gnuabi64",
    "mips64el-unknown-linux-muslabi64",
    "mipsel-sony-psp",
    "mipsel-sony-psx",
    "mipsel-unknown-linux-gnu",
    "mipsel-unknown-linux-musl",
    "mipsel-unknown-linux-uclibc",
    "mipsel-unknown-netbsd",
    "mipsel-unknown-none",
    "mipsisa32r6-unknown-linux-gnu",
    "mipsisa32r6el-unknown-linux-gnu",
    "mipsisa64r6-unknown-linux-gnuabi64",
    "mipsisa64r6el-unknown-linux-gnuabi64",
    "msp430-none-elf",
    "nvptx64-nvidia-cuda",
    "powerpc-unknown-freebsd",
    "powerpc-unknown-linux-gnu",
    "powerpc-unknown-linux-gnuspe",
    "powerpc-unknown-linux-musl",
    "powerpc-unknown-netbsd",
    "powerpc-unknown-openbsd",
    "powerpc-wrs-vxworks",
    "powerpc-wrs-vxworks-spe",
    "powerpc64-ibm-aix",
    "powerpc64-unknown-freebsd",
    "powerpc64-unknown-linux-gnu",
    "powerpc64-unknown-linux-musl",
    "powerpc64-unknown-openbsd",
    "powerpc64-wrs-vxworks",
    "powerpc64le-unknown-freebsd",
    "powerpc64le-unknown-linux-gnu",
    "powerpc64le-unknown-linux-musl",
    "riscv32gc-unknown-linux-gnu",
    "riscv32gc-unknown-linux-musl",
    "riscv32i-unknown-none-elf",
    "riscv32im-risc0-zkvm-elf",
    "riscv32im-unknown-none-elf",
    "riscv32ima-unknown-none-elf",
    "riscv32imac-esp-espidf",
    "riscv32imac-unknown-none-elf",
    "riscv32imac-unknown-xous-elf",
    "riscv32imafc-esp-espidf",
    "riscv32imafc-unknown-none-elf",
    "riscv32imc-esp-espidf",
    "riscv32imc-unknown-none-elf",
    "riscv64-linux-android",
    "riscv64gc-unknown-freebsd",
    "riscv64gc-unknown-fuchsia",
    "riscv64gc-unknown-hermit",
    "riscv64gc-unknown-linux-gnu",
    "riscv64gc-unknown-linux-musl",
    "riscv64gc-unknown-netbsd",
    "riscv64gc-unknown-none-elf",
    "riscv64gc-unknown-openbsd",
    "riscv64imac-unknown-none-elf",
    "s390x-unknown-linux-gnu",
    "s390x-unknown-linux-musl",
    "sparc-unknown-linux-gnu",
    "sparc-unknown-none-elf",
    "sparc64-unknown-linux-gnu",
    "sparc64-unknown-netbsd",
    "sparc64-unknown-openbsd",
    "sparcv9-sun-solaris",
    "thumbv4t-none-eabi",
    "thumbv5te-none-eabi",
    "thumbv6m-none-eabi",
    "thumbv7a-pc-windows-msvc",
    "thumbv7a-uwp-windows-msvc",
    "thumbv7em-none-eabi",
    "thumbv7em-none-eabihf",
    "thumbv7m-none-eabi",
    "thumbv7neon-linux-androideabi",
    "thumbv7neon-unknown-linux-gnueabihf",
    "thumbv7neon-unknown-linux-musleabihf",
    "thumbv8m.base-none-eabi",
    "thumbv8m.main-none-eabi",
    "thumbv8m.main-none-eabihf",
    "wasm32-unknown-emscripten",
    "wasm32-unknown-unknown",
    "wasm32-wasi",
    "wasm32-wasip1",
    "wasm32-wasip1-threads",
    "wasm32-wasip2",
    "wasm64-unknown-unknown",
    "x86_64-apple-darwin",
    "x86_64-apple-ios",
    "x86_64-apple-ios-macabi",
    "x86_64-apple-tvos",
    "x86_64-apple-watchos-sim",
    "x86_64-fortanix-unknown-sgx",
    "x86_64-fuchsia",
    "x86_64-linux-android",
    "x86_64-pc-nto-qnx710",
    "x86_64-pc-solaris",
    "x86_64-pc-windows-gnu",
    "x86_64-pc-windows-gnullvm",
    "x86_64-pc-windows-msvc",
    "x86_64-unikraft-linux-musl",
    "x86_64-unknown-dragonfly",
    "x86_64-unknown-freebsd",
    "x86_64-unknown-fuchsia",
    "x86_64-unknown-haiku",
    "x86_64-unknown-hermit",
    "x86_64-unknown-illumos",
    "x86_64-unknown-l4re-uclibc",
    "x86_64-unknown-linux-gnu",
    "x86_64-unknown-linux-gnux32",
    "x86_64-unknown-linux-musl",
    "x86_64-unknown-linux-none",
    "x86_64-unknown-linux-ohos",
    "x86_64-unknown-netbsd",
    "x86_64-unknown-none",
    "x86_64-unknown-openbsd",
    "x86_64-unknown-redox",
    "x86_64-unknown-uefi",
    "x86_64-uwp-windows-gnu",
    "x86_64-uwp-windows-msvc",
    "x86_64-win7-windows-msvc",
    "x86_64-wrs-vxworks",
    "x86_64h-apple-darwin",
];
