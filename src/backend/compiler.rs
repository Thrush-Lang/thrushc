use {
    super::{
        super::frontend::lexer::{DataTypes, TokenKind},
        infraestructure::{BasicInfraestructure, StringInfraestructure, VectorInfraestructure},
        instruction::Instruction,
        llvm::{
            build_alloca_with_float, build_alloca_with_integer, build_const_float,
            build_const_integer, build_int_array_type_from_size, datatype_float_to_type,
            datatype_integer_to_type, datatype_to_fn_type,
        },
    },
    ahash::AHashMap as HashMap,
    inkwell::{
        basic_block::BasicBlock,
        builder::Builder,
        context::Context,
        module::{Linkage, Module},
        targets::{CodeModel, RelocMode, TargetMachine, TargetTriple},
        types::{ArrayType, FunctionType, VectorType},
        values::{
            BasicMetadataValueEnum, BasicValueEnum, FunctionValue, GlobalValue, InstructionOpcode,
            InstructionValue, IntValue, PointerValue, VectorValue,
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
    scope: usize,
    locals: Vec<HashMap<String, BasicValueEnum<'ctx>>>,
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
            scope: 0,
            locals: Vec::new(),
        }
        .start();
    }

    fn start(&mut self) {
        BasicInfraestructure::new(self.module, self.context).define();
        VectorInfraestructure::new(self.module, self.builder, self.context).define();
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
                let variable: BasicValueEnum<'_> = self.get_variable(origin);

                let value: IntValue<'_> = self
                    .builder
                    .build_call(
                        self.module.get_function("String.extract").unwrap(),
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

                return Instruction::BasicValueEnum(char.into());
            }

            Instruction::MutVar { name, kind, value } => {
                self.emit_mut_variable(name, kind, value);
                Instruction::Null
            }

            Instruction::EntryPoint { body } => {
                self.emit_main();

                /* let alloc_char = self
                    .builder
                    .build_alloca(self.context.i8_type(), "")
                    .unwrap();

                self.builder
                    .build_store(alloc_char, self.context.i8_type().const_zero())
                    .unwrap();

                let alloc_vec: PointerValue<'ctx> = self
                    .builder
                    .build_alloca(
                        self.context.struct_type(
                            &[
                                self.context.i64_type().into(),                        // size
                                self.context.i64_type().into(),                        // capacity
                                self.context.i64_type().into(),                        // element_size
                                self.context.ptr_type(AddressSpace::default()).into(), // data
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
                            alloc_vec.into(),
                            self.context.i64_type().const_int(10, false).into(),
                            self.context.i64_type().const_int(4, false).into(),
                        ],
                        "",
                    )
                    .unwrap();

                self.builder
                    .build_call(
                        self.module.get_function("Vec.push_back").unwrap(),
                        &[alloc_vec.into(), alloc_char.into()],
                        "",
                    )
                    .unwrap();

                self.builder
                    .build_call(
                        self.module.get_function("Vec.destroy").unwrap(),
                        &[alloc_vec.into()],
                        "",
                    )
                    .unwrap(); */

                self.codegen(body);

                self.builder
                    .build_return(Some(&self.context.i32_type().const_int(0, false)))
                    .unwrap();

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
        let main_type: FunctionType = self.context.i32_type().fn_type(&[], false);
        let main: FunctionValue = self.module.add_function("main", main_type, None);

        let entry_point: BasicBlock = self.context.append_basic_block(main, "");

        self.builder.position_at_end(entry_point);
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
                    &self.locals,
                    self.scope,
                    name,
                    kind,
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

    fn emit_mut_variable(&mut self, name: &str, kind: &DataTypes, value: &Instruction) {
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

                self.locals[self.scope - 1].insert(name.to_string(), variable);
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

                self.locals[self.scope - 1].insert(name.to_string(), variable);
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

                    self.locals[self.scope - 1].insert(name.to_string(), string.into());
                }
            }

            _ => todo!(),
        };
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
                    let variable: BasicValueEnum<'_> = self.get_variable(name);

                    let load: BasicValueEnum<'ctx> = self
                        .builder
                        .build_load(
                            datatype_integer_to_type(self.context, kind_refvar),
                            variable.into_pointer_value(),
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
                    let string: GlobalValue<'_> = self.emit_string(str);

                    self.locals[self.scope - 1].insert(
                        name.to_string(),
                        string.get_initializer().unwrap().into_vector_value().into(),
                    );
                }

                Instruction::RefVar {
                    name: refvar_name, ..
                } => {
                    let variable: BasicValueEnum<'_> = self.get_variable(refvar_name);

                    self.locals[self.scope - 1]
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

                        self.locals[self.scope - 1]
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

                    self.locals[self.scope - 1]
                        .insert(indexe_name.unwrap().to_string(), char.into());
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

    fn emit_string(&mut self, value: &str) -> GlobalValue<'ctx> {
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

    #[inline]
    fn get_variable(&self, name: &str) -> BasicValueEnum<'ctx> {
        for index in (0..self.scope).rev() {
            if self.locals[index].contains_key(name) {
                return *self.locals[index].get(name).unwrap();
            }
        }

        panic!("\nUnexpected error\n Report this issue at https://github.com/Thrush-Lang/thrushc")
    }

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

#[inline]
fn get_variable<'ctx>(
    locals: &'ctx [HashMap<String, BasicValueEnum<'ctx>>],
    scope: usize,
    name: &str,
) -> BasicValueEnum<'ctx> {
    for index in (0..scope).rev() {
        if locals[index].contains_key(name) {
            return *locals[index].get(name).unwrap();
        }
    }

    panic!("\nUnexpected error\n Report this issue at https://github.com/Thrush-Lang/thrushc")
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

    global.set_initializer(&context.const_string(string.as_ref(), true));

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
    locals: &'ctx [HashMap<String, BasicValueEnum<'ctx>>],
    scope: usize,
    name: &'ctx str,
    kind: &'ctx DataTypes,
) -> BasicMetadataValueEnum<'ctx> {
    let variable: BasicValueEnum<'_> = get_variable(locals, scope, name);

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
                .build_load(datatype_integer_to_type(context, kind), ptr, "")
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
            let ptr_string_ref: VectorValue<'ctx> = variable.into_vector_value();

            let string_type: ArrayType<'_> = context
                .i8_type()
                .array_type(ptr_string_ref.get_type().get_size());

            let string: PointerValue<'ctx> = builder.build_alloca(string_type, "").unwrap();

            for i in 0..ptr_string_ref.get_type().get_size() {
                unsafe {
                    let gep_element_of_the_string: PointerValue<'ctx> = builder
                        .build_in_bounds_gep(
                            string_type,
                            string,
                            &[
                                context.i64_type().const_int(0, false),
                                context.i64_type().const_int(i as u64, false),
                            ],
                            "",
                        )
                        .unwrap();

                    builder
                        .build_store(
                            gep_element_of_the_string,
                            ptr_string_ref.get_element_as_constant(i).into_int_value(),
                        )
                        .unwrap();
                }
            }

            unsafe {
                let string_loaded: PointerValue<'ctx> = builder
                    .build_in_bounds_gep(
                        string_type,
                        string,
                        &[
                            context.i64_type().const_int(0, false),
                            context.i64_type().const_int(0, false),
                        ],
                        "",
                    )
                    .unwrap();

                string_loaded.into()
            }
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

#[derive(Default, Debug)]
pub enum Opt {
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
