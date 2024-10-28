use {
    super::{super::logging, compiler::CompilerOptions},
    inkwell::module::Module,
    std::{
        env, fs,
        path::{Path, PathBuf},
        process::Command,
    },
};

pub struct FileBuilder<'a, 'ctx> {
    module: &'a Module<'ctx>,
    options: &'a CompilerOptions,
    backend: &'a str,
}

impl<'a, 'ctx> FileBuilder<'a, 'ctx> {
    pub fn new(options: &'a CompilerOptions, module: &'a Module<'ctx>, backend: &'a str) -> Self {
        Self {
            options,
            module,
            backend,
        }
    }

    pub fn build(self) {
        let linking: &str = self.options.linking.to_str();
        let file: String = format!("{}.ll", self.options.name);

        if self.options.emit_llvm {
            self.module.print_to_file(file).unwrap();
            return;
        }

        self.module.print_to_file(&file).unwrap();

        self.optimization(self.options.optimization.to_str());

        if self.options.build {
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

            if !home.join("thrushlang").exists() {
                logging::log(
                    logging::LogType::ERROR,
                    "The home folder for thrush lang don't exists. Use the command 'throium config repair' to restore it.",
                );
                return;
            }

            if !home.join("thrushlang/apis/").exists() {
                logging::log(
                    logging::LogType::ERROR,
                    "The folder of thrush lang with the compiler apis don't exists. Use the command 'throium config repair' to restore it.",
                );
                return;
            }

            if !home.join("thrushlang/apis/vectorapi.o").exists() && !self.options.rebuild_apis {
                logging::log(
                    logging::LogType::ERROR,
                    "The file with the vector api don't exists. Use the command 'throium config repair' to restore it.",
                );
                return;
            }

            if self.options.include_vec_api {
                self.handle_error(
                    Command::new(Path::new(self.backend).join("clang-18"))
                        .arg("-opaque-pointers")
                        .arg(linking)
                        .arg("-ffast-math")
                        .arg(&file)
                        .arg("-o")
                        .arg(&self.options.name),
                );
            } else {
                self.handle_error(
                    Command::new(Path::new(self.backend).join("clang-18"))
                        .arg("-opaque-pointers")
                        .arg(linking)
                        .arg("-ffast-math")
                        .arg(&file)
                        .arg("-o")
                        .arg(&self.options.name)
                        .arg(home.join("thrushlang/apis/vectorapi.o").to_str().unwrap()),
                );
            }
        } else if self.options.emit_object {
            self.handle_error(
                Command::new(Path::new(self.backend).join("clang-18"))
                    .arg("-opaque-pointers")
                    .arg(linking)
                    .arg("-ffast-math")
                    .arg("-c")
                    .arg(&file)
                    .arg("-o")
                    .arg(format!("{}.o", self.options.name)),
            );
        }

        if Path::new(&file).exists() {
            fs::remove_file(&file).unwrap();
        }
    }

    fn optimization(&self, opt_level: &str) {
        self.handle_error(
            Command::new(Path::new(self.backend).join("opt"))
                .arg(format!("-p={}", opt_level))
                .arg("-p=globalopt")
                .arg("-p=globaldce")
                .arg("-p=dce")
                .arg("-p=instcombine")
                .arg("-p=strip-dead-prototypes")
                .arg("-p=strip")
                .arg("-p=mem2reg")
                .arg("-p=memcpyopt")
                .arg(format!("{}.ll", self.options.name))
                .arg("-o")
                .arg(format!("{}.ll", self.options.name)),
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
