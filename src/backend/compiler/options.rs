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

#[derive(Default, Debug, PartialEq)]
pub enum Linking {
    Static,
    #[default]
    Dynamic,
}

#[derive(Debug)]
pub struct CompilerOptions {
    pub output: String,
    pub target_triple: TargetTriple,
    pub optimization: Opt,
    pub emit_llvm: bool,
    pub emit_asm: bool,
    pub library: bool,
    pub executable: bool,
    pub linking: Linking,
    pub is_main: bool,
    pub include_vector_api: bool,
    pub include_debug_api: bool,
    pub delete_built_in_apis_after: bool,
    pub reloc_mode: RelocMode,
    pub code_model: CodeModel,
    pub file_path: String,
    pub another_args: String,
}

impl Default for CompilerOptions {
    fn default() -> Self {
        Self {
            output: String::new(),
            target_triple: TargetMachine::get_default_triple(),
            optimization: Opt::default(),
            emit_llvm: false,
            emit_asm: false,
            library: false,
            executable: false,
            linking: Linking::default(),
            is_main: false,
            include_vector_api: false,
            include_debug_api: false,
            delete_built_in_apis_after: false,
            reloc_mode: RelocMode::Default,
            code_model: CodeModel::Default,
            file_path: String::new(),
            another_args: String::new(),
        }
    }
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

    #[inline]
    pub fn to_string(&self, single_slash: bool, double_slash: bool) -> String {
        match self {
            Opt::None if !single_slash && !double_slash => String::from("O0"),
            Opt::Low if !single_slash && !double_slash => String::from("O1"),
            Opt::Mid if !single_slash && !double_slash => String::from("O2"),
            Opt::Mcqueen if !single_slash && !double_slash => String::from("O3"),
            Opt::None if single_slash => String::from("-O0"),
            Opt::Low if single_slash => String::from("-O1"),
            Opt::Mid if single_slash => String::from("-O2"),
            Opt::Mcqueen if single_slash => String::from("-O3"),
            Opt::None if double_slash => String::from("-O0"),
            Opt::Low if double_slash => String::from("-O1"),
            Opt::Mid if double_slash => String::from("-O2"),
            Opt::Mcqueen if double_slash => String::from("-O3"),
            _ if single_slash => String::from("-O0"),
            _ if double_slash => String::from("--O0"),
            _ => String::from("O0"),
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
}

impl Linking {
    pub fn to_str(&self) -> &str {
        match self {
            Linking::Static => "--static",
            Linking::Dynamic => "-dynamic",
        }
    }
}
