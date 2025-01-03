use {
    super::{
        super::{
            super::frontend::lexer::DataTypes,
            apis::{debug::DebugAPI, vector::VectorAPI},
            instruction::Instruction,
        },
        functions, general,
        objects::CompilerObjects,
        options::CompilerOptions,
        utils, variable,
    },
    inkwell::{
        basic_block::BasicBlock,
        builder::Builder,
        context::Context,
        module::{Linkage, Module},
        types::{FunctionType, StructType},
        values::{
            BasicMetadataValueEnum, BasicValueEnum, FloatValue, FunctionValue, GlobalValue,
            InstructionOpcode, InstructionValue, IntValue, PointerValue,
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
    objects: CompilerObjects<'ctx>,
    options: &'a CompilerOptions,
    function: Option<FunctionValue<'ctx>>,
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
            objects: CompilerObjects::new(),
            options,
            function: None,
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

        self.predefine_functions();

        while !self.is_end() {
            let instr: &Instruction<'_> = self.advance();
            self.codegen(instr);
        }
    }

    fn codegen(&mut self, instr: &'ctx Instruction<'ctx>) -> Instruction<'ctx> {
        match instr {
            Instruction::Block { stmts, .. } => {
                self.objects.push();

                stmts.iter().for_each(|instr| {
                    self.codegen(instr);
                });

                self.objects.pop();

                Instruction::Null
            }

            Instruction::Free {
                name,
                is_string,
                free_only,
            } => {
                let var: PointerValue<'ctx> = self.objects.find_and_get(name).unwrap();

                if *is_string && !free_only {
                    self.builder
                        .build_call(
                            self.module.get_function("Vec.destroy").unwrap(),
                            &[var.into()],
                            "",
                        )
                        .unwrap();
                }

                self.builder.build_free(var).unwrap();

                Instruction::Null
            }

            Instruction::ForLoop {
                variable,
                cond,
                actions,
                block,
            } => {
                let function: FunctionValue<'ctx> = self.function.unwrap();

                self.codegen(variable.as_ref().unwrap());

                let start_block: BasicBlock<'ctx> = self.context.append_basic_block(function, "");

                self.builder
                    .build_unconditional_branch(start_block)
                    .unwrap();

                self.builder.position_at_end(start_block);

                let cond: IntValue<'ctx> = self
                    .codegen(cond.as_ref().unwrap())
                    .as_basic_value()
                    .into_int_value();

                let then_block: BasicBlock<'ctx> = self.context.append_basic_block(function, "");
                let exit_block: BasicBlock<'ctx> = self.context.append_basic_block(function, "");

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
                external_name,
                params,
                body,
                return_kind,
                is_public,
                is_external,
            } => {
                if let Some(body) = body {
                    self.compile_function(name, params, body, return_kind, *is_public, false);
                    return Instruction::Null;
                }

                if *is_external {
                    self.compile_external_function(name, params, return_kind, external_name);
                }

                Instruction::Null
            }

            Instruction::Return(instr, kind) => {
                self.emit_return(instr, kind);
                Instruction::Null
            }

            Instruction::String(_, _) => {
                compile_instr_as_basic_value_enum(
                    self.module,
                    self.builder,
                    self.context,
                    instr,
                    &[],
                    false,
                    &self.objects,
                );
                Instruction::Null
            }

            Instruction::Println(data) | Instruction::Print(data) => {
                if self.module.get_function("printf").is_none() {
                    self.define_printf();
                }

                self.compile_print(data);

                Instruction::Null
            }

            Instruction::Var {
                name,
                kind,
                value,
                only_comptime,
                ..
            } => {
                if *only_comptime {
                    return Instruction::Null;
                }

                variable::compile(
                    self.module,
                    self.builder,
                    self.context,
                    name,
                    kind,
                    value,
                    &mut self.objects,
                    self.function.unwrap(),
                );

                Instruction::Null
            }

            Instruction::MutVar { name, kind, value } => {
                variable::compile_mut(
                    self.module,
                    self.builder,
                    self.context,
                    &mut self.objects,
                    name,
                    kind,
                    value,
                    self.function.unwrap(),
                );

                Instruction::Null
            }

            Instruction::Indexe {
                origin: origin_name,
                index,
                ..
            } => {
                let variable: PointerValue<'ctx> = self.objects.find_and_get(origin_name).unwrap();

                let value: IntValue<'_> = self
                    .builder
                    .build_call(
                        self.module.get_function("Vec.get_i8").unwrap(),
                        &[
                            variable.into(),
                            self.context.i64_type().const_int(*index, false).into(),
                        ],
                        "",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .unwrap_left()
                    .into_int_value();

                let char: PointerValue<'_> = self.emit_char_from_indexe(value);

                Instruction::BasicValueEnum(char.into())
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
                &self.objects,
                self.function.unwrap(),
            )),

            Instruction::Unary { .. } => Instruction::BasicValueEnum(general::compile_unary_op(
                self.module,
                self.builder,
                self.context,
                instr,
                &self.objects,
                self.function.unwrap(),
            )),

            Instruction::EntryPoint { body } => {
                self.function = Some(self.build_main());

                self.codegen(body);

                self.builder
                    .build_return(Some(&self.context.i32_type().const_int(0, false)))
                    .unwrap();

                Instruction::Null
            }

            Instruction::Call { name, args, kind } => {
                functions::compile_call(
                    self.module,
                    self.builder,
                    self.context,
                    name,
                    args,
                    kind,
                    &self.objects,
                );

                Instruction::Null
            }

            e => {
                println!("{:?}", e);
                todo!()
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

    fn build_main(&mut self) -> FunctionValue<'ctx> {
        let main_type: FunctionType = self.context.i32_type().fn_type(&[], false);
        let main: FunctionValue = self.module.add_function("main", main_type, None);

        let entry_point: BasicBlock = self.context.append_basic_block(main, "");

        self.builder.position_at_end(entry_point);

        main
    }

    fn compile_print(&mut self, instrs: &'ctx [Instruction]) {
        let mut args: Vec<BasicMetadataValueEnum> = Vec::with_capacity(instrs.len());

        instrs.iter().for_each(|instr| match instr {
            Instruction::String(_, _)
            | Instruction::Integer(_, _, _)
            | Instruction::Float(_, _, _)
            | Instruction::Char(_) => {
                args.push(
                    compile_instr_as_basic_value_enum(
                        self.module,
                        self.builder,
                        self.context,
                        instr,
                        instrs,
                        false,
                        &self.objects,
                    )
                    .into(),
                );
            }
            Instruction::RefVar { name, kind, .. } => {
                args.push(reference_of_a_variable_into_basicametadatavaluenum(
                    self.builder,
                    self.context,
                    name,
                    kind,
                    &mut self.objects,
                ));
            }

            _ => todo!(),
        });

        self.builder
            .build_call(self.module.get_function("printf").unwrap(), &args, "")
            .unwrap();
    }

    fn emit_return(&mut self, instr: &'ctx Instruction, kind: &DataTypes) {
        if *kind == DataTypes::Void {
            self.builder.build_return(None).unwrap();

            return;
        }

        if let Instruction::Integer(_, num, is_signed) = instr {
            self.builder
                .build_return(Some(&utils::build_const_integer(
                    self.context,
                    kind,
                    *num as u64,
                    *is_signed,
                )))
                .unwrap();

            return;
        }

        if let Instruction::Indexe { origin, index, .. } = instr {
            let var: PointerValue<'ctx> = self.objects.find_and_get(origin).unwrap();

            let char: IntValue<'_> = self
                .builder
                .build_call(
                    self.module.get_function("Vec.get_i8").unwrap(),
                    &[
                        var.into(),
                        self.context.i64_type().const_int(*index, false).into(),
                    ],
                    "",
                )
                .unwrap()
                .try_as_basic_value()
                .unwrap_left()
                .into_int_value();

            self.builder.build_return(Some(&char)).unwrap();

            return;
        }

        if let Instruction::String(_, _) = instr {
            self.builder
                .build_return(Some(&compile_instr_as_basic_value_enum(
                    self.module,
                    self.builder,
                    self.context,
                    instr,
                    &[],
                    true,
                    &self.objects,
                )))
                .unwrap();

            return;
        }

        if let Instruction::Char(byte) = instr {
            self.builder
                .build_return(Some(&self.context.i8_type().const_int(*byte as u64, false)))
                .unwrap();

            return;
        }

        if let Instruction::Boolean(bool) = instr {
            self.builder
                .build_return(Some(
                    &self.context.bool_type().const_int(*bool as u64, false),
                ))
                .unwrap();

            return;
        }

        if let Instruction::RefVar { name, .. } = instr {
            if let DataTypes::String = kind {
                self.builder
                    .build_return(Some(&self.objects.find_and_get(name).unwrap()))
                    .unwrap();

                return;
            }

            if kind.is_integer() {
                let num: IntValue<'_> = self
                    .builder
                    .build_load(
                        utils::datatype_integer_to_llvm_type(self.context, kind),
                        self.objects.find_and_get(name).unwrap(),
                        "",
                    )
                    .unwrap()
                    .into_int_value();

                self.builder.build_return(Some(&num)).unwrap();

                return;
            }

            if kind.is_float() {
                let num: FloatValue<'_> = self
                    .builder
                    .build_load(
                        utils::datatype_float_to_llvm_type(self.context, kind),
                        self.objects.find_and_get(name).unwrap(),
                        "",
                    )
                    .unwrap()
                    .into_float_value();

                self.builder.build_return(Some(&num)).unwrap();

                return;
            }
        }

        todo!()
    }

    fn compile_external_function(
        &mut self,
        name: &'ctx str,
        params: &[Instruction<'ctx>],
        return_kind: &Option<DataTypes>,
        external_name: &str,
    ) {
        let kind: FunctionType<'_> = utils::datatype_to_fn_type(self.context, return_kind, params);
        let function: FunctionValue<'_> =
            self.module
                .add_function(external_name, kind, Some(Linkage::External));

        self.objects.insert_function(name, function);
    }

    fn compile_function(
        &mut self,
        name: &'ctx str,
        params: &[Instruction<'ctx>],
        body: &'ctx Instruction<'ctx>,
        return_kind: &Option<DataTypes>,
        is_public: bool,
        only_define: bool,
    ) {
        if only_define && self.module.get_function(name).is_none() {
            let kind: FunctionType = utils::datatype_to_fn_type(self.context, return_kind, params);

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

            self.function = Some(function);

            self.objects.insert_function(name, function);

            return;
        }

        let function: FunctionValue<'ctx> = self.module.get_function(name).unwrap();

        // self.function = Some(function);

        let entry: BasicBlock = self.context.append_basic_block(function, "");

        self.builder.position_at_end(entry);

        self.codegen(body);

        if return_kind.is_none() {
            self.builder.build_return(None).unwrap();
        }
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

    fn predefine_functions(&mut self) {
        self.instructions.iter().for_each(|instr| {
            if let Instruction::Function {
                name,
                params,
                body,
                return_kind,
                is_public,
                ..
            } = instr
            {
                if body.is_some() {
                    self.compile_function(
                        name,
                        params,
                        body.as_ref().unwrap(),
                        return_kind,
                        *is_public,
                        true,
                    );
                }
            }
        });
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

fn reference_of_a_variable_into_basicametadatavaluenum<'ctx, 'a>(
    builder: &'a Builder<'ctx>,
    context: &'ctx Context,
    name: &'ctx str,
    kind: &'ctx DataTypes,
    objects: &mut CompilerObjects<'ctx>,
) -> BasicMetadataValueEnum<'ctx> {
    let var: PointerValue<'ctx> = objects.find_and_get(name).unwrap();

    match kind {
        kind if kind.is_integer() => builder
            .build_load(utils::datatype_integer_to_llvm_type(context, kind), var, "")
            .unwrap()
            .into(),

        DataTypes::F32 => {
            let load: BasicValueEnum<'ctx> = builder
                .build_load(utils::datatype_float_to_llvm_type(context, kind), var, "")
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

        DataTypes::F64 => builder
            .build_load(utils::datatype_float_to_llvm_type(context, kind), var, "")
            .unwrap()
            .into(),

        DataTypes::Char => {
            let char: IntValue<'_> = builder
                .build_load(context.i8_type(), var, "")
                .unwrap()
                .into_int_value();

            char.into()
        }

        DataTypes::String => {
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
                    var,
                    3,
                    "",
                )
                .unwrap();

            let load: BasicValueEnum<'ctx> = builder
                .build_load(get_data.get_type(), get_data, "")
                .unwrap();

            load.into()
        }

        DataTypes::Bool => builder
            .build_load(context.bool_type(), var, "")
            .unwrap()
            .into(),

        _ => todo!(),
    }
}

pub fn compile_instr_as_basic_value_enum<'ctx>(
    module: &Module<'ctx>,
    builder: &Builder<'ctx>,
    context: &'ctx Context,
    instr: &'ctx Instruction,
    extra: &[Instruction],
    is_var: bool,
    objects: &CompilerObjects<'ctx>,
) -> BasicValueEnum<'ctx> {
    if let Instruction::String(str, is_fmt) = instr {
        if *is_fmt {
            return utils::build_formatter_string(module, builder, context, str, extra).into();
        }

        if !is_var {
            return utils::build_string_constant(module, builder, context, str).into();
        }

        return utils::build_dynamic_string(module, builder, context, str).into();
    }

    if let Instruction::Float(kind, num, _) = instr {
        return utils::build_const_float(context, kind, *num).into();
    }

    if let Instruction::Integer(kind, num, is_signed) = instr {
        return utils::build_const_integer(context, kind, *num as u64, *is_signed).into();
    }

    if let Instruction::Char(char) = instr {
        return context.i8_type().const_int(*char as u64, false).into();
    }

    if let Instruction::Boolean(bool) = instr {
        return context.bool_type().const_int(*bool as u64, false).into();
    }

    if let Instruction::RefVar { name, kind, .. } = instr {
        let var: PointerValue<'ctx> = objects.find_and_get(name).unwrap();

        if kind.is_float() {
            return builder
                .build_load(utils::datatype_float_to_llvm_type(context, kind), var, "")
                .unwrap();
        }

        if kind.is_integer() {
            return builder
                .build_load(utils::datatype_integer_to_llvm_type(context, kind), var, "")
                .unwrap();
        }

        return builder
            .build_call(module.get_function("Vec.clone").unwrap(), &[var.into()], "")
            .unwrap()
            .try_as_basic_value()
            .unwrap_left();
    }

    unreachable!()
}
