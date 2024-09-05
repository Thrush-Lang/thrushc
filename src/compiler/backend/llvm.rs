use {
    super::super::frontend::{lexer::DataTypes, parser::Instruction},
    inkwell::{
        builder::Builder,
        context::Context,
        module::Linkage,
        types::{ArrayType, BasicMetadataTypeEnum, FunctionType, IntType},
        values::{GlobalValue, InstructionValue, IntValue, PointerValue},
    },
};

pub fn build_const_integer<'ctx>(
    context: &'ctx Context,
    kind: &'ctx DataTypes,
    num: f64,
) -> IntValue<'ctx> {
    match kind {
        DataTypes::U8 => context.i8_type().const_int(num as u64, false),
        DataTypes::U16 => context.i16_type().const_int(num as u64, false),
        DataTypes::U32 => context.i32_type().const_int(num as u64, false),
        DataTypes::U64 => context.i64_type().const_int(num as u64, false),
        DataTypes::I8 => context.i8_type().const_int(num as u64, true).const_neg(),
        DataTypes::I16 => context.i16_type().const_int(num as u64, true).const_neg(),
        DataTypes::I32 => context.i32_type().const_int(num as u64, true).const_neg(),
        DataTypes::I64 => context.i64_type().const_int(num as u64, true).const_neg(),

        _ => unreachable!(),
    }
}

pub fn build_alloca<'ctx>(builder: &'ctx Builder, kind: IntType<'ctx>) -> PointerValue<'ctx> {
    builder.build_alloca(kind, "").unwrap()
}

pub fn build_store_with_integer<'ctx>(
    context: &'ctx Context,
    builder: &'ctx Builder,
    kind: &DataTypes,
    value: f64,
    ptr: PointerValue<'ctx>,
) -> InstructionValue<'ctx> {
    match kind {
        DataTypes::U8 => builder
            .build_store(ptr, context.i8_type().const_int(value as u64, false))
            .unwrap(),

        DataTypes::U16 => builder
            .build_store(ptr, context.i16_type().const_int(value as u64, false))
            .unwrap(),

        DataTypes::U32 => builder
            .build_store(ptr, context.i32_type().const_int(value as u64, false))
            .unwrap(),

        DataTypes::U64 => builder
            .build_store(ptr, context.i64_type().const_int(value as u64, false))
            .unwrap(),

        DataTypes::I8 => builder
            .build_store(
                ptr,
                context.i8_type().const_int(value as u64, true).const_neg(),
            )
            .unwrap(),

        DataTypes::I16 => builder
            .build_store(
                ptr,
                context.i16_type().const_int(value as u64, true).const_neg(),
            )
            .unwrap(),

        DataTypes::I32 => builder
            .build_store(
                ptr,
                context.i32_type().const_int(value as u64, true).const_neg(),
            )
            .unwrap(),

        DataTypes::I64 => builder
            .build_store(
                ptr,
                context.i64_type().const_int(value as u64, true).const_neg(),
            )
            .unwrap(),

        _ => todo!(),
    }
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
        Instruction::Param(_, kind) => {
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
