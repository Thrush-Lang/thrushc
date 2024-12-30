use std::path::PathBuf;

use inkwell::{
    targets::{CodeModel, RelocMode, TargetMachine, TargetTriple},
    OptimizationLevel,
};

#[derive(Default, Debug)]
pub enum Opt {
    #[default]
    None,
    Low,
    Mid,
    Mcqueen,
}

impl Opt {
    #[inline]
    pub fn to_str(&self, single_slash: bool, double_slash: bool) -> &str {
        match self {
            Opt::None if !single_slash && !double_slash => "O0",
            Opt::Low if !single_slash && !double_slash => "O1",
            Opt::Mid if !single_slash && !double_slash => "O2",
            Opt::Mcqueen if !single_slash && !double_slash => "O3",
            Opt::None if single_slash => "-O0",
            Opt::Low if single_slash => "-O1",
            Opt::Mid if single_slash => "-O2",
            Opt::Mcqueen if single_slash => "-O3",
            Opt::None if double_slash => "-O0",
            Opt::Low if double_slash => "-O1",
            Opt::Mid if double_slash => "-O2",
            Opt::Mcqueen if double_slash => "-O3",
            _ if single_slash => "-O0",
            _ if double_slash => "--O0",
            _ => "O0",
        }
    }

    pub fn to_llvm_opt(&self) -> OptimizationLevel {
        match self {
            Opt::None => OptimizationLevel::None,
            Opt::Low => OptimizationLevel::Default,
            Opt::Mid => OptimizationLevel::Less,
            Opt::Mcqueen => OptimizationLevel::Aggressive,
        }
    }

    #[inline]
    pub fn to_llvm_17_passes(&self) -> &str {
        match self {
            Opt::None => "default<O0>",
            Opt::Low => "default<O1>",
            Opt::Mid => "default<O2>",
            Opt::Mcqueen => "default<O3>",
        }
    }
}

#[derive(Default, Debug, PartialEq)]
pub enum Linking {
    Static,
    #[default]
    Dynamic,
}

impl Linking {
    pub fn to_str(&self) -> &str {
        match self {
            Linking::Static => "--static",
            Linking::Dynamic => "-dynamic",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ThrushFile {
    pub name: String,
    pub path: PathBuf,
    pub is_main: bool,
}

impl ThrushFile {
    pub fn new(name: String, path: PathBuf, is_main: bool) -> Self {
        Self {
            name,
            path,
            is_main,
        }
    }
}

#[derive(Debug)]
pub struct CompilerOptions {
    pub output: String,
    pub target_triple: TargetTriple,
    pub optimization: Opt,
    pub emit_llvm_ir: bool,
    pub emit_llvm_bitcode: bool,
    pub emit_asm: bool,
    pub library: bool,
    pub static_library: bool,
    pub executable: bool,
    pub linking: Linking,
    pub include_vector_api: bool,
    pub include_debug_api: bool,
    pub reloc_mode: RelocMode,
    pub code_model: CodeModel,
    pub files: Vec<ThrushFile>,
    pub args: Vec<String>,
}

impl Default for CompilerOptions {
    fn default() -> Self {
        Self {
            output: String::new(),
            target_triple: TargetMachine::get_default_triple(),
            optimization: Opt::default(),
            emit_llvm_ir: false,
            emit_llvm_bitcode: false,

            emit_asm: false,
            library: false,
            static_library: false,
            executable: false,
            linking: Linking::default(),
            include_vector_api: false,
            include_debug_api: false,
            reloc_mode: RelocMode::Default,
            code_model: CodeModel::Default,
            files: Vec::new(),
            args: Vec::new(),
        }
    }
}
