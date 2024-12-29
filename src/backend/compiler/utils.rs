use {
    super::super::{
        super::{
            diagnostic,
            frontend::lexer::{DataTypes, TokenKind},
        },
        instruction::Instruction,
    },
    inkwell::{
        basic_block::BasicBlock,
        builder::Builder,
        context::Context,
        module::{Linkage, Module},
        types::{ArrayType, BasicMetadataTypeEnum, FloatType, FunctionType, IntType},
        values::{
            BasicValueEnum, FloatValue, FunctionValue, GlobalValue, IntValue, PointerValue,
            StructValue,
        },
        AddressSpace,
    },
};

pub fn datatype_integer_to_llvm_type<'ctx>(
    context: &'ctx Context,
    kind: &DataTypes,
) -> IntType<'ctx> {
    match kind {
        DataTypes::I8 | DataTypes::Char => context.i8_type(),
        DataTypes::I16 => context.i16_type(),
        DataTypes::I32 => context.i32_type(),
        DataTypes::I64 => context.i64_type(),

        DataTypes::U8 => context.i8_type(),
        DataTypes::U16 => context.i16_type(),
        DataTypes::U32 => context.i32_type(),
        DataTypes::U64 => context.i64_type(),
        DataTypes::Bool => context.bool_type(),

        _ => unreachable!(),
    }
}

pub fn datatype_float_to_llvm_type<'ctx>(
    context: &'ctx Context,
    kind: &DataTypes,
) -> FloatType<'ctx> {
    match kind {
        DataTypes::F32 => context.f32_type(),
        DataTypes::F64 => context.f64_type(),

        _ => unreachable!(),
    }
}

pub fn build_const_float<'ctx>(
    context: &'ctx Context,
    kind: &'ctx DataTypes,
    num: f64,
) -> FloatValue<'ctx> {
    match kind {
        DataTypes::F32 => context.f32_type().const_float(num),
        DataTypes::F64 => context.f64_type().const_float(num),

        _ => unreachable!(),
    }
}

pub fn build_const_integer<'ctx>(
    context: &'ctx Context,
    kind: &'ctx DataTypes,
    num: u64,
    is_signed: bool,
) -> IntValue<'ctx> {
    match kind {
        DataTypes::U8 | DataTypes::Char if is_signed => {
            context.i8_type().const_int(num, is_signed).const_neg()
        }
        DataTypes::Bool => context.bool_type().const_int(num, false),
        DataTypes::U8 | DataTypes::Char => context.i8_type().const_int(num, is_signed),
        DataTypes::U16 if is_signed => context.i16_type().const_int(num, is_signed).const_neg(),
        DataTypes::U16 => context.i16_type().const_int(num, is_signed),
        DataTypes::U32 if is_signed => context.i32_type().const_int(num, is_signed).const_neg(),
        DataTypes::U32 => context.i32_type().const_int(num, is_signed),
        DataTypes::U64 if is_signed => context.i64_type().const_int(num, is_signed).const_neg(),
        DataTypes::U64 => context.i64_type().const_int(num, is_signed),
        DataTypes::I8 if is_signed => context.i8_type().const_int(num, is_signed).const_neg(),
        DataTypes::I8 => context.i8_type().const_int(num, is_signed),
        DataTypes::I16 if is_signed => context.i16_type().const_int(num, is_signed).const_neg(),
        DataTypes::I16 => context.i16_type().const_int(num, is_signed),
        DataTypes::I32 if is_signed => context.i32_type().const_int(num, is_signed).const_neg(),
        DataTypes::I32 => context.i32_type().const_int(num, is_signed),
        DataTypes::I64 if is_signed => context.i64_type().const_int(num, is_signed).const_neg(),
        DataTypes::I64 => context.i64_type().const_int(num, is_signed),

        _ => unreachable!(),
    }
}

pub fn build_alloca_int<'a, 'ctx>(
    builder: &'a Builder<'ctx>,
    kind: IntType<'ctx>,
) -> PointerValue<'ctx> {
    let alloca: PointerValue<'ctx> = builder.build_alloca(kind, "").unwrap();

    alloca.as_instruction().unwrap().set_alignment(4).unwrap();

    alloca
}

pub fn build_alloca_float<'a, 'ctx>(
    builder: &'a Builder<'ctx>,
    kind: FloatType<'ctx>,
) -> PointerValue<'ctx> {
    let alloca: PointerValue<'ctx> = builder.build_alloca(kind, "").unwrap();

    alloca.as_instruction().unwrap().set_alignment(4).unwrap();

    alloca
}

pub fn build_int_array_type_from_size(
    context: &'_ Context,
    kind: DataTypes,
    size: u32,
) -> ArrayType<'_> {
    match kind {
        DataTypes::I8 | DataTypes::U8 => context.i8_type().array_type(size),
        DataTypes::I16 | DataTypes::U16 => context.i16_type().array_type(size),
        DataTypes::I32 | DataTypes::U32 => context.i32_type().array_type(size),
        DataTypes::I64 | DataTypes::U64 => context.i64_type().array_type(size),

        _ => unreachable!(),
    }
}

pub fn datatype_to_fn_type<'ctx>(
    context: &'ctx Context,
    kind: &Option<DataTypes>,
    params: &[Instruction<'_>],
) -> FunctionType<'ctx> {
    let mut param_types: Vec<BasicMetadataTypeEnum<'ctx>> = Vec::with_capacity(params.len());

    params.iter().for_each(|param| match param {
        Instruction::Param { kind, .. } => {
            param_types.push(datatype_to_basicmetadata_type_enum(context, kind));
        }

        _ => unreachable!(),
    });

    match kind {
        Some(kind) => match kind {
            DataTypes::I8 | DataTypes::Char => context.i8_type().fn_type(&param_types, true),
            DataTypes::I16 => context.i16_type().fn_type(&param_types, true),
            DataTypes::I32 => context.i32_type().fn_type(&param_types, true),
            DataTypes::I64 => context.i64_type().fn_type(&param_types, true),
            DataTypes::U8 => context.i8_type().fn_type(&param_types, true),
            DataTypes::U16 => context.i16_type().fn_type(&param_types, true),
            DataTypes::U32 => context.i32_type().fn_type(&param_types, true),
            DataTypes::U64 => context.i64_type().fn_type(&param_types, true),
            DataTypes::Void => context.void_type().fn_type(&param_types, true),
            DataTypes::String => context
                .ptr_type(AddressSpace::default())
                .fn_type(&param_types, true),
            DataTypes::Bool => context.bool_type().fn_type(&param_types, true),
            DataTypes::F32 => context.f32_type().fn_type(&param_types, true),
            DataTypes::F64 => context.f64_type().fn_type(&param_types, true),
        },

        None => context.void_type().fn_type(&param_types, true),
    }
}

pub fn datatype_to_basicmetadata_type_enum<'ctx>(
    context: &'ctx Context,
    kind: &DataTypes,
) -> BasicMetadataTypeEnum<'ctx> {
    match kind {
        DataTypes::I8 => context.i8_type().into(),
        DataTypes::I16 => context.i16_type().into(),
        DataTypes::I32 => context.i32_type().into(),
        DataTypes::I64 => context.i64_type().into(),
        DataTypes::U8 => context.i8_type().into(),
        DataTypes::U16 => context.i16_type().into(),
        DataTypes::U32 => context.i32_type().into(),
        DataTypes::U64 => context.bool_type().into(),
        DataTypes::F32 => context.f32_type().into(),
        DataTypes::F64 => context.f64_type().into(),
        DataTypes::String => context.ptr_type(AddressSpace::default()).into(),

        _ => unreachable!(),
    }
}

#[inline]
pub fn float_autocast<'ctx>(
    kind: &DataTypes,
    target: &DataTypes,
    ptr: Option<PointerValue<'ctx>>,
    from: BasicValueEnum<'ctx>,
    builder: &Builder<'ctx>,
    context: &'ctx Context,
) -> Option<BasicValueEnum<'ctx>> {
    if kind == target {
        return None;
    }

    let cast: FloatValue<'ctx>;

    if kind != target && from.is_float_value() {
        cast = builder
            .build_float_cast(
                from.into_float_value(),
                datatype_float_to_llvm_type(context, target),
                "",
            )
            .unwrap();
    } else if kind != target && from.is_pointer_value() {
        let load: FloatValue<'ctx> = builder
            .build_load(
                datatype_float_to_llvm_type(context, kind),
                from.into_pointer_value(),
                "",
            )
            .unwrap()
            .into_float_value();

        cast = builder
            .build_float_cast(load, datatype_float_to_llvm_type(context, target), "")
            .unwrap();
    } else {
        builder.build_store(ptr.unwrap(), from).unwrap();
        return None;
    }

    if ptr.is_none() {
        return Some(cast.into());
    }

    builder.build_store(ptr.unwrap(), cast).unwrap();

    Some(cast.into())
}

#[inline]
pub fn integer_autocast<'ctx>(
    kind: &DataTypes,
    target: &DataTypes,
    ptr: Option<PointerValue<'ctx>>,
    from: BasicValueEnum<'ctx>,
    builder: &Builder<'ctx>,
    context: &'ctx Context,
) -> Option<BasicValueEnum<'ctx>> {
    if kind == target {
        return None;
    }

    let cast: IntValue<'ctx>;

    if kind != target && from.is_int_value() {
        cast = builder
            .build_int_cast_sign_flag(
                from.into_int_value(),
                datatype_integer_to_llvm_type(context, target),
                is_signed_integer(kind),
                "",
            )
            .unwrap()
    } else if kind != target && from.is_pointer_value() {
        let load: IntValue<'_> = builder
            .build_load(
                datatype_integer_to_llvm_type(context, kind),
                from.into_pointer_value(),
                "",
            )
            .unwrap()
            .into_int_value();

        cast = builder
            .build_int_cast_sign_flag(
                load,
                datatype_integer_to_llvm_type(context, target),
                is_signed_integer(kind),
                "",
            )
            .unwrap();
    } else {
        builder.build_store(ptr.unwrap(), from).unwrap();
        return None;
    }

    if ptr.is_none() {
        return Some(cast.into());
    }

    builder.build_store(ptr.unwrap(), cast).unwrap();

    Some(cast.into())
}

#[inline]
pub fn is_signed_integer(kind: &DataTypes) -> bool {
    matches!(
        kind,
        DataTypes::I8 | DataTypes::I16 | DataTypes::I32 | DataTypes::I64
    )
}

pub fn build_string_constant<'ctx>(
    module: &Module<'ctx>,
    builder: &Builder<'ctx>,
    context: &'ctx Context,
    string: &str,
) -> PointerValue<'ctx> {
    let kind: ArrayType<'_> = context.i8_type().array_type(string.len() as u32 + 1);
    let global: GlobalValue<'_> = module.add_global(kind, Some(AddressSpace::default()), "");

    global.set_linkage(Linkage::LinkerPrivate);
    global.set_initializer(&context.const_string(string.as_ref(), true));
    global.set_constant(true);
    global.set_unnamed_addr(true);

    builder
        .build_pointer_cast(
            global.as_pointer_value(),
            context.ptr_type(AddressSpace::default()),
            "",
        )
        .unwrap()
}

pub fn build_formatter_string<'ctx>(
    module: &Module<'ctx>,
    builder: &Builder<'ctx>,
    context: &'ctx Context,
    str: &str,
    args: &[Instruction],
) -> PointerValue<'ctx> {
    let mut string: String = str.to_string();

    let mut instr_pos: usize = 1;

    let parsed_fmts: Vec<String> = string
        .split_inclusive("{}")
        .filter(|substr| substr.contains("{}"))
        .map(|substr| {
            let fmt: String = substr.replace("{}", args[instr_pos].get_data_type().as_fmt());

            instr_pos += 1;

            fmt
        })
        .collect();

    let mut fmts: Vec<u8> = Vec::new();

    for arg in parsed_fmts {
        fmts.extend(arg.bytes());
    }

    string = String::from_utf8_lossy(&fmts).to_string();

    let kind: ArrayType<'_> =
        build_int_array_type_from_size(context, DataTypes::I8, string.len() as u32 + 1);
    let global: GlobalValue<'_> = module.add_global(kind, Some(AddressSpace::default()), "");

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
}

pub fn build_dynamic_string<'ctx>(
    module: &Module<'ctx>,
    builder: &Builder<'ctx>,
    context: &'ctx Context,
    from: &str,
) -> PointerValue<'ctx> {
    let string: PointerValue<'ctx> = builder
        .build_malloc(
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
            "",
        )
        .unwrap();

    builder
        .build_call(
            module.get_function("Vec.init").unwrap(),
            &[
                string.into(),
                context
                    .i64_type()
                    .const_int(from.len() as u64, false)
                    .into(),
                context
                    .i64_type()
                    .const_int(context.i8_type().get_bit_width() as u64, false)
                    .into(),
                context.i8_type().const_int(1, false).into(),
            ],
            "",
        )
        .unwrap();

    for i in from.as_bytes() {
        builder
            .build_call(
                module.get_function("Vec.push_i8").unwrap(),
                &[
                    string.into(),
                    context.i8_type().const_int(*i as u64, false).into(),
                ],
                "",
            )
            .unwrap();
    }

    string
}

pub fn build_possible_overflow<'ctx>(
    module: &Module<'ctx>,
    context: &'ctx Context,
    builder: &Builder<'ctx>,
    result: StructValue<'ctx>,
    instr_data: (DataTypes, &TokenKind, DataTypes, usize),
    current_function: FunctionValue<'ctx>,
) -> BasicValueEnum<'ctx> {
    let overflowed: IntValue<'_> = builder
        .build_extract_value(result, 1, "")
        .unwrap()
        .into_int_value();

    let true_block: BasicBlock<'_> = context.append_basic_block(current_function, "");
    let false_block: BasicBlock<'_> = context.append_basic_block(current_function, "");

    builder
        .build_conditional_branch(overflowed, true_block, false_block)
        .unwrap();

    builder.position_at_end(true_block);

    builder
        .build_call(
            module.get_function("panic").unwrap(),
            &[
                module
                    .get_global("stderr")
                    .unwrap()
                    .as_pointer_value()
                    .into(),
                build_string_constant(module, builder, context, "%s\0").into(),
                build_string_constant(
                    module,
                    builder,
                    context,
                    &format!(
                        "{}

Details:

    ● Line: {}
    ● Instruction: {} {} {}
    ● Operation: {}

{} \n\0",
                        diagnostic::create_panic_message("Integer / Float Overflow"),
                        instr_data.3,
                        instr_data.0,
                        instr_data.1,
                        instr_data.2,
                        instr_data.1,
                        diagnostic::create_help_message(
                            "Check that the limit of a primitive type has not been overflowed."
                        )
                    ),
                )
                .into(),
            ],
            "",
        )
        .unwrap();

    builder.build_unreachable().unwrap();

    builder.position_at_end(false_block);

    builder.build_extract_value(result, 0, "").unwrap()
}

pub fn build_overflow<'ctx>(
    module: &Module<'ctx>,
    builder: &Builder<'ctx>,
    kind: &DataTypes,
    op: &TokenKind,
    left_num: IntValue<'ctx>,
    right_num: IntValue<'ctx>,
) -> BasicValueEnum<'ctx> {
    match kind {
        DataTypes::I8 => builder
            .build_call(
                module
                    .get_function(&format!(
                        "llvm.s{}.with.overflow.i8",
                        op.to_llvm_intrinsic_identifier()
                    ))
                    .unwrap(),
                &[left_num.into(), right_num.into()],
                "",
            )
            .unwrap()
            .try_as_basic_value()
            .unwrap_left(),
        DataTypes::I16 => builder
            .build_call(
                module
                    .get_function(&format!(
                        "llvm.s{}.with.overflow.i16",
                        op.to_llvm_intrinsic_identifier()
                    ))
                    .unwrap(),
                &[left_num.into(), right_num.into()],
                "",
            )
            .unwrap()
            .try_as_basic_value()
            .unwrap_left(),
        DataTypes::I32 => builder
            .build_call(
                module
                    .get_function(&format!(
                        "llvm.s{}.with.overflow.i32",
                        op.to_llvm_intrinsic_identifier()
                    ))
                    .unwrap(),
                &[left_num.into(), right_num.into()],
                "",
            )
            .unwrap()
            .try_as_basic_value()
            .unwrap_left(),
        DataTypes::I64 => builder
            .build_call(
                module
                    .get_function(&format!(
                        "llvm.s{}.with.overflow.i64",
                        op.to_llvm_intrinsic_identifier()
                    ))
                    .unwrap(),
                &[left_num.into(), right_num.into()],
                "",
            )
            .unwrap()
            .try_as_basic_value()
            .unwrap_left(),
        DataTypes::U8 => builder
            .build_call(
                module
                    .get_function(&format!(
                        "llvm.s{}.with.overflow.i8",
                        op.to_llvm_intrinsic_identifier()
                    ))
                    .unwrap(),
                &[left_num.into(), right_num.into()],
                "",
            )
            .unwrap()
            .try_as_basic_value()
            .unwrap_left(),
        DataTypes::U16 => builder
            .build_call(
                module
                    .get_function(&format!(
                        "llvm.u{}.with.overflow.i16",
                        op.to_llvm_intrinsic_identifier()
                    ))
                    .unwrap(),
                &[left_num.into(), right_num.into()],
                "",
            )
            .unwrap()
            .try_as_basic_value()
            .unwrap_left(),
        DataTypes::U32 => builder
            .build_call(
                module
                    .get_function(&format!(
                        "llvm.u{}.with.overflow.i32",
                        op.to_llvm_intrinsic_identifier()
                    ))
                    .unwrap(),
                &[left_num.into(), right_num.into()],
                "",
            )
            .unwrap()
            .try_as_basic_value()
            .unwrap_left(),
        DataTypes::U64 => builder
            .build_call(
                module
                    .get_function(&format!(
                        "llvm.u{}.with.overflow.i64",
                        op.to_llvm_intrinsic_identifier()
                    ))
                    .unwrap(),
                &[left_num.into(), right_num.into()],
                "",
            )
            .unwrap()
            .try_as_basic_value()
            .unwrap_left(),
        _ => unreachable!(),
    }
}

pub fn build_ptr<'ctx>(
    context: &'ctx Context,
    builder: &Builder<'ctx>,
    kind: DataTypes,
) -> PointerValue<'ctx> {
    match kind {
        kind if kind.is_integer() => {
            build_alloca_int(builder, datatype_integer_to_llvm_type(context, &kind))
        }

        DataTypes::F64 | DataTypes::F32 => {
            build_alloca_float(builder, datatype_float_to_llvm_type(context, &kind))
        }

        _ => context.ptr_type(AddressSpace::default()).const_null(),
    }
}
