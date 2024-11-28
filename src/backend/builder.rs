use {
    super::{
        super::{logging, utils, BACKEND_COMPILER, NAME},
        compiler::CompilerOptions,
    },
    inkwell::module::Module,
    std::{env, fs, path::PathBuf, process::Command},
};

pub struct FileBuilder<'a, 'ctx> {
    module: &'a Module<'ctx>,
    options: &'a CompilerOptions,
    arguments: Vec<&'a str>,
    file: &'a str,
}

impl<'a, 'ctx> FileBuilder<'a, 'ctx> {
    pub fn new(options: &'a CompilerOptions, module: &'a Module<'ctx>, file: &'a str) -> Self {
        Self {
            options,
            module,
            arguments: Vec::new(),
            file,
        }
    }

    pub fn build(mut self) {
        if self.options.emit_llvm {
            self.module.print_to_file(self.file).unwrap();
            return;
        }

        self.module.print_to_file(self.file).unwrap();

        if self.options.emit_asm {
            self.emit_asm();
            return;
        }

        self.optimization(self.options.optimization.to_str(false, false));

        if self.options.executable {
            self.compile_to_executable();
        } else if self.options.library {
            self.compile_to_library();
        }

        if PathBuf::from(self.file).exists() {
            fs::remove_file(self.file).unwrap();
        }
    }

    fn optimization(&self, opt_level: &str) {
        self.handle_error(
            Command::new(PathBuf::from(BACKEND_COMPILER.lock().unwrap().as_str()).join("opt"))
                .arg(format!("-p={}", opt_level))
                .arg("-p=globalopt")
                .arg("-p=globaldce")
                .arg("-p=dce")
                .arg("-p=instcombine")
                .arg("-p=strip-dead-prototypes")
                .arg("-p=strip")
                .arg("-p=mem2reg")
                .arg("-p=memcpyopt")
                .arg(format!(
                    "{}.ll",
                    utils::extract_file_name(&NAME.lock().unwrap())
                ))
                .arg("-o")
                .arg(format!(
                    "{}.ll",
                    utils::extract_file_name(&NAME.lock().unwrap())
                )),
        );
    }

    fn emit_asm(&mut self) {
        self.arguments.extend(
            [
                self.options.optimization.to_str(true, false),
                "--asm-verbose",
                "--filetype=asm",
                self.file,
            ]
            .iter(),
        );

        self.options
            .extra_args
            .split_ascii_whitespace()
            .for_each(|arg| {
                self.arguments.insert(0, arg.trim());
            });

        self.handle_error(
            Command::new(PathBuf::from(BACKEND_COMPILER.lock().unwrap().as_str()).join("llc"))
                .args(&self.arguments),
        );
    }

    fn compile_to_executable(&mut self) {
        let home: String = match env::consts::OS {
            "windows" => env::var("APPDATA").unwrap(),
            "linux" => env::var("HOME").unwrap(),
            _ => {
                logging::log(logging::LogType::ERROR, "Compilation from unsupported OS.");
                return;
            }
        };

        let home: PathBuf = PathBuf::from(&home);

        if !home.exists() {
            logging::log(
                logging::LogType::ERROR,
                "The home of your system don't exists.",
            );
            return;
        } else if !home.join(".thrushc").exists() {
            logging::log(
                    logging::LogType::ERROR,
                    "The home folder for thrush lang don't exists. Use the command 'throium compiler restore' to restore it.",
                );
            return;
        } else if !home.join(".thrushc/natives/").exists() {
            logging::log(
                    logging::LogType::ERROR,
                    "The folder of thrushc with the the native apis don't exists. Use the command 'throium natives restore' to restore it.",
                );
            return;
        } else if !home.join(".thrushc/natives/vector.o").exists()
            && !self.options.restore_natives_apis
        {
            logging::log(
                    logging::LogType::ERROR,
                    "The file with the vector native api don't exists. Use the command 'throium natives restore' to restore it.",
                );
            return;
        }

        let mut default_args: Vec<&str> = Vec::from([
            "-opaque-pointers",
            self.options.linking.to_str(),
            "-ffast-math",
            self.file,
            "-o",
            &self.options.output,
        ]);

        if !self.options.insert_vector_natives {
            default_args.push(home.join(".thrushc/natives/vector.o").to_str().unwrap());
        }

        self.arguments.extend(
            [
                "-opaque-pointers",
                self.options.linking.to_str(),
                "-ffast-math",
                self.file,
                "-o",
                &self.options.output,
            ]
            .iter(),
        );

        self.options.extra_args.split(",").for_each(|arg| {
            self.arguments.insert(0, arg.trim());
        });

        self.handle_error(
            Command::new(PathBuf::from(BACKEND_COMPILER.lock().unwrap().as_str()).join("clang-18"))
                .args(&self.arguments),
        );
    }

    fn compile_to_library(&mut self) {
        self.arguments.extend(
            [
                "-opaque-pointers",
                self.options.linking.to_str(),
                "-ffast-math",
                "-c",
                self.file,
                "-o",
                &self.options.output,
            ]
            .iter(),
        );

        self.options.extra_args.split(",").for_each(|arg| {
            self.arguments.insert(0, arg.trim());
        });

        self.handle_error(
            Command::new(PathBuf::from(BACKEND_COMPILER.lock().unwrap().as_str()).join("clang-18"))
                .args(&self.arguments),
        );
    }

    #[inline]
    fn handle_error(&self, command: &mut Command) {
        if let Ok(child) = command.output() {
            if !child.status.success() {
                logging::log(
                    logging::LogType::ERROR,
                    &String::from_utf8_lossy(&child.stderr).replace("\n", ""),
                );
            }
        }
    }
}
