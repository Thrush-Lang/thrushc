use {
    super::{
        super::{frontend::lexer::DataTypes, logging::Logging},
        llvm::{
            build_alloca_with_float, build_alloca_with_integer, build_const_float,
            build_const_integer, build_int_array_type_from_size, datatype_float_to_type,
            datatype_integer_to_type, datatype_to_fn_type, set_globals_options,
        },
    },
    inkwell::{
        basic_block::BasicBlock,
        builder::Builder,
        context::Context,
        module::{Linkage, Module},
        targets::{TargetMachine, TargetTriple},
        types::{ArrayType, FunctionType, IntType},
        values::{
            BasicMetadataValueEnum, BasicValue, BasicValueEnum, FunctionValue, GlobalValue,
            InstructionValue, PointerValue,
        },
        AddressSpace,
    },
    std::{
        collections::HashMap,
        fs::remove_file,
        path::{Path, PathBuf},
        process::Command,
    },
};

pub struct Compiler<'a, 'ctx> {
    module: &'a Module<'ctx>,
    builder: &'a Builder<'ctx>,
    context: &'ctx Context,
    instructions: &'ctx [Instruction<'ctx>],
    current: usize,
    locals: HashMap<String, Instruction<'ctx>>,
}

impl<'a, 'ctx> Compiler<'a, 'ctx> {
    pub fn compile(
        module: &'a Module<'ctx>,
        builder: &'a Builder<'ctx>,
        context: &'ctx Context,
        instructions: &'ctx [Instruction<'ctx>],
    ) {
        Self {
            module,
            builder,
            context,
            instructions,
            current: 0,
            locals: HashMap::new(),
        }
        .start();
    }

    fn start(&mut self) {
        self.define_standard_functions();

        while !self.is_end() {
            let instr: &Instruction<'_> = self.advance();
            self.codegen(instr);
        }
    }

    fn codegen(&mut self, instr: &'ctx Instruction<'ctx>) {
        match instr {
            Instruction::Block(body) => {
                self.begin_scope();

                body.iter().for_each(|instr| {
                    self.codegen(instr);
                });

                self.end_scope();
            }

            Instruction::Function {
                name,
                params,
                body,
                return_kind,
                is_public,
            } => {
                self.emit_function(name, params, body, return_kind, *is_public);
            }

            Instruction::Return(instr) => {
                self.emit_return(instr);
            }

            Instruction::String(string) => {
                self.emit_global_string_constant(string, "");
            }

            Instruction::Println(args) => {
                self.emit_println(args);
            }

            Instruction::Puts(instr) => {
                self.emit_puts(instr);
            }

            Instruction::Var { name, kind, value } => match value {
                Some(value) => {
                    self.emit_variable(name, kind, value);
                }
                None => (),
            },

            Instruction::EntryPoint { body } => {
                self.emit_main();
                self.codegen(body);
                self.build_const_integer_return(self.context.i32_type(), 0, false);
            }

            Instruction::End => (),

            _ => todo!(),
        }
    }

    fn define_standard_functions(&mut self) {
        self.define_puts();
        self.define_printf();
    }

    fn define_printf(&mut self) {
        let printf: FunctionType = self.context.i32_type().fn_type(
            &[self.context.ptr_type(AddressSpace::default()).into()],
            true,
        );
        self.module
            .add_function("printf", printf, Some(Linkage::External));
    }

    fn define_puts(&mut self) {
        let puts: FunctionType = self.context.i32_type().fn_type(
            &[self.context.ptr_type(AddressSpace::default()).into()],
            true,
        );

        self.module
            .add_function("puts", puts, Some(Linkage::External));
    }

    fn emit_main(&mut self) {
        let main_kind: FunctionType = self.context.i32_type().fn_type(&[], false);
        let main: FunctionValue = self.module.add_function("main", main_kind, None);

        let entry_point: BasicBlock = self.context.append_basic_block(main, "");

        self.builder.position_at_end(entry_point);
    }

    fn emit_println(&mut self, instrs: &[Instruction]) {
        let mut args: Vec<BasicMetadataValueEnum> = Vec::with_capacity(24);

        instrs.iter().for_each(|instr| match instr {
            Instruction::String(string) => {
                let kind: ArrayType<'_> = build_int_array_type_from_size(
                    self.context,
                    DataTypes::I8,
                    string.len() as u32,
                );

                let global: GlobalValue<'ctx> =
                    self.module
                        .add_global(kind, Some(AddressSpace::default()), "");

                set_globals_options(self.context, global, Some(instr));

                args.push(
                    self.builder
                        .build_pointer_cast(
                            global.as_pointer_value(),
                            self.context.ptr_type(AddressSpace::default()),
                            "",
                        )
                        .unwrap()
                        .into(),
                );
            }

            Instruction::Integer(kind, num) => {
                args.push(build_const_integer(self.context, kind, *num).into());
            }

            Instruction::RefVar { name, scope } => match scope {
                Scope::Global => {
                    todo!()
                }

                Scope::Local => {
                    let var: &Instruction<'_> = self.get_local(name);

                    if let Instruction::Value(pointer) = var {
                        match pointer {
                            BasicValueEnum::IntValue(value) => {
                                args.push((*value).into());
                            }

                            _ => todo!(),
                        }
                    }
                }
            },

            _ => todo!(),
        });

        self.builder
            .build_call(self.module.get_function("printf").unwrap(), &args, "")
            .unwrap();
    }

    fn emit_puts(&mut self, instr: &Instruction) {
        let pointer: PointerValue<'ctx> = match instr {
            Instruction::String(string) => {
                let kind: ArrayType<'_> = build_int_array_type_from_size(
                    self.context,
                    DataTypes::I8,
                    string.len() as u32,
                );

                let global: GlobalValue<'ctx> =
                    self.module
                        .add_global(kind, Some(AddressSpace::default()), "");

                set_globals_options(self.context, global, Some(instr));

                self.builder
                    .build_pointer_cast(
                        global.as_pointer_value(),
                        self.context.ptr_type(AddressSpace::default()),
                        "",
                    )
                    .unwrap()
            }

            _ => todo!(),
        };

        self.builder
            .build_call(
                self.module.get_function("puts").unwrap(),
                &[BasicMetadataValueEnum::PointerValue(pointer)],
                "",
            )
            .unwrap();
    }

    fn emit_variable(&mut self, name: &str, kind: &DataTypes, value: &Instruction) {
        let ptr_kind: IntType<'_> = datatype_integer_to_type(self.context, kind);

        let ptr: PointerValue<'_> = match kind {
            DataTypes::I8 => build_alloca_with_integer(
                self.builder,
                datatype_integer_to_type(self.context, kind),
            ),

            DataTypes::I16 => build_alloca_with_integer(
                self.builder,
                datatype_integer_to_type(self.context, kind),
            ),

            DataTypes::I32 => build_alloca_with_integer(
                self.builder,
                datatype_integer_to_type(self.context, kind),
            ),

            DataTypes::I64 => build_alloca_with_integer(
                self.builder,
                datatype_integer_to_type(self.context, kind),
            ),

            DataTypes::U8 => build_alloca_with_integer(
                self.builder,
                datatype_integer_to_type(self.context, kind),
            ),

            DataTypes::U16 => build_alloca_with_integer(
                self.builder,
                datatype_integer_to_type(self.context, kind),
            ),

            DataTypes::U32 => build_alloca_with_integer(
                self.builder,
                datatype_integer_to_type(self.context, kind),
            ),

            DataTypes::U64 => build_alloca_with_integer(
                self.builder,
                datatype_integer_to_type(self.context, kind),
            ),

            DataTypes::F32 => {
                build_alloca_with_float(self.builder, datatype_float_to_type(self.context, kind))
            }

            DataTypes::F64 => {
                build_alloca_with_float(self.builder, datatype_float_to_type(self.context, kind))
            }

            _ => todo!(),
        };

        match value {
            Instruction::Null => {}

            Instruction::Integer(kind, num) => match kind {
                DataTypes::I8
                | DataTypes::I16
                | DataTypes::I32
                | DataTypes::I64
                | DataTypes::U8
                | DataTypes::U16
                | DataTypes::U32
                | DataTypes::U64 => {
                    let store: InstructionValue<'_> = self
                        .builder
                        .build_store(ptr, build_const_integer(self.context, kind, *num))
                        .unwrap();

                    store.set_alignment(4).unwrap();
                }

                DataTypes::F32 | DataTypes::F64 => {
                    let store: InstructionValue<'_> = self
                        .builder
                        .build_store(ptr, build_const_float(self.context, kind, *num))
                        .unwrap();

                    store.set_alignment(4).unwrap();
                }

                _ => todo!(),
            },

            _ => todo!(),
        }

        let load: BasicValueEnum<'ctx> = self.builder.build_load(ptr_kind, ptr, name).unwrap();

        load.as_instruction_value()
            .unwrap()
            .set_alignment(4)
            .unwrap();

        self.locals
            .insert(name.to_string(), Instruction::Value(load));
    }

    fn emit_return(&mut self, instr: &Instruction) {
        match &instr {
            Instruction::Null => {}
            Instruction::Integer(kind, num) => {
                self.builder
                    .build_return(Some(&build_const_integer(self.context, kind, *num)))
                    .unwrap();
            }

            Instruction::String(string) => {
                let kind: ArrayType<'_> = build_int_array_type_from_size(
                    self.context,
                    DataTypes::I8,
                    string.len() as u32,
                );

                let global: GlobalValue<'ctx> =
                    self.module
                        .add_global(kind, Some(AddressSpace::default()), "");

                set_globals_options(self.context, global, Some(instr));

                self.builder.build_return(Some(&global)).unwrap();
            }

            _ => todo!(),
        }
    }

    fn emit_function(
        &mut self,
        name: &str,
        params: &[Instruction<'ctx>],
        body: &'ctx Instruction<'ctx>,
        return_kind: &Option<DataTypes>,
        is_public: bool,
    ) {
        let kind: FunctionType = datatype_to_fn_type(self.context, return_kind, params, None);

        let function: FunctionValue<'_> = self.module.add_function(name, kind, None);

        if is_public {
            function.set_linkage(Linkage::External);
        } else {
            function.set_linkage(Linkage::Private);
        }

        let mut index: usize = 0;

        function.get_params().iter().for_each(|param| {
            if let Some(Instruction::Param(name, _)) = params.get(index) {
                param.set_name(name);
            }

            index += 1;
        });

        let entry: BasicBlock = self.context.append_basic_block(function, "");

        self.builder.position_at_end(entry);

        self.codegen(body);

        if return_kind.is_none() {
            self.builder.build_return(None).unwrap();
        }
    }

    fn emit_global_string_constant(&mut self, string: &str, name: &str) -> PointerValue {
        let ty: ArrayType<'_> = self.context.i8_type().array_type(string.len() as u32);
        let gv: GlobalValue<'_> = self
            .module
            .add_global(ty, Some(AddressSpace::default()), name);
        gv.set_linkage(Linkage::Private);
        gv.set_initializer(&self.context.const_string(string.as_ref(), false));
        gv.set_constant(true);

        let pointer: PointerValue<'_> = self
            .builder
            .build_pointer_cast(
                gv.as_pointer_value(),
                self.context.ptr_type(AddressSpace::default()),
                name,
            )
            .unwrap();

        pointer
    }

    #[inline(always)]
    fn begin_scope(&mut self) {
        self.locals.clear();
    }

    #[inline(always)]
    fn end_scope(&mut self) {
        self.locals.clear();
    }

    fn build_const_integer_return(&mut self, kind: IntType, value: u64, signed: bool) {
        self.builder
            .build_return(Some(&kind.const_int(value, signed)))
            .unwrap();
    }

    fn get_local(&self, name: &str) -> &Instruction {
        self.locals.get(name).unwrap()
    }

    fn advance(&mut self) -> &'ctx Instruction<'ctx> {
        let c: &Instruction = &self.instructions[self.current];
        self.current += 1;

        c
    }

    fn is_end(&self) -> bool {
        self.current >= self.instructions.len()
    }
}

#[derive(Debug, Clone)]
pub enum Scope {
    Global,
    Local,
}

#[derive(Debug, Clone)]
pub enum Instruction<'ctx> {
    Puts(Box<Instruction<'ctx>>),
    Println(Vec<Instruction<'ctx>>),
    String(String),
    Integer(DataTypes, f64),
    Block(Vec<Instruction<'ctx>>),
    EntryPoint {
        body: Box<Instruction<'ctx>>,
    },
    Value(BasicValueEnum<'ctx>),
    Param(String, DataTypes),
    Function {
        name: String,
        params: Vec<Instruction<'ctx>>,
        body: Box<Instruction<'ctx>>,
        return_kind: Option<DataTypes>,
        is_public: bool,
    },
    Return(Box<Instruction<'ctx>>),
    Var {
        name: String,
        kind: DataTypes,
        value: Option<Box<Instruction<'ctx>>>,
    },
    RefVar {
        name: String,
        scope: Scope,
    },
    Boolean(bool),
    Null,
    End,
}

#[derive(Default, Debug)]
pub enum Optimization {
    #[default]
    None,
    Low,
    Mid,
    Mcqueen,
}

#[derive(Default, Debug)]
pub enum Linking {
    #[default]
    Static,
    Dynamic,
}

#[derive(Debug)]
pub struct Options {
    pub name: String,
    pub target_triple: TargetTriple,
    pub optimization: Optimization,
    pub interpret: bool,
    pub emit_llvm: bool,
    pub build: bool,
    pub linking: Linking,
    pub path: PathBuf,
    pub is_main: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            name: String::from("main"),
            target_triple: TargetMachine::get_default_triple(),
            optimization: Optimization::default(),
            interpret: false,
            emit_llvm: false,
            build: false,
            linking: Linking::default(),
            path: PathBuf::new(),
            is_main: true,
        }
    }
}

pub struct FileBuilder<'a, 'ctx> {
    module: &'a Module<'ctx>,
    options: &'a Options,
}

impl<'a, 'ctx> FileBuilder<'a, 'ctx> {
    pub fn new(options: &'a Options, module: &'a Module<'ctx>) -> Self {
        Self { options, module }
    }

    pub fn build(self) {
        let opt_level: &str = match self.options.optimization {
            Optimization::None => "O0",
            Optimization::Low => "O1",
            Optimization::Mid => "O2",
            Optimization::Mcqueen => "O3",
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

        match Command::new("clang-17").spawn() {
            Ok(mut child) => {
                child.kill().unwrap();

                if self.options.build {
                    match self.opt(opt_level) {
                        Ok(()) => {
                            Command::new("clang-17")
                                .arg("-opaque-pointers")
                                .arg(linking)
                                .arg("-ffast-math")
                                .arg(format!("{}.bc", self.options.name))
                                .arg("-o")
                                .arg(self.options.name.as_str())
                                .output()
                                .unwrap();
                        }
                        Err(err) => {
                            Logging::new(err).error();
                            return;
                        }
                    }
                } else {
                    match self.opt(opt_level) {
                        Ok(()) => {
                            Command::new("clang-17")
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
                        Err(err) => {
                            Logging::new(err).error();
                            return;
                        }
                    }
                }

                remove_file(format!("{}.bc", self.options.name)).unwrap();
            }
            Err(_) => {
                Logging::new("Compilation failed. Clang version 17 is not installed.".to_string())
                    .error();
            }
        }
    }

    fn opt(&self, opt_level: &str) -> Result<(), String> {
        match Command::new("opt-17").spawn() {
            Ok(mut child) => {
                child.kill().unwrap();

                Command::new("opt-17")
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
                "Compilation failed. LLVM Optimizer version 17 is not installed.",
            )),
        }
    }
}
