use {
    super::{
        super::{
            frontend::{lexer::DataTypes, parser::Instruction},
            CompilerOptions, Linking, Logging, Optimization, ThrushFile,
        },
        llvm::{
            build_const_integer, build_int_array_type_from_size, datatype_to_fn_type,
            set_globals_options,
        },
    },
    inkwell::{
        basic_block::BasicBlock,
        builder::Builder,
        context::Context,
        module::{Linkage, Module},
        targets::{CodeModel, RelocMode, Target, TargetMachine},
        types::{ArrayType, FunctionType, IntType},
        values::{BasicMetadataValueEnum, FunctionValue, GlobalValue, PointerValue},
        AddressSpace, OptimizationLevel,
    },
    std::{fs::remove_file, mem, path::Path, process::Command},
};

pub struct CodeGen<'ctx, 'instr> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    stmts: &'instr [Instruction<'instr>],
    current: usize,
    main: Option<FunctionValue<'ctx>>,
    file: ThrushFile,
    options: CompilerOptions,
}

impl<'ctx, 'instr> CodeGen<'ctx, 'instr> {
    pub fn new(
        context: &'ctx Context,
        module: Module<'ctx>,
        builder: Builder<'ctx>,
        stmts: &'instr [Instruction<'ctx>],
        file: ThrushFile,
        options: CompilerOptions,
    ) -> Self {
        Self {
            context,
            module,
            builder,
            stmts,
            current: 0,
            main: None,
            file,
            options,
        }
    }

    pub fn compile(&mut self) {
        self.standard_functions();

        while !self.end() {
            let instr: &Instruction<'instr> = self.advance();
            self.codegen(instr);
        }

        self.module.set_triple(&self.options.target_triple);

        let target_machine: TargetMachine = Target::from_triple(&self.module.get_triple())
            .unwrap()
            .create_target_machine(
                &self.module.get_triple(),
                "generic",
                "",
                OptimizationLevel::Default,
                RelocMode::Default,
                CodeModel::Default,
            )
            .unwrap();

        self.module
            .set_data_layout(&target_machine.get_target_data().get_data_layout());

        self.module.strip_debug_info();

        if self.options.interpret {
            self.interpret();
            return;
        }

        self.finish();
    }

    fn codegen(&mut self, stmt: &Instruction) -> Instruction {
        match stmt {
            Instruction::Block(body) => {
                body.iter().for_each(|stmt| {
                    self.codegen(stmt);
                });

                Instruction::Null
            }

            Instruction::Var { name, kind, value } => match value {
                Some(value) => {
                    let value = self.codegen(value);
                    /* self.define_variable(name, kind, value); */
                    Instruction::Null
                }
                None => Instruction::Null,
            },

            Instruction::Function {
                name,
                params,
                body,
                return_kind,
                is_public,
            } => {
                self.define_function(name, params, body, return_kind, *is_public);

                Instruction::Null
            }

            Instruction::Return(instr) => {
                self.emit_return(instr);

                Instruction::Null
            }

            Instruction::Puts(instr) => {
                self.puts(instr);
                Instruction::Null
            }

            Instruction::Println(args) => {
                self.println(args);

                Instruction::Null
            }

            Instruction::String(string) => {
                Instruction::PointerValue(self.emit_global_string_constant(string, ""))
            }

            Instruction::EntryPoint { body } => {
                self.main = Some(self.emit_main());
                self.codegen(body);
                self.build_const_integer_return(self.context.i32_type(), 0, false);

                Instruction::Null
            }

            Instruction::End => Instruction::Null,

            _ => todo!(),
        }
    }

    fn standard_functions(&mut self) {
        self.define_puts();
        self.define_printf();
    }

    fn emit_return(&mut self, instr: &Instruction) {
        match instr {
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

    fn define_function(
        &mut self,
        name: &str,
        params: &[Instruction<'_>],
        body: &Instruction,
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

    fn println(&mut self, instrs: &[Instruction]) {
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

            _ => todo!(),
        });

        self.builder
            .build_call(self.module.get_function("printf").unwrap(), &args, "")
            .unwrap();
    }

    fn puts(&mut self, instr: &Instruction) {
        let pointer = match instr {
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

    fn emit_main(&mut self) -> FunctionValue<'ctx> {
        let main_kind: FunctionType = self.context.i32_type().fn_type(&[], false);
        let main: FunctionValue = self.module.add_function("main", main_kind, None);

        let entry_point: BasicBlock = self.context.append_basic_block(main, "");

        self.builder.position_at_end(entry_point);

        main
    }

    fn build_const_integer_return(&mut self, kind: IntType, value: u64, signed: bool) {
        self.builder
            .build_return(Some(&kind.const_int(value, signed)))
            .unwrap();
    }

    fn advance(&mut self) -> &'instr Instruction<'instr> {
        if !self.end() {
            self.current += 1;
        }

        self.previous()
    }

    fn previous(&self) -> &'instr Instruction<'instr> {
        &self.stmts[self.current - 1]
    }

    fn end(&self) -> bool {
        self.current >= self.stmts.len()
    }

    fn interpret(&self) {}

    fn finish(&mut self) {
        if self.options.emit_llvm {
            self.module
                .print_to_file(format!("{}.ll", self.file.name))
                .unwrap();

            return;
        }

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

        self.module
            .write_bitcode_to_path(Path::new(&format!("{}.bc", self.file.name)));

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
                                .arg(format!("{}.bc", self.file.name))
                                .arg("-o")
                                .arg(self.file.name.as_str())
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
                                .arg(format!("{}.bc", self.file.name))
                                .arg("-o")
                                .arg(format!("{}.o", self.file.name))
                                .output()
                                .unwrap();
                        }
                        Err(err) => {
                            Logging::new(err).error();
                            return;
                        }
                    }
                }

                remove_file(format!("{}.bc", self.file.name)).unwrap();
            }
            Err(_) => {
                Logging::new("Compilation failed. Clang 17 is not installed.".to_string()).error();
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
                    .arg(format!("{}.bc", self.file.name))
                    .output()
                    .unwrap();

                Ok(())
            }

            Err(_) => Err(String::from("Compilation failed. Opt 17 is not installed.")),
        }
    }
}
