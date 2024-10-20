use {
    super::{
        super::frontend::lexer::DataTypes,
        infraestructure::StringInfraestructure,
        instruction::Instruction,
        llvm::{
            build_alloca_with_float, build_alloca_with_integer, build_const_float,
            build_const_integer, build_int_array_type_from_size, datatype_float_to_type,
            datatype_integer_to_type, datatype_to_fn_type, set_globals_options,
        },
    },
    ahash::AHashMap as HashMap,
    inkwell::{
        basic_block::BasicBlock,
        builder::Builder,
        context::Context,
        module::{Linkage, Module},
        targets::{CodeModel, RelocMode, TargetMachine, TargetTriple},
        types::{ArrayType, FunctionType, IntType},
        values::{
            BasicMetadataValueEnum, BasicValueEnum, FunctionValue, GlobalValue, InstructionOpcode,
            InstructionValue, IntValue, PointerValue,
        },
        AddressSpace,
    },
    std::path::PathBuf,
};

pub struct Compiler<'a, 'ctx> {
    module: &'a Module<'ctx>,
    builder: &'a Builder<'ctx>,
    context: &'ctx Context,
    instructions: &'ctx [Instruction<'ctx>],
    current: usize,
    locals: Vec<HashMap<String, BasicValueEnum<'ctx>>>,
    scope: usize,
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
            locals: Vec::new(),
            scope: 0,
        }
        .start();
    }

    fn start(&mut self) {
        StringInfraestructure::new(self.module, self.builder, self.context).define();

        while !self.is_end() {
            let instr: &Instruction<'_> = self.advance();
            self.codegen(instr);
        }
    }

    fn codegen(&mut self, instr: &'ctx Instruction<'ctx>) -> Instruction<'ctx> {
        match instr {
            Instruction::Block { stmts, .. } => {
                self.scope += 1;
                self.locals.push(HashMap::new());

                stmts.iter().for_each(|instr| {
                    self.codegen(instr);
                });

                self.scope -= 1;
                self.locals.pop();

                Instruction::Null
            }

            Instruction::Function {
                name,
                params,
                body,
                return_kind,
                is_public,
            } => {
                self.emit_function(name, params, body, return_kind, *is_public);
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
                name, kind, value, ..
            } => match value {
                Some(value) => {
                    self.emit_variable(name, kind, value);
                    Instruction::Null
                }
                None => {
                    self.emit_variable(name, kind, &Instruction::Null);
                    Instruction::Null
                }
            },

            Instruction::Indexe { origin, index, .. } => {
                if let Some(var) = self.get_variable(origin) {
                    let value: IntValue<'_> = self
                        .builder
                        .build_call(
                            self.module.get_function("String.extract").unwrap(),
                            &[
                                var.0.into_pointer_value().into(),
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

                    return Instruction::BasicValueEnum(char.into());
                }

                Instruction::Null
            }

            Instruction::MutVar { name, kind, value } => {
                self.emit_mut_variable(name, kind, value);
                Instruction::Null
            }

            Instruction::EntryPoint { body } => {
                self.emit_main();
                self.codegen(body);
                self.build_const_integer_return(self.context.i32_type(), 0, false);

                Instruction::Null
            }

            _ => todo!(),
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

    fn emit_main(&mut self) {
        let main_kind: FunctionType = self.context.i32_type().fn_type(&[], false);
        let main: FunctionValue = self.module.add_function("main", main_kind, None);

        let entry_point: BasicBlock = self.context.append_basic_block(main, "");

        self.builder.position_at_end(entry_point);
    }

    fn emit_print(&mut self, instrs: &'ctx [Instruction]) {
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

            Instruction::Integer(kind, num) => match kind {
                DataTypes::F32 | DataTypes::F64 => {
                    args.push(build_const_float(self.context, kind, *num).into())
                }
                _ => args.push(build_const_integer(self.context, kind, *num as u64).into()),
            },

            Instruction::RefVar { name, kind, .. } => {
                if let Some(var) = self.get_variable(name) {
                    let ptr: PointerValue<'ctx> = var.0.dissamble_to_pointer_value().unwrap();

                    match kind {
                        DataTypes::U8
                        | DataTypes::Char
                        | DataTypes::U16
                        | DataTypes::U32
                        | DataTypes::U64
                        | DataTypes::I8
                        | DataTypes::I16
                        | DataTypes::I32
                        | DataTypes::I64 => args.push(
                            self.builder
                                .build_load(datatype_integer_to_type(self.context, kind), ptr, "")
                                .unwrap()
                                .into(),
                        ),

                        DataTypes::F32 => {
                            let load: BasicValueEnum<'ctx> = self
                                .builder
                                .build_load(datatype_float_to_type(self.context, kind), ptr, "")
                                .unwrap();

                            let bitcast: BasicValueEnum<'ctx> = self
                                .builder
                                .build_cast(
                                    InstructionOpcode::FPExt,
                                    load.into_float_value(),
                                    self.context.f64_type(),
                                    "",
                                )
                                .unwrap();

                            args.push(bitcast.into())
                        }

                        DataTypes::F64 => args.push(
                            self.builder
                                .build_load(datatype_float_to_type(self.context, kind), ptr, "")
                                .unwrap()
                                .into(),
                        ),

                        DataTypes::String => args.push(
                            self.builder
                                .build_load(ptr.get_type(), ptr, "")
                                .unwrap()
                                .into(),
                        ),

                        DataTypes::Bool => args.push(
                            self.builder
                                .build_load(self.context.bool_type(), ptr, "")
                                .unwrap()
                                .into(),
                        ),

                        _ => todo!(),
                    }
                } else {
                    todo!()
                }
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

    fn emit_mut_variable(&mut self, name: &str, kind: &DataTypes, value: &Instruction) {
        let var: Option<(BasicValueEnum<'_>, usize)> = self.get_variable(name);

        if let Some(var) = var {
            match kind {
                DataTypes::I8
                | DataTypes::I16
                | DataTypes::I32
                | DataTypes::I64
                | DataTypes::U8
                | DataTypes::U16
                | DataTypes::U32
                | DataTypes::U64 => {
                    let pointer: PointerValue<'ctx> = var.0.dissamble_to_pointer_value().unwrap();

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

                    self.locals[self.scope - 1].insert(name.to_string(), var.0);
                }

                DataTypes::F32 | DataTypes::F64 => {
                    let pointer: PointerValue<'ctx> = var.0.dissamble_to_pointer_value().unwrap();

                    if let Instruction::Integer(_, value) = value {
                        self.builder
                            .build_store(pointer, build_const_float(self.context, kind, *value))
                            .unwrap();
                    } else {
                        todo!()
                    }

                    self.locals[self.scope - 1].insert(name.to_string(), var.0);
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
                        self.emit_string(name, value);
                    }
                }

                _ => todo!(),
            };
        }
    }

    fn emit_variable(&mut self, name: &str, kind: &DataTypes, value: &Instruction) {
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

            _ => self.context.ptr_type(AddressSpace::default()).const_null(),
        };

        match kind {
            DataTypes::I8
            | DataTypes::I16
            | DataTypes::I32
            | DataTypes::I64
            | DataTypes::U8
            | DataTypes::U16
            | DataTypes::U32
            | DataTypes::U64 => {
                if let Instruction::Null = value {
                    let store: InstructionValue<'_> = self
                        .builder
                        .build_store(ptr, build_const_integer(self.context, kind, 0))
                        .unwrap();

                    store.set_alignment(4).unwrap();
                } else if let Instruction::Integer(kind_value, num) = value {
                    if ![
                        DataTypes::I8,
                        DataTypes::I16,
                        DataTypes::I32,
                        DataTypes::I64,
                        DataTypes::U8,
                        DataTypes::U16,
                        DataTypes::U32,
                        DataTypes::U64,
                    ]
                    .contains(kind_value)
                    {
                        todo!()
                    } else if let DataTypes::I8
                    | DataTypes::I16
                    | DataTypes::I32
                    | DataTypes::I64
                    | DataTypes::U8
                    | DataTypes::U16
                    | DataTypes::U32
                    | DataTypes::U64 = kind_value
                    {
                        let store: InstructionValue<'_> = self
                            .builder
                            .build_store(ptr, build_const_integer(self.context, kind, *num as u64))
                            .unwrap();

                        store.set_alignment(4).unwrap();
                    }
                } else if let Instruction::RefVar {
                    name,
                    kind: kind_refvar,
                    ..
                } = value
                {
                    if let Some(var) = self.get_variable(name) {
                        let load: BasicValueEnum<'ctx> = self
                            .builder
                            .build_load(
                                datatype_integer_to_type(self.context, kind_refvar),
                                var.0.into_pointer_value(),
                                "",
                            )
                            .unwrap();

                        let store: InstructionValue<'_> = if kind != kind_refvar {
                            let cast: IntValue<'_> = self
                                .builder
                                .build_int_cast(
                                    load.into_int_value(),
                                    datatype_integer_to_type(self.context, kind),
                                    "",
                                )
                                .unwrap();

                            self.builder.build_store(ptr, cast).unwrap()
                        } else {
                            self.builder.build_store(ptr, load).unwrap()
                        };

                        store.set_alignment(4).unwrap();
                    } else {
                        unreachable!()
                    }
                } else {
                    todo!()
                }

                self.locals[self.scope - 1].insert(name.to_string(), ptr.into());
            }

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
                    if let Some(var) = self.get_variable(name) {
                        let load: BasicValueEnum<'ctx> = self
                            .builder
                            .build_load(
                                datatype_float_to_type(self.context, kind_refvar),
                                var.0.into_pointer_value(),
                                "",
                            )
                            .unwrap();

                        self.transform_an_float(kind, kind_refvar, ptr, load);
                    } else {
                        unreachable!()
                    }
                } else {
                    unreachable!()
                }

                self.locals[self.scope - 1].insert(name.to_string(), ptr.into());
            }

            DataTypes::String => match value {
                Instruction::Null => {
                    self.locals[self.scope - 1].insert(
                        name.to_string(),
                        self.context
                            .ptr_type(AddressSpace::default())
                            .const_null()
                            .into(),
                    );
                }

                Instruction::String(str) => {
                    self.emit_string(name, str);
                }

                Instruction::RefVar {
                    name: refvar_name, ..
                } => {
                    if let Some(var) = self.get_variable(refvar_name) {
                        let ptr: PointerValue<'_> = self.emit_string(name, "");

                        let load: BasicValueEnum<'ctx> = self
                            .builder
                            .build_load(
                                var.0.into_pointer_value().get_type(),
                                var.0.into_pointer_value(),
                                "",
                            )
                            .unwrap();

                        let store: InstructionValue<'_> =
                            self.builder.build_store(ptr, load).unwrap();

                        store.set_alignment(4).unwrap();
                    } else {
                        unreachable!()
                    }
                }

                _ => unreachable!(),
            },

            DataTypes::Bool => match value {
                Instruction::Null => {
                    self.locals[self.scope - 1].insert(
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
                    if let Some(var) = self.get_variable(refvar_name) {
                        let ptr: PointerValue<'_> = self.emit_boolean(name, false);

                        let load: BasicValueEnum<'ctx> = self
                            .builder
                            .build_load(self.context.bool_type(), var.0.into_pointer_value(), "")
                            .unwrap();

                        let store: InstructionValue<'_> =
                            self.builder.build_store(ptr, load).unwrap();

                        store.set_alignment(4).unwrap();
                    } else {
                        unreachable!()
                    }
                }

                _ => unimplemented!(),
            },

            DataTypes::Char => match value {
                Instruction::Null => {
                    self.locals[self.scope - 1].insert(
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
                    if let Some(var) = self.get_variable(refvar_name) {
                        let ptr: PointerValue<'_> = self.emit_char(name, None);

                        let load: BasicValueEnum<'ctx> = self
                            .builder
                            .build_load(self.context.i8_type(), var.0.into_pointer_value(), "")
                            .unwrap();

                        let store: InstructionValue<'_> =
                            self.builder.build_store(ptr, load).unwrap();

                        store.set_alignment(4).unwrap();
                    } else {
                        unreachable!()
                    }
                }

                Instruction::Indexe {
                    origin,
                    name: indexe_name,
                    index,
                    ..
                } => {
                    if let Some(var) = self.get_variable(origin) {
                        let value: IntValue<'_> = self
                            .builder
                            .build_call(
                                self.module.get_function("String.extract").unwrap(),
                                &[
                                    var.0.into_pointer_value().into(),
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

                        self.locals[self.scope - 1]
                            .insert(indexe_name.unwrap().to_string(), char.into());
                    } else {
                        unreachable!()
                    }
                }

                _ => todo!(),
            },

            _ => todo!(),
        };
    }

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
            function.set_param_alignment(index as u32, 4);

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

    fn emit_char(&mut self, name: &str, value: Option<u8>) -> PointerValue<'ctx> {
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

        self.locals[self.scope - 1].insert(name.to_string(), char.into());

        char
    }

    fn emit_boolean(&mut self, name: &str, value: bool) -> PointerValue<'ctx> {
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

        self.locals[self.scope - 1].insert(name.to_string(), boolean.into());

        boolean
    }

    fn emit_string_constant(&mut self, string: &str) -> PointerValue<'ctx> {
        let kind: ArrayType<'_> = self.context.i8_type().array_type(string.len() as u32);
        let global: GlobalValue<'_> =
            self.module
                .add_global(kind, Some(AddressSpace::default()), "");
        global.set_linkage(Linkage::Private);
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

    fn emit_string(&mut self, name: &str, value: &str) -> PointerValue<'ctx> {
        if self.module.get_function("malloc").is_none() {
            self.define_malloc();
        }

        let string: PointerValue<'ctx> = self
            .builder
            .build_alloca(self.context.ptr_type(AddressSpace::default()), "")
            .unwrap();

        let init_string: PointerValue<'ctx> = self
            .builder
            .build_call(
                self.module.get_function("malloc").unwrap(),
                &[self
                    .context
                    .i64_type()
                    .const_int(
                        (self.context.i8_type().get_bit_width() * value.len() as u32) as u64,
                        false,
                    )
                    .into()],
                "",
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        self.builder.build_store(string, init_string).unwrap();

        let mut index: u64 = 0;

        value.as_bytes().iter().for_each(|byte| {
            self.builder
                .build_call(
                    self.module.get_function("String.append").unwrap(),
                    &[
                        string.into(),
                        self.context.i8_type().const_int(*byte as u64, false).into(),
                        self.context.i64_type().const_int(index, false).into(),
                    ],
                    "",
                )
                .unwrap();

            index += 1;
        });

        self.locals[self.scope - 1].insert(name.to_string(), string.into());

        string
    }

    fn transform_an_float(
        &mut self,
        origin_kind: &DataTypes,
        kind: &DataTypes,
        ptr: PointerValue<'ctx>,
        load: BasicValueEnum<'ctx>,
    ) {
        let store: InstructionValue<'_> = if origin_kind != kind {
            let cast: BasicValueEnum<'ctx> = if kind == &DataTypes::F32 {
                self.builder
                    .build_cast(
                        InstructionOpcode::FPExt,
                        load.into_float_value(),
                        datatype_float_to_type(self.context, kind),
                        "",
                    )
                    .unwrap()
            } else {
                self.builder
                    .build_cast(
                        InstructionOpcode::FPTrunc,
                        load.into_float_value(),
                        datatype_float_to_type(self.context, kind),
                        "",
                    )
                    .unwrap()
            };

            self.builder.build_store(ptr, cast).unwrap()
        } else {
            self.builder.build_store(ptr, load).unwrap()
        };

        store.set_alignment(4).unwrap();
    }

    fn build_const_integer_return(&mut self, kind: IntType, value: u64, signed: bool) {
        self.builder
            .build_return(Some(&kind.const_int(value, signed)))
            .unwrap();
    }

    fn define_malloc(&mut self) {
        self.module.add_function(
            "malloc",
            self.context
                .ptr_type(AddressSpace::default())
                .fn_type(&[self.context.i64_type().into()], false),
            Some(Linkage::External),
        );
    }

    fn get_variable(&self, name: &str) -> Option<(BasicValueEnum<'ctx>, usize)> {
        for i in (0..self.scope).rev() {
            if self.locals[i].contains_key(name) {
                return Some((*self.locals[i].get(name).unwrap(), i));
            }
        }

        None
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

#[derive(Default, Debug)]
pub enum Opt {
    #[default]
    None,
    Low,
    Mid,
    Mcqueen,
}

trait Dissambler<'ctx> {
    fn dissamble_to_pointer_value(&self) -> Option<PointerValue<'ctx>>;
}

impl<'ctx> Dissambler<'ctx> for BasicValueEnum<'ctx> {
    fn dissamble_to_pointer_value(&self) -> Option<PointerValue<'ctx>> {
        if let BasicValueEnum::PointerValue(value) = &self {
            return Some(*value);
        }

        None
    }
}

#[derive(Default, Debug)]
pub enum Linking {
    #[default]
    Static,
    Dynamic,
}

#[derive(Debug)]
pub struct CompilerOptions {
    pub name: String,
    pub target_triple: TargetTriple,
    pub optimization: Opt,
    pub interpret: bool,
    pub emit_llvm: bool,
    pub emit_object: bool,
    pub build: bool,
    pub linking: Linking,
    pub path: PathBuf,
    pub is_main: bool,
    pub reloc_mode: RelocMode,
    pub code_model: CodeModel,
}

impl Default for CompilerOptions {
    fn default() -> Self {
        Self {
            name: String::from("main"),
            target_triple: TargetMachine::get_default_triple(),
            optimization: Opt::default(),
            interpret: false,
            emit_llvm: false,
            emit_object: false,
            build: false,
            linking: Linking::default(),
            path: PathBuf::new(),
            is_main: false,
            reloc_mode: RelocMode::Default,
            code_model: CodeModel::Default,
        }
    }
}
