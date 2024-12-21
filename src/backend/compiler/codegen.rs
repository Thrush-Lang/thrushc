use {
    super::{
        super::{
            super::{diagnostic::Diagnostic, frontend::lexer::DataTypes},
            apis::{debug::DebugAPI, vector::VectorAPI},
            instruction::Instruction,
        },
        general,
        locals::CompilerLocals,
        options::CompilerOptions,
        utils::{
            build_const_float, build_const_integer, build_int_array_type_from_size,
            datatype_float_to_type, datatype_int_to_type, datatype_to_fn_type,
        },
        variable,
    },
    inkwell::{
        basic_block::BasicBlock,
        builder::Builder,
        context::Context,
        module::{Linkage, Module},
        types::{ArrayType, FunctionType, StructType},
        values::{
            BasicMetadataValueEnum, BasicValueEnum, FunctionValue, GlobalValue, InstructionOpcode,
            InstructionValue, IntValue, PointerValue,
        },
        AddressSpace,
    },
};

pub struct Codegen<'a, 'ctx> {
    module: &'a Module<'ctx>,
    builder: &'a Builder<'ctx>,
    context: &'ctx Context,
    instructions: &'ctx [Instruction<'ctx>],
    current: usize,
    function: Option<FunctionValue<'ctx>>,
    locals: CompilerLocals<'ctx>,
    diagnostics: Diagnostic,
    options: &'a CompilerOptions,
}

impl<'a, 'ctx> Codegen<'a, 'ctx> {
    pub fn gen(
        module: &'a Module<'ctx>,
        builder: &'a Builder<'ctx>,
        context: &'ctx Context,
        options: &'a CompilerOptions,
        instructions: &'ctx [Instruction<'ctx>],
    ) {
        Self {
            module,
            builder,
            context,
            instructions,
            current: 0,
            function: None,
            locals: CompilerLocals::new(),
            diagnostics: Diagnostic::new(&options.file_path),
            options,
        }
        .start();
    }

    fn start(&mut self) {
        self.declare_basics();
        self.define_math_functions();

        if self.options.include_vector_api {
            VectorAPI::include(self.module, self.builder, self.context);
        } else {
            VectorAPI::define(self.module, self.builder, self.context);
        }

        if self.options.include_debug_api {
            DebugAPI::include(self.module, self.builder, self.context);
        } else {
            DebugAPI::define(self.module, self.builder, self.context);
        }

        while !self.is_end() {
            let instr: &Instruction<'_> = self.advance();
            self.codegen(instr);
        }
    }

    fn codegen(&mut self, instr: &'ctx Instruction<'ctx>) -> Instruction<'ctx> {
        match instr {
            Instruction::Block { stmts, .. } => {
                self.locals.push();

                stmts.iter().for_each(|instr| {
                    self.codegen(instr);
                });

                self.locals.pop();

                Instruction::Null
            }

            Instruction::Free { name, is_string } => {
                let variable: BasicValueEnum<'_> = self.locals.find_and_get(name).unwrap();

                if let BasicValueEnum::PointerValue(ptr) = variable {
                    if *is_string {
                        self.builder
                            .build_call(
                                self.module.get_function("Vec.destroy").unwrap(),
                                &[ptr.into()],
                                "",
                            )
                            .unwrap();
                    }

                    self.builder.build_free(ptr).unwrap();
                }

                Instruction::Null
            }

            Instruction::ForLoop {
                variable,
                cond,
                actions,
                block,
            } => {
                self.codegen(variable.as_ref().unwrap());

                let start_block: BasicBlock<'ctx> =
                    self.context.append_basic_block(self.function.unwrap(), "");

                self.builder
                    .build_unconditional_branch(start_block)
                    .unwrap();

                self.builder.position_at_end(start_block);

                let cond: IntValue<'ctx> = self
                    .codegen(cond.as_ref().unwrap())
                    .as_basic_value()
                    .into_int_value();

                let then_block: BasicBlock<'ctx> =
                    self.context.append_basic_block(self.function.unwrap(), "");

                let exit_block: BasicBlock<'ctx> =
                    self.context.append_basic_block(self.function.unwrap(), "");

                self.builder
                    .build_conditional_branch(cond, then_block, exit_block)
                    .unwrap();

                self.builder.position_at_end(then_block);

                self.codegen(actions.as_ref().unwrap());
                self.codegen(block.as_ref());

                self.builder
                    .build_unconditional_branch(start_block)
                    .unwrap();

                self.builder.position_at_end(exit_block);

                Instruction::Null
            }

            Instruction::Function {
                name,
                params,
                body,
                return_kind,
                is_public,
            } => {
                self.function =
                    Some(self.emit_function(name, params, body, return_kind, *is_public));

                Instruction::Null
            }

            Instruction::Return(instr) => {
                self.emit_return(instr);
                Instruction::Null
            }

            Instruction::String(string) => {
                self.emit_string_constant(string);
                Instruction::Null
            }

            Instruction::Println(data) | Instruction::Print(data) => {
                if self.module.get_function("printf").is_none() {
                    self.define_printf();
                }

                self.emit_print(data);

                Instruction::Null
            }

            Instruction::Var {
                name,
                kind,
                value,
                comptime,
                ..
            } => {
                if *comptime {
                    return Instruction::Null;
                }

                variable::compile(
                    self.module,
                    self.builder,
                    self.context,
                    &self.function.unwrap(),
                    name,
                    kind,
                    value,
                    &mut self.locals,
                    &mut self.diagnostics,
                );

                Instruction::Null
            }

            Instruction::Indexe {
                origin: origin_name,
                index,
                ..
            } => {
                let variable: BasicValueEnum<'_> = self.locals.find_and_get(origin_name).unwrap();

                let value: IntValue<'_> = self
                    .builder
                    .build_call(
                        self.module.get_function("Vec.get").unwrap(),
                        &[
                            variable.into_pointer_value().into(),
                            self.context.i64_type().const_int(*index, false).into(),
                        ],
                        "",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();

                let char: PointerValue<'_> = self.emit_char_from_indexe(value);

                Instruction::BasicValueEnum(char.into())
            }

            Instruction::MutVar { name, kind, value } => {
                variable::compile_mut(
                    self.module,
                    self.builder,
                    self.context,
                    &mut self.locals,
                    name,
                    kind,
                    value,
                );

                Instruction::Null
            }

            Instruction::Binary {
                op,
                left,
                right,
                kind,
                ..
            } => Instruction::BasicValueEnum(general::compile_binary_op(
                self.module,
                self.builder,
                self.context,
                left,
                op,
                right,
                kind,
                &self.locals,
            )),

            Instruction::Unary { op, value, kind } => {
                Instruction::BasicValueEnum(general::compile_unary_op(
                    self.module,
                    self.builder,
                    self.context,
                    op,
                    value,
                    kind,
                    &self.locals,
                    &self.function.unwrap(),
                    &mut self.diagnostics,
                ))
            }

            Instruction::EntryPoint { body } => {
                self.function = Some(self.emit_main());

                /* let alloc_char = self
                    .builder
                    .build_alloca(self.context.i8_type(), "")
                    .unwrap();

                self.builder
                    .build_store(alloc_char, self.context.i8_type().const_zero())
                    .unwrap();

                let malloc_vec: PointerValue<'ctx> = self
                    .builder
                    .build_malloc(
                        self.context.struct_type(
                            &[
                                self.context.i64_type().into(),                        // size
                                self.context.i64_type().into(),                        // capacity
                                self.context.i64_type().into(), // element_size
                                self.context.ptr_type(AddressSpace::default()).into(), // data
                                self.context.i8_type().into(),  // type
                            ],
                            false,
                        ),
                        "",
                    )
                    .unwrap();

                self.builder
                    .build_call(
                        self.module.get_function("Vec.init").unwrap(),
                        &[
                            malloc_vec.into(),
                            self.context.i64_type().const_int(10, false).into(),
                            self.context.i64_type().const_int(8, false).into(),
                            self.context.i8_type().const_int(1, false).into(),
                        ],
                        "",
                    )
                    .unwrap();

                let load_char: IntValue<'_> = self
                    .builder
                    .build_load(self.context.i8_type(), alloc_char, "")
                    .unwrap()
                    .into_int_value();

                self.builder
                    .build_call(
                        self.module.get_function("Vec.push_i8").unwrap(),
                        &[malloc_vec.into(), load_char.into()],
                        "",
                    )
                    .unwrap();

                self.builder
                    .build_call(
                        self.module.get_function("Vec.destroy").unwrap(),
                        &[malloc_vec.into()],
                        "",
                    )
                    .unwrap();

                self.builder.build_free(malloc_vec).unwrap(); */

                self.codegen(body);

                self.builder
                    .build_return(Some(&self.context.i32_type().const_int(0, false)))
                    .unwrap();

                Instruction::Null
            }

            e => {
                println!("{:?}", e);
                todo!()
            }
        }
    }

    fn declare_basics(&mut self) {
        let stderr: GlobalValue = self.module.add_global(
            self.context.ptr_type(AddressSpace::default()),
            Some(AddressSpace::default()),
            "stderr",
        );

        stderr.set_linkage(Linkage::External);

        let stdout: GlobalValue = self.module.add_global(
            self.context.ptr_type(AddressSpace::default()),
            Some(AddressSpace::default()),
            "stdout",
        );

        stdout.set_linkage(Linkage::External);
    }

    fn define_math_functions(&mut self) {
        let i8_type: StructType<'_> = self.context.struct_type(
            &[
                self.context.i8_type().into(),
                self.context.bool_type().into(),
            ],
            false,
        );

        let i16_type: StructType<'_> = self.context.struct_type(
            &[
                self.context.i16_type().into(),
                self.context.bool_type().into(),
            ],
            false,
        );

        let i32_type: StructType<'_> = self.context.struct_type(
            &[
                self.context.i32_type().into(),
                self.context.bool_type().into(),
            ],
            false,
        );

        let i64_type: StructType<'_> = self.context.struct_type(
            &[
                self.context.i64_type().into(),
                self.context.bool_type().into(),
            ],
            false,
        );

        for &size in &[
            ("8", i8_type),
            ("16", i16_type),
            ("32", i32_type),
            ("64", i64_type),
        ] {
            for &signed in &["u", "s"] {
                for &op in &["add", "sub", "mul", "div"] {
                    self.module.add_function(
                        &format!("llvm.{}{}.with.overflow.i{}", signed, op, size.0),
                        size.1.fn_type(
                            &[
                                size.1.get_field_type_at_index(0).unwrap().into(),
                                size.1.get_field_type_at_index(0).unwrap().into(),
                            ],
                            false,
                        ),
                        Some(Linkage::External),
                    );
                }
            }
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

    fn emit_main(&mut self) -> FunctionValue<'ctx> {
        let main_type: FunctionType = self.context.i32_type().fn_type(&[], false);
        let main: FunctionValue = self.module.add_function("main", main_type, None);

        let entry_point: BasicBlock = self.context.append_basic_block(main, "");

        self.builder.position_at_end(entry_point);

        main
    }

    fn emit_print(&mut self, instrs: &'ctx [Instruction]) {
        let mut args: Vec<BasicMetadataValueEnum> = Vec::with_capacity(instrs.len());

        instrs.iter().for_each(|instr| match instr {
            Instruction::String(string) => {
                args.push(string_into_basimetadatavaluenum(
                    self.module,
                    self.builder,
                    self.context,
                    string,
                ));
            }

            Instruction::Integer(kind, num) => {
                args.push(integer_into_basimetadatavaluenum(self.context, kind, *num));
            }

            Instruction::RefVar { name, kind, .. } => {
                args.push(reference_of_a_variable_into_basicametadatavaluenum(
                    self.builder,
                    self.context,
                    name,
                    kind,
                    &mut self.locals,
                ));
            }

            Instruction::Char(char) => {
                args.push(self.context.i8_type().const_int(*char as u64, false).into());
            }

            _ => todo!(),
        });

        self.builder
            .build_call(self.module.get_function("printf").unwrap(), &args, "")
            .unwrap();
    }

    /* fn emit_mut_variable(&mut self, name: &str, kind: &DataTypes, value: &Instruction) {
        let variable: BasicValueEnum<'_> = self.get_variable(name);

        match kind {
            DataTypes::I8
            | DataTypes::I16
            | DataTypes::I32
            | DataTypes::I64
            | DataTypes::U8
            | DataTypes::U16
            | DataTypes::U32
            | DataTypes::U64 => {
                let pointer: PointerValue<'ctx> = variable.into_pointer_value();

                if let Instruction::Integer(_, value) = value {
                    self.builder
                        .build_store(
                            pointer,
                            build_const_integer(self.context, kind, *value as u64),
                        )
                        .unwrap();
                } else {
                    todo!()
                }

                self.locals.insert(name.to_string(), variable);
            }

            DataTypes::F32 | DataTypes::F64 => {
                let pointer: PointerValue<'ctx> = variable.into_pointer_value();

                if let Instruction::Integer(_, value) = value {
                    self.builder
                        .build_store(pointer, build_const_float(self.context, kind, *value))
                        .unwrap();
                } else {
                    todo!()
                }

                self.locals.insert(name.to_string(), variable);
            }

            DataTypes::Bool => {
                if let Instruction::Boolean(value) = value {
                    self.emit_boolean(name, *value);
                }
            }

            DataTypes::Char => {
                if let Instruction::Char(value) = value {
                    self.emit_char(name, Some(*value));
                }
            }

            DataTypes::String => {
                if let Instruction::String(value) = value {
                    let string: PointerValue<'_> = self.emit_string(value).as_pointer_value();
                    self.locals.insert(name.to_string(), string.into());
                }
            }

            _ => todo!(),
        };
    } */

    /* match kind {
        DataTypes::F32 | DataTypes::F64 => {
            if let Instruction::Null = value {
                let store: InstructionValue<'_> = self
                    .builder
                    .build_store(ptr, build_const_float(self.context, kind, 0.0))
                    .unwrap();

                store.set_alignment(4).unwrap()
            } else if let Instruction::Integer(kind_value, num) = value {
                if ![DataTypes::F32, DataTypes::F64].contains(kind_value) {
                    todo!()
                }

                let store: InstructionValue<'_> = self
                    .builder
                    .build_store(ptr, build_const_float(self.context, kind, *num))
                    .unwrap();

                store.set_alignment(4).unwrap();
            } else if let Instruction::RefVar {
                name,
                kind: kind_refvar,
                ..
            } = value
            {
                let variable = self.get_variable(name);
                let load: BasicValueEnum<'ctx> = self
                    .builder
                    .build_load(
                        datatype_float_to_type(self.context, kind_refvar),
                        variable.into_pointer_value(),
                        "",
                    )
                    .unwrap();

                self.transform_an_float(kind, kind_refvar, ptr, load);
            } else {
                unreachable!()
            }

            self.locals.insert(name.to_string(), ptr.into());
        }

        DataTypes::String => match value {
            Instruction::Null => {
                self.locals.insert(
                    name.to_string(),
                    self.context
                        .ptr_type(AddressSpace::default())
                        .const_null()
                        .into(),
                );
            }

            Instruction::String(str) => {
                let string: GlobalValue<'_> = self.emit_string(str);

                self.locals.insert(
                    name.to_string(),
                    string.get_initializer().unwrap().into_vector_value().into(),
                );
            }

            Instruction::RefVar {
                name: refvar_name, ..
            } => {
                let variable: BasicValueEnum<'_> = self.get_variable(refvar_name);

                self.locals
                    .insert(name.to_string(), variable.into_vector_value().into());
            }

            Instruction::Binary {
                left, op, right, ..
            } => match (&(**left), op, &(**right)) {
                (
                    Instruction::String(left_str),
                    TokenKind::Plus,
                    Instruction::String(right_str),
                ) => {
                    /* let first_string_ptr: PointerValue<'_> = self.emit_string(left_str);
                    let second_string_ptr: PointerValue<'_> = self.emit_string(right_str);

                    let new_string: PointerValue<'_> = self
                        .builder
                        .build_call(
                            self.module.get_function("String.concat").unwrap(),
                            &[first_string_ptr.into(), second_string_ptr.into()],
                            "",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    self.locals[self.scope - 1].insert(name.to_string(), new_string.into()); */
                }

                _ => unreachable!(),
            },

            _ => unreachable!(),
        },

        DataTypes::Bool => match value {
            Instruction::Null => {
                self.locals.insert(
                    name.to_string(),
                    self.context
                        .ptr_type(AddressSpace::default())
                        .const_null()
                        .into(),
                );
            }

            Instruction::Boolean(bool) => {
                self.emit_boolean(name, *bool);
            }

            Instruction::RefVar {
                name: refvar_name, ..
            } => {
                let variable: BasicValueEnum<'_> = self.get_variable(refvar_name);

                let ptr: PointerValue<'_> = self.emit_boolean(name, false);

                let load: BasicValueEnum<'ctx> = self
                    .builder
                    .build_load(self.context.bool_type(), variable.into_pointer_value(), "")
                    .unwrap();

                let store: InstructionValue<'_> = self.builder.build_store(ptr, load).unwrap();

                store.set_alignment(4).unwrap();
            }

            _ => unimplemented!(),
        },

        DataTypes::Char => match value {
            Instruction::Null => {
                self.locals.insert(
                    name.to_string(),
                    self.context
                        .ptr_type(AddressSpace::default())
                        .const_null()
                        .into(),
                );
            }

            Instruction::Char(char) => {
                self.emit_char(name, Some(*char));
            }

            Instruction::RefVar {
                name: refvar_name, ..
            } => {
                let variable: BasicValueEnum<'_> = self.get_variable(refvar_name);

                let ptr: PointerValue<'_> = self.emit_char(name, None);

                let load: BasicValueEnum<'ctx> = self
                    .builder
                    .build_load(self.context.i8_type(), variable.into_pointer_value(), "")
                    .unwrap();

                let store: InstructionValue<'_> = self.builder.build_store(ptr, load).unwrap();

                store.set_alignment(4).unwrap();
            }

            Instruction::Indexe {
                origin,
                name: indexe_name,
                index,
                ..
            } => {
                let variable: BasicValueEnum<'_> = self.get_variable(origin);

                let string: VectorValue<'_> = variable.into_vector_value();

                let char: PointerValue<'ctx> = self
                    .builder
                    .build_alloca(self.context.i8_type(), "")
                    .unwrap();

                if *index > string.get_type().get_size() as u64 {
                    self.builder
                        .build_store(char, self.context.i8_type().const_zero())
                        .unwrap();

                    self.locals
                        .insert(indexe_name.unwrap().to_string(), char.into());

                    return;
                }

                self.builder
                    .build_store(
                        char,
                        string
                            .get_element_as_constant(*index as u32)
                            .into_int_value(),
                    )
                    .unwrap();

                self.locals
                    .insert(indexe_name.unwrap().to_string(), char.into());
            }

            _ => todo!(),
        },

        _ => todo!(),
    }; */

    fn emit_return(&mut self, instr: &Instruction) {
        match &instr {
            Instruction::Null => {}
            Instruction::Integer(kind, num) => {
                self.builder
                    .build_return(Some(&build_const_integer(self.context, kind, *num as u64)))
                    .unwrap();
            }

            Instruction::String(string) => {
                self.builder
                    .build_return(Some(&self.emit_string_constant(string)))
                    .unwrap();
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
    ) -> FunctionValue<'ctx> {
        let kind: FunctionType = datatype_to_fn_type(self.context, return_kind, params, None);

        let function: FunctionValue<'_> =
            self.module.add_function(name, kind, Some(Linkage::Common));

        if !is_public {
            function.set_linkage(Linkage::LinkerPrivate);
        }

        let mut index: usize = 0;

        function.get_params().iter().for_each(|param| {
            if let Some(Instruction::Param { name, .. }) = params.get(index) {
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

        function
    }

    fn emit_char_from_indexe(&mut self, value: IntValue<'ctx>) -> PointerValue<'ctx> {
        let char: PointerValue<'ctx> = self
            .builder
            .build_alloca(self.context.i8_type(), "")
            .unwrap();

        let store: InstructionValue<'ctx> = self.builder.build_store(char, value).unwrap();

        store.set_alignment(4).unwrap();

        char
    }

    /* fn emit_char(&mut self, name: &str, value: Option<u8>) -> PointerValue<'ctx> {
        let char: PointerValue<'ctx> = self
            .builder
            .build_alloca(self.context.i8_type(), "")
            .unwrap();

        if let Some(value) = value {
            let store: InstructionValue<'ctx> = self
                .builder
                .build_store(char, self.context.i8_type().const_int(value as u64, false))
                .unwrap();

            store.set_alignment(4).unwrap();
        }

        self.locals.insert(name.to_string(), char.into());

        char
    } */

    /* fn emit_boolean(&mut self, name: &str, value: bool) -> PointerValue<'ctx> {
        let boolean: PointerValue<'ctx> = self
            .builder
            .build_alloca(self.context.bool_type(), "")
            .unwrap();

        if value {
            self.builder
                .build_store(boolean, self.context.bool_type().const_int(1, false))
                .unwrap();
        } else {
            self.builder
                .build_store(boolean, self.context.bool_type().const_int(0, false))
                .unwrap();
        }

        self.locals.insert(name.to_string(), boolean.into());

        boolean
    } */

    fn emit_string_constant(&mut self, string: &str) -> PointerValue<'ctx> {
        let kind: ArrayType<'_> = self.context.i8_type().array_type(string.len() as u32);
        let global: GlobalValue<'_> =
            self.module
                .add_global(kind, Some(AddressSpace::default()), "");

        global.set_linkage(Linkage::LinkerPrivate);
        global.set_initializer(&self.context.const_string(string.as_ref(), false));
        global.set_constant(true);
        global.set_unnamed_addr(true);

        self.builder
            .build_pointer_cast(
                global.as_pointer_value(),
                self.context.ptr_type(AddressSpace::default()),
                "",
            )
            .unwrap()
    }

    /* fn emit_string(&mut self, value: &str) -> GlobalValue<'ctx> {
        let global: GlobalValue<'_> = self.module.add_global(
            self.context.i8_type().vec_type(value.len() as u32),
            Some(AddressSpace::default()),
            "",
        );

        let contain: Vec<IntValue<'ctx>> = value
            .as_bytes()
            .iter()
            .map(|byte| self.context.i8_type().const_int(*byte as u64, false))
            .collect();

        global.set_linkage(Linkage::LinkerPrivate);
        global.set_initializer(&VectorType::const_vector(&contain));

        global
    } */

    /* #[inline]
    fn get_variable(&self, name: &str) -> BasicValueEnum<'ctx> {
        for position in (0..self.locals.scope).rev() {
            if self.locals.contains_in_block(position, name) {
                return *self.locals.get_in_block(position, name).unwrap();
            }
        }

        panic!("\nUnexpected error at compiler internal at `fn get_variable()`\n Report this issue at https://github.com/Thrush-Lang/thrushc")
    } */

    #[inline]
    fn advance(&mut self) -> &'ctx Instruction<'ctx> {
        let c: &Instruction = &self.instructions[self.current];
        self.current += 1;

        c
    }

    #[inline]
    fn is_end(&self) -> bool {
        self.current >= self.instructions.len()
    }
}

fn string_into_basimetadatavaluenum<'ctx, 'a>(
    module: &'a Module<'ctx>,
    builder: &'a Builder<'ctx>,
    context: &'ctx Context,
    string: &str,
) -> BasicMetadataValueEnum<'ctx> {
    let kind: ArrayType<'_> =
        build_int_array_type_from_size(context, DataTypes::I8, string.len() as u32);

    let global: GlobalValue<'ctx> = module.add_global(kind, Some(AddressSpace::default()), "");

    global.set_initializer(&context.const_string(string.as_ref(), false));

    global.set_linkage(Linkage::LinkerPrivate);
    global.set_constant(true);
    global.set_unnamed_addr(true);
    global.set_alignment(4);

    builder
        .build_pointer_cast(
            global.as_pointer_value(),
            context.ptr_type(AddressSpace::default()),
            "",
        )
        .unwrap()
        .into()
}

fn integer_into_basimetadatavaluenum<'ctx>(
    context: &'ctx Context,
    kind: &'ctx DataTypes,
    num: f64,
) -> BasicMetadataValueEnum<'ctx> {
    match kind {
        DataTypes::F32 | DataTypes::F64 => build_const_float(context, kind, num).into(),
        _ => build_const_integer(context, kind, num.round() as u64).into(),
    }
}

fn reference_of_a_variable_into_basicametadatavaluenum<'ctx, 'a>(
    builder: &'a Builder<'ctx>,
    context: &'ctx Context,
    name: &'ctx str,
    kind: &'ctx DataTypes,
    locals: &mut CompilerLocals<'ctx>,
) -> BasicMetadataValueEnum<'ctx> {
    let variable: BasicValueEnum<'_> = locals.find_and_get(name).unwrap();

    match kind {
        DataTypes::U8
        | DataTypes::U16
        | DataTypes::U32
        | DataTypes::U64
        | DataTypes::I8
        | DataTypes::I16
        | DataTypes::I32
        | DataTypes::I64 => {
            let ptr: PointerValue<'ctx> = variable.into_pointer_value();

            builder
                .build_load(datatype_int_to_type(context, kind), ptr, "")
                .unwrap()
                .into()
        }

        DataTypes::F32 => {
            let ptr: PointerValue<'ctx> = variable.into_pointer_value();

            let load: BasicValueEnum<'ctx> = builder
                .build_load(datatype_float_to_type(context, kind), ptr, "")
                .unwrap();

            let bitcast: BasicValueEnum<'ctx> = builder
                .build_cast(
                    InstructionOpcode::FPExt,
                    load.into_float_value(),
                    context.f64_type(),
                    "",
                )
                .unwrap();

            bitcast.into()
        }

        DataTypes::F64 => {
            let ptr: PointerValue<'ctx> = variable.into_pointer_value();

            builder
                .build_load(datatype_float_to_type(context, kind), ptr, "")
                .unwrap()
                .into()
        }

        DataTypes::Char => {
            let ptr: PointerValue<'_> = variable.into_pointer_value();

            let char: IntValue<'_> = builder
                .build_load(context.i8_type(), ptr, "")
                .unwrap()
                .into_int_value();

            char.into()
        }

        DataTypes::String => {
            let ptr: PointerValue<'ctx> = variable.into_pointer_value();

            let get_data: PointerValue<'ctx> = builder
                .build_struct_gep(
                    context.struct_type(
                        &[
                            context.i64_type().into(),                        // size
                            context.i64_type().into(),                        // capacity
                            context.i64_type().into(),                        // element_size
                            context.ptr_type(AddressSpace::default()).into(), // data
                            context.i8_type().into(),                         // type
                        ],
                        false,
                    ),
                    ptr,
                    3,
                    "",
                )
                .unwrap();

            let load: BasicValueEnum<'ctx> = builder
                .build_load(get_data.get_type(), get_data, "")
                .unwrap();

            load.into()
        }

        DataTypes::Bool => {
            let ptr: PointerValue<'ctx> = variable.into_pointer_value();

            builder
                .build_load(context.bool_type(), ptr, "")
                .unwrap()
                .into()
        }

        _ => todo!(),
    }
}
