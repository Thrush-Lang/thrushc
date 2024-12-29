#![allow(clippy::upper_case_acronyms)]

use {
    super::{
        super::{logging, LLVM_BACKEND_COMPILER},
        compiler::options::CompilerOptions,
    },
    std::{path::PathBuf, process::Command},
};

pub struct Clang<'a> {
    files: &'a [PathBuf],
    options: &'a CompilerOptions,
}

impl<'a> Clang<'a> {
    pub fn new(files: &'a [PathBuf], options: &'a CompilerOptions) -> Self {
        Self { files, options }
    }

    pub fn compile(&self) {
        let mut clang_command: Command = Command::new(LLVM_BACKEND_COMPILER.join("clang-17"));

        if self.options.executable {
            clang_command.args([
                "-v",
                "-opaque-pointers",
                self.options.linking.to_str(),
                self.options.optimization.to_str(true, false),
            ]);
        } else {
            let library_variant: &str = if self.options.library {
                "-c"
            } else {
                "--emit-static-lib"
            };

            clang_command.args([
                "-v",
                "-opaque-pointers",
                self.options.linking.to_str(),
                self.options.optimization.to_str(true, false),
                library_variant,
            ]);
        }

        clang_command.args(self.files);

        clang_command.args(&self.options.args);

        clang_command.args(["-o", &self.options.output]);

        handle_command(&mut clang_command);
    }
}

pub struct LLC<'a> {
    files: &'a [PathBuf],
    options: &'a CompilerOptions,
}

impl<'a> LLC<'a> {
    pub fn new(files: &'a [PathBuf], options: &'a CompilerOptions) -> Self {
        Self { files, options }
    }

    pub fn compile(&self) {
        let mut llc_command: Command = Command::new(LLVM_BACKEND_COMPILER.join("llc"));

        llc_command.args([
            self.options.optimization.to_str(true, false),
            "--asm-verbose",
            "--filetype=asm",
        ]);

        llc_command.args(self.files);

        handle_command(&mut llc_command);
    }
}

pub struct LLVMOptimization;

impl LLVMOptimization {
    pub fn optimize(path: &str, opt: &str, opt_lto: &str) {
        handle_command(
            Command::new(LLVM_BACKEND_COMPILER.join("opt"))
                .arg(format!("-p={}", opt))
                .arg("-p=globalopt")
                .arg("-p=globaldce")
                .arg("-p=dce")
                .arg("-p=instcombine")
                .arg("-p=agressive-instcombine")
                .arg("-p=loop-deletion")
                .arg("-p=loop-simplify")
                .arg("-p=strip-dead-prototypes")
                .arg("-p=strip-dead-debug-info")
                .arg("-p=strip")
                .arg("-p=mem2reg")
                .arg("-p=memcpyopt")
                .arg("-p=inline")
                .arg("-p=loop-simplifycfg")
                .arg("-p=instsimplify")
                .arg("-p=loop-instsimplify")
                .arg("-p=simplifycfg")
                .arg(path)
                .arg("-o")
                .arg(path),
        );

        handle_command(
            Command::new(LLVM_BACKEND_COMPILER.join("llvm-lto"))
                .arg(opt_lto)
                .arg(path),
        );
    }
}

#[inline]
fn handle_command(command: &mut Command) {
    if let Ok(child) = command.output() {
        if !child.status.success() {
            logging::log(
                logging::LogType::ERROR,
                &String::from_utf8_lossy(&child.stderr).replace("\n", ""),
            );
        }
    }
}
