use {
    super::super::{logging, BACKEND},
    super::compiler::{CompilerOptions, Linking, Opt},
    inkwell::module::Module,
    std::{fs, path::Path, process::Command},
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
        let opt_level: &str = match self.options.optimization {
            Opt::None => "O0",
            Opt::Low => "O1",
            Opt::Mid => "O2",
            Opt::Mcqueen => "O3",
        };

        let linking: &str = match self.options.linking {
            Linking::Static => "--static",
            Linking::Dynamic => "-dynamic",
        };

        if self.options.emit_llvm {
            self.module
                .print_to_file(format!("{}.ll", self.options.name))
                .unwrap();
            return;
        }

        self.module
            .write_bitcode_to_path(Path::new(&format!("{}.bc", self.options.name)));

        match Command::new(Path::new(&BACKEND.lock().unwrap().as_str()).join("clang-18")).spawn() {
            Ok(mut child) => {
                child.kill().unwrap();

                if self.options.build {
                    match self.opt(opt_level) {
                        Ok(()) => {
                            Command::new(
                                Path::new(&BACKEND.lock().unwrap().as_str()).join("clang-18"),
                            )
                            .arg("-opaque-pointers")
                            .arg(linking)
                            .arg("-ffast-math")
                            .arg(format!("{}.bc", self.options.name))
                            .arg("-o")
                            .arg(self.options.name.as_str())
                            .output()
                            .unwrap();
                        }
                        Err(error) => {
                            logging::log(logging::LogType::ERROR, &error);
                            return;
                        }
                    }
                } else if self.options.emit_object {
                    match self.opt(opt_level) {
                        Ok(()) => {
                            Command::new(
                                Path::new(&BACKEND.lock().unwrap().as_str()).join("clang-18"),
                            )
                            .arg("-opaque-pointers")
                            .arg(linking)
                            .arg("-ffast-math")
                            .arg("-c")
                            .arg(format!("{}.bc", self.options.name))
                            .arg("-o")
                            .arg(format!("{}.o", self.options.name))
                            .output()
                            .unwrap();
                        }
                        Err(error) => {
                            logging::log(logging::LogType::ERROR, &error);
                            return;
                        }
                    }
                }

                fs::remove_file(format!("{}.bc", self.options.name)).unwrap();
            }
            Err(_) => {
                logging::log(
                    logging::LogType::ERROR,
                    "Compilation failed. Does can't accesed to Clang 18.",
                );
            }
        }
    }

    fn opt(&self, opt_level: &str) -> Result<(), String> {
        match Command::new(Path::new(&BACKEND.lock().unwrap().as_str()).join("opt")).spawn() {
            Ok(mut child) => {
                child.kill().unwrap();

                Command::new("opt")
                    .arg(format!("-p={}", opt_level))
                    .arg("-p=globalopt")
                    .arg("-p=globaldce")
                    .arg("-p=dce")
                    .arg("-p=instcombine")
                    .arg("-p=strip-dead-prototypes")
                    .arg("-p=strip")
                    .arg("-p=mem2reg")
                    .arg("-p=memcpyopt")
                    .arg(format!("{}.bc", self.options.name))
                    .output()
                    .unwrap();

                Ok(())
            }

            Err(_) => Err(String::from(
                "Compilation failed. Does can't accesed to LLVM Optimizer.",
            )),
        }
    }
}
