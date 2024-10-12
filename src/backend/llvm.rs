use {
    super::{super::frontend::lexer::DataTypes, compiler::Instruction},
    inkwell::{
        builder::Builder,
        context::Context,
        module::Linkage,
        types::{ArrayType, BasicMetadataTypeEnum, FloatType, FunctionType, IntType},
        values::{FloatValue, GlobalValue, IntValue, PointerValue},
    },
};

pub fn datatype_integer_to_type<'ctx>(context: &'ctx Context, kind: &DataTypes) -> IntType<'ctx> {
    match kind {
        DataTypes::I8 | DataTypes::Char => context.i8_type(),
        DataTypes::I16 => context.i16_type(),
        DataTypes::I32 => context.i32_type(),
        DataTypes::I64 => context.i64_type(),

        DataTypes::U8 => context.i8_type(),
        DataTypes::U16 => context.i16_type(),
        DataTypes::U32 => context.i32_type(),
        DataTypes::U64 => context.i64_type(),

        _ => unreachable!(),
    }
}

pub fn datatype_float_to_type<'ctx>(context: &'ctx Context, kind: &DataTypes) -> FloatType<'ctx> {
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
) -> IntValue<'ctx> {
    match kind {
        DataTypes::U8 | DataTypes::Char => context.i8_type().const_int(num, false),
        DataTypes::U16 => context.i16_type().const_int(num, false),
        DataTypes::U32 => context.i32_type().const_int(num, false),
        DataTypes::U64 => context.i64_type().const_int(num, false),
        DataTypes::I8 => context.i8_type().const_int(num, true).const_neg(),
        DataTypes::I16 => context.i16_type().const_int(num, true).const_neg(),
        DataTypes::I32 => context.i32_type().const_int(num, true).const_neg(),
        DataTypes::I64 => context.i64_type().const_int(num, true).const_neg(),

        _ => unreachable!(),
    }
}

pub fn build_alloca_with_integer<'a, 'ctx>(
    builder: &'a Builder<'ctx>,
    kind: IntType<'ctx>,
) -> PointerValue<'ctx> {
    let alloca: PointerValue<'ctx> = builder.build_alloca(kind, "").unwrap();

    alloca.as_instruction().unwrap().set_alignment(4).unwrap();

    alloca
}

pub fn build_alloca_with_float<'a, 'ctx>(
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

pub fn set_globals_options<'ctx>(
    context: &'ctx Context,
    global: GlobalValue<'ctx>,
    initializer: Option<&Instruction>,
) {
    if let Some(Instruction::String(string)) = initializer {
        global.set_initializer(&context.const_string(string.as_ref(), false))
    }

    global.set_linkage(Linkage::Private);
    global.set_constant(true);
    global.set_unnamed_addr(true);
    global.set_alignment(1);
}

pub fn datatype_to_fn_type<'ctx>(
    context: &'ctx Context,
    kind: &Option<DataTypes>,
    params: &[Instruction<'_>],
    string: Option<String>,
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
            DataTypes::I8 => context.i8_type().fn_type(&param_types, true),
            DataTypes::I16 => context.i16_type().fn_type(&param_types, true),
            DataTypes::I32 => context.i32_type().fn_type(&param_types, true),
            DataTypes::I64 => context.i64_type().fn_type(&param_types, true),
            DataTypes::U8 => context.i8_type().fn_type(&param_types, true),
            DataTypes::U16 => context.i16_type().fn_type(&param_types, true),
            DataTypes::U32 => context.i32_type().fn_type(&param_types, true),
            DataTypes::U64 => context.i64_type().fn_type(&param_types, true),
            DataTypes::Void => context.void_type().fn_type(&param_types, true),
            DataTypes::String => context
                .i8_type()
                .vec_type(string.unwrap().len() as u32)
                .fn_type(&param_types, true),

            DataTypes::Bool => context.bool_type().fn_type(&param_types, true),
            DataTypes::F32 => context.f32_type().fn_type(&param_types, true),
            DataTypes::F64 => context.f64_type().fn_type(&param_types, true),

            _ => unimplemented!(),
        },

        None => context.void_type().fn_type(&param_types, true),
    }
}

pub fn datatype_to_basicmetadata_type_enum<'ctx>(
    context: &'ctx Context,
    kind: &DataTypes,
) -> BasicMetadataTypeEnum<'ctx> {
    match kind {
        DataTypes::I8 => BasicMetadataTypeEnum::IntType(context.i8_type()),
        DataTypes::I16 => BasicMetadataTypeEnum::IntType(context.i16_type()),
        DataTypes::I32 => BasicMetadataTypeEnum::IntType(context.i32_type()),
        DataTypes::I64 => BasicMetadataTypeEnum::IntType(context.i64_type()),
        DataTypes::U8 => BasicMetadataTypeEnum::IntType(context.i8_type()),
        DataTypes::U16 => BasicMetadataTypeEnum::IntType(context.i16_type()),
        DataTypes::U32 => BasicMetadataTypeEnum::IntType(context.i32_type()),
        DataTypes::U64 => BasicMetadataTypeEnum::IntType(context.i64_type()),
        DataTypes::Bool => BasicMetadataTypeEnum::IntType(context.bool_type()),
        DataTypes::F32 => BasicMetadataTypeEnum::FloatType(context.f32_type()),
        DataTypes::F64 => BasicMetadataTypeEnum::FloatType(context.f64_type()),

        _ => unreachable!(),
    }
}
