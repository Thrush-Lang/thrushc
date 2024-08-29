use {
    super::super::{
        frontend::parser::Instruction, CompilerOptions, LarkFile, Logging, OptimizationLevel,
    },
    inkwell::{
        basic_block::BasicBlock,
        builder::Builder,
        context::Context,
        module::{Linkage, Module},
        targets::TargetTriple,
        types::{ArrayType, FunctionType, IntType},
        values::{BasicMetadataValueEnum, FunctionValue, GlobalValue, PointerValue},
        AddressSpace,
    },
    std::{fs::remove_file, process::Command},
};

pub struct CodeGen<'ctx, 'instr> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    stmts: &'instr [Instruction<'instr>],
    current: usize,
    main: Option<FunctionValue<'ctx>>,
    file: LarkFile,
    options: CompilerOptions,
}

impl<'ctx, 'instr> CodeGen<'ctx, 'instr> {
    pub fn new(
        context: &'ctx Context,
        module: Module<'ctx>,
        builder: Builder<'ctx>,
        stmts: &'instr [Instruction<'ctx>],
        file: LarkFile,
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

        self.module.set_triple(&self.options.target);

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

            Instruction::Puts(instr) => match (**instr).clone() {
                Instruction::String(string) => {
                    self.puts(&string);
                    Instruction::Null
                }

                _ => todo!(),
            },

            Instruction::String(string) => {
                Instruction::PointerValue(self.emit_global_string_constant(string.as_str(), ""))
            }

            Instruction::EntryPoint { body } => {
                self.main = Some(self.emit_main());
                self.codegen(body);
                self.build_integer_return(self.context.i32_type(), 0, false);
                self.advance();

                Instruction::Null
            }

            _ => todo!(),
        }
    }

    fn standard_functions(&mut self) {
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

    fn puts(&mut self, string: &str) {
        let ty: ArrayType<'_> = self.context.i8_type().array_type(string.len() as u32);
        let gv: GlobalValue<'_> = self
            .module
            .add_global(ty, Some(AddressSpace::default()), "");
        gv.set_linkage(Linkage::Private);
        gv.set_initializer(&self.context.const_string(string.as_ref(), false));
        gv.set_constant(true);

        let pointer: PointerValue<'_> = self
            .builder
            .build_pointer_cast(
                gv.as_pointer_value(),
                self.context.ptr_type(AddressSpace::default()),
                "",
            )
            .unwrap();

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

        let entry_point: BasicBlock = self.context.append_basic_block(main, "entrypoint");

        self.builder.position_at_end(entry_point);

        main
    }

    fn build_integer_return(&mut self, kind: IntType, value: u64, signed: bool) {
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

        self.module
            .print_to_file(format!("{}.ll", self.file.name))
            .unwrap();

        match Command::new("clang-17").spawn() {
            Ok(mut child) => {
                child.kill().unwrap();

                let opt: &str = match self.options.optimization {
                    OptimizationLevel::None => "-O0",
                    OptimizationLevel::Low => "-O1",
                    OptimizationLevel::Mid => "-O2",
                    OptimizationLevel::Mcqueen => "-O3",
                };

                if self.options.build {
                    Command::new("clang-17")
                        .arg("-opaque-pointers")
                        .arg(opt)
                        .arg(format!("{}.ll", self.file.name))
                        .arg("-o")
                        .arg(self.file.name.as_str())
                        .output()
                        .unwrap();
                } else {
                    Command::new("clang-17")
                        .arg("-opaque-pointers")
                        .arg(opt)
                        .arg("-c")
                        .arg(format!("{}.ll", self.file.name))
                        .arg("-o")
                        .arg(format!("{}.o", self.file.name))
                        .output()
                        .unwrap();
                }
                remove_file(format!("{}.ll", self.file.name)).unwrap();
            }
            Err(_) => {
                Logging::new("Compilation failed. Clang 17 is not installed.".to_string()).error();
            }
        }
    }
}
