use {
    super::{
        super::{logging, utils, BACKEND_COMPILER, NAME},
        compiler::CompilerOptions,
    },
    inkwell::module::Module,
    std::{env, fs, path::Path, process::Command},
};

pub struct FileBuilder<'a, 'ctx> {
    module: &'a Module<'ctx>,
    options: &'a CompilerOptions,
}

impl<'a, 'ctx> FileBuilder<'a, 'ctx> {
    pub fn new(options: &'a CompilerOptions, module: &'a Module<'ctx>) -> Self {
        Self { options, module }
    }

    pub fn build(self) {
        let linking: &str = self.options.linking.to_str();
        let file: String = format!("{}.ll", utils::extract_file_name(&NAME.lock().unwrap()));

        if self.options.emit_llvm {
            self.module.print_to_file(file).unwrap();
            return;
        }

        self.module.print_to_file(&file).unwrap();

        self.optimization(self.options.optimization.to_str());

        if self.options.executable {
            let home: String = match env::consts::OS {
                "windows" => env::var("APPDATA").unwrap(),
                "linux" => env::var("HOME").unwrap(),
                _ => panic!("Unsupported OS"),
            };

            let home: &Path = Path::new(&home);

            if !home.exists() {
                logging::log(
                    logging::LogType::ERROR,
                    "The home of your system don't exists.",
                );
                return;
            }

            if !home.join(".thrushc").exists() {
                logging::log(
                    logging::LogType::ERROR,
                    "The home folder for thrush lang don't exists. Use the command 'throium compiler restore' to restore it.",
                );
                return;
            }

            if !home.join(".thrushc/natives/").exists() {
                logging::log(
                    logging::LogType::ERROR,
                    "The folder of thrushc with the the native apis don't exists. Use the command 'throium natives restore' to restore it.",
                );
                return;
            }

            if !home.join(".thrushc/natives/vector.o").exists()
                && !self.options.restore_natives_apis
            {
                logging::log(
                    logging::LogType::ERROR,
                    "The file with the vector native api don't exists. Use the command 'throium natives restore' to restore it.",
                );
                return;
            }

            if self.options.insert_vector_natives {
                self.handle_error(
                    Command::new(
                        Path::new(BACKEND_COMPILER.lock().unwrap().as_str()).join("clang-18"),
                    )
                    .arg("-opaque-pointers")
                    .arg(linking)
                    .arg("-ffast-math")
                    .arg(&file)
                    .arg("-o")
                    .arg(&self.options.output),
                );
            } else {
                self.handle_error(
                    Command::new(
                        Path::new(BACKEND_COMPILER.lock().unwrap().as_str()).join("clang-18"),
                    )
                    .arg("-opaque-pointers")
                    .arg(linking)
                    .arg("-ffast-math")
                    .arg(&file)
                    .arg("-o")
                    .arg(&self.options.output)
                    .arg(home.join(".thrushc/natives/vector.o").to_str().unwrap()),
                );
            }
        } else if self.options.library {
            self.handle_error(
                Command::new(Path::new(BACKEND_COMPILER.lock().unwrap().as_str()).join("clang-18"))
                    .arg("-opaque-pointers")
                    .arg(linking)
                    .arg("-ffast-math")
                    .arg("-c")
                    .arg(&file)
                    .arg("-o")
                    .arg(&self.options.output),
            );
        }

        if Path::new(&file).exists() {
            fs::remove_file(&file).unwrap();
        }
    }

    fn optimization(&self, opt_level: &str) {
        self.handle_error(
            Command::new(Path::new(BACKEND_COMPILER.lock().unwrap().as_str()).join("opt"))
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

    fn handle_error(&self, command: &mut Command) {
        if let Ok(mut child) = command.spawn() {
            child.wait().unwrap();
        } else if let Err(error) = command.spawn() {
            logging::log(logging::LogType::ERROR, &error.to_string());
        }
    }
}
