pub mod logging;
pub mod utils;

use {
    super::compiler::{
        Compiler, CompilerOptions, OptimizationLevel, ThrushFile, FILE_NAME_WITH_EXT, FILE_PATH,
    },
    ahash::HashSet,
    colored::Colorize,
    inkwell::targets::{TargetMachine, TargetTriple},
    logging::Logging,
    std::{
        mem,
        path::{Path, PathBuf},
    },
};

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

pub struct Cli {
    args: Vec<String>,
    options: CompilerOptions,
    targets: HashSet<&'static str>,
}

impl Cli {
    pub fn new(mut args: Vec<String>) -> Self {
        args.remove(0);

        Self {
            args,
            options: CompilerOptions::default(),
            targets: HashSet::from_iter(TARGETS),
        }
    }

    pub fn eval(&mut self) {
        match self.args.len() {
            arg if arg >= 1 => match self.args[0].as_str() {
                "-h" | "help" => {
                    self.help();
                }
                "-pt" | "print-targets" => self.targets.iter().for_each(|target| {
                    println!("{}", target.bold());
                }),
                "-pnt" | "print-native-target" => {
                    println!("{}", TargetMachine::get_default_triple().to_string().bold());
                }
                "-v" | "version" => {
                    println!("{}", env!("CARGO_PKG_VERSION").bold());
                }
                "-c" | "compile" => {
                    if self.args.len() == 1 {
                        self.compile_help();
                        return;
                    }

                    let index: usize = self.args.len() - 1;
                    let path: &Path = Path::new(&self.args[index]);

                    if !path.exists() {
                        Logging::new(format!(
                            "The path or file '{}' cannot be accessed.",
                            self.args[index]
                        ))
                        .error();
                        return;
                    }

                    if !path.is_file() {
                        Logging::new(format!(
                            "The path '{}' ended with not a file.",
                            self.args[index]
                        ))
                        .error();
                        return;
                    }

                    if path.extension().is_none() {
                        Logging::new(format!(
                            "The file in path '{}' does not have an extension.",
                            self.args[index]
                        ))
                        .error();
                        return;
                    }

                    if path.extension().unwrap() != "th" {
                        Logging::new(format!(
                            "The file in path '{}' does not have the extension '.th'.",
                            self.args[index]
                        ))
                        .error();
                        return;
                    }

                    for i in 1..self.args.len() - 1 {
                        match self.args[i].as_str() {
                            "--name" | "-n" => {
                                self.options.name = mem::take(&mut self.args[i + 1]);
                            }
                            "--target" | "-t" => {
                                if self.targets.contains(&self.args[i + 1].as_str()) {
                                    self.options.target = TargetTriple::create(&self.args[i + 1]);
                                    continue;
                                }

                                Logging::new(format!(
                                    "The target '{}' is not supported, see the list with Thrushr --print-targets.",
                                    self.args[i + 1]
                                ))
                                .error();
                                return;
                            }
                            "--optimization" | "-opt" => match self.args[i + 1].as_str() {
                                "none" => {
                                    self.options.optimization = OptimizationLevel::None;
                                }
                                "low" => {
                                    self.options.optimization = OptimizationLevel::Low;
                                }
                                "mid" => {
                                    self.options.optimization = OptimizationLevel::Mid;
                                }
                                "mcqueen" => {
                                    self.options.optimization = OptimizationLevel::Mcqueen;
                                }
                                _ => {
                                    self.options.optimization = OptimizationLevel::None;
                                }
                            },
                            "--emit-llvm" | "-emit-llvm" => {
                                self.options.emit_llvm = true;
                            }
                            "--build" | "-b" => {
                                self.options.build = true;
                            }

                            _ => continue,
                        }
                    }

                    FILE_NAME_WITH_EXT.lock().unwrap().push_str(
                        Path::new(&self.args[index])
                            .file_name()
                            .unwrap()
                            .to_str()
                            .unwrap(),
                    );

                    FILE_PATH.lock().unwrap().push_str(
                        Path::new(&self.args[index])
                            .to_path_buf()
                            .display()
                            .to_string()
                            .as_str(),
                    );

                    Compiler::new(
                        mem::take(&mut self.options),
                        ThrushFile {
                            is_main: self.is_main(Path::new(&self.args[index])),
                            path: PathBuf::from(&self.args[index]),
                            name: Path::new(&self.args[index])
                                .file_name()
                                .unwrap()
                                .to_str()
                                .unwrap()
                                .split(".")
                                .collect::<Vec<_>>()[0]
                                .to_string(),
                        },
                    )
                    .compile();
                }
                "-i" | "interpret" => {}
                _ => {
                    self.help();
                }
            },
            _ => {
                self.help();
            }
        }
    }

    fn is_main(&self, path: &Path) -> bool {
        if path.file_stem().unwrap().to_str().unwrap() == "main" {
            return true;
        }

        false
    }

    fn help(&self) {
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
            "print-targets".bold().bright_cyan(),
            "-pt".bold().bright_cyan(),
            "Print the list of supported targets machines.".bold()
        );

        println!(
            "{} ({} | {}) {}",
            "•".bold(),
            "print-native-target".bold().bright_cyan(),
            "-pnt".bold().bright_cyan(),
            "Print the native target of this machine.".bold()
        )
    }

    fn compile_help(&self) {
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
            "--build".bold().bright_cyan(),
            "-b".bold().bright_cyan(),
            "Compile the code into executable.".bold()
        );
    }
}
