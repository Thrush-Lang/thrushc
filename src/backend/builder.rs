use {
    super::{
        super::{logging, LLVM_BACKEND_COMPILER},
        compiler::options::CompilerOptions,
    },
    inkwell::module::Module,
    regex::Regex,
    std::{fs, process::Command},
};

pub struct FileBuilder<'a, 'ctx> {
    module: &'a Module<'ctx>,
    options: &'a CompilerOptions,
    arguments: Vec<String>,
    from: &'a str,
    output: &'a str,
    args_regex: Regex,
}

impl<'a, 'ctx> FileBuilder<'a, 'ctx> {
    pub fn new(
        options: &'a CompilerOptions,
        module: &'a Module<'ctx>,
        from: &'a str,
        output: &'a str,
    ) -> Self {
        Self {
            options,
            module,
            arguments: Vec::new(),
            from,
            output,
            args_regex: Regex::new(r"!\((.*?)\)").unwrap(),
        }
    }

    pub fn build(mut self) {
        if self.options.emit_llvm {
            self.module.print_to_file(self.from).unwrap();
            return;
        }

        self.module.print_to_file(self.from).unwrap();

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

        let _ = fs::remove_file(self.from);
    }

    fn optimization(&self, opt_level: &str) {
        self.handle_error(
            Command::new(LLVM_BACKEND_COMPILER.as_ref().unwrap().join("opt"))
                .arg(format!("-p={}", opt_level))
                .arg("-p=globalopt")
                .arg("-p=globaldce")
                .arg("-p=dce")
                .arg("-p=instcombine")
                .arg("-p=strip-dead-prototypes")
                .arg("-p=strip")
                .arg("-p=mem2reg")
                .arg("-p=memcpyopt")
                .arg(self.from)
                .arg("-o")
                .arg(self.from),
        );
    }

    fn emit_asm(&mut self) {
        self.arguments.extend([
            self.options.optimization.to_string(true, false),
            "--asm-verbose".to_string(),
            "--filetype=asm".to_string(),
            self.from.to_string(),
        ]);

        self.parse_and_build_args();

        self.handle_error(
            Command::new(LLVM_BACKEND_COMPILER.as_ref().unwrap().join("llc")).args(&self.arguments),
        );
    }

    fn compile_to_executable(&mut self) {
        let mut default_args: Vec<String> = Vec::from([
            "-opaque-pointers".to_string(),
            self.options.linking.to_str().to_string(),
            "-ffast-math".to_string(),
            self.from.to_string(),
        ]);

        self.parse_and_build_args();

        default_args.extend(["-o".to_string(), self.output.to_string()]);

        self.arguments.extend(default_args);

        self.handle_error(
            Command::new(LLVM_BACKEND_COMPILER.as_ref().unwrap().join("clang-17"))
                .args(&self.arguments),
        );
    }

    fn compile_to_library(&mut self) {
        self.arguments.extend([
            "-opaque-pointers".to_string(),
            self.options.linking.to_str().to_string(),
            "-ffast-math".to_string(),
            "-c".to_string(),
            self.from.to_string(),
        ]);

        self.parse_and_build_args();

        self.arguments
            .extend(["-o".to_string(), self.output.to_string()]);

        self.handle_error(
            Command::new(LLVM_BACKEND_COMPILER.as_ref().unwrap().join("clang-17"))
                .args(&self.arguments),
        );
    }

    fn parse_and_build_args(&mut self) {
        let module_name: &str = self.module.get_name().to_str().unwrap();

        let extra_args = self.options.another_args.split(";").filter_map(|arg| {
            if let Some(cap) = self.args_regex.captures(arg) {
                if let Some(matched) = cap.get(1) {
                    let files: Vec<&str> = matched.as_str().split(',').map(str::trim).collect();

                    if files.contains(&module_name) {
                        return None;
                    }

                    return Some(arg.replace(&format!("!({})", matched.as_str()), ""));
                }
            }

            Some(arg.to_string())
        });

        self.arguments.extend(extra_args);
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
