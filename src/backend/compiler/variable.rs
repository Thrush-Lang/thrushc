#![allow(clippy::too_many_arguments)]

use {
    super::{
        super::super::{frontend::lexer::DataTypes, NAME},
        general,
        locals::CompilerLocals,
        utils, Instruction,
    },
    inkwell::{
        basic_block::BasicBlock,
        builder::Builder,
        context::Context,
        module::Module,
        values::{
            BasicValueEnum, FunctionValue, InstructionValue, IntValue, PointerValue, StructValue,
        },
        AddressSpace, IntPredicate,
    },
    std::default,
};

pub fn compile<'ctx>(
    module: &Module<'ctx>,
    builder: &Builder<'ctx>,
    context: &'ctx Context,
    function: &FunctionValue<'ctx>,
    name: &str,
    kind: &DataTypes,
    value: &Instruction<'ctx>,
    locals: &mut CompilerLocals<'ctx>,
) {
    let default_ptr: PointerValue<'_> = match kind {
        DataTypes::I8 => {
            utils::build_alloca_int(builder, utils::datatype_int_to_type(context, kind))
        }

        DataTypes::I16 => {
            utils::build_alloca_int(builder, utils::datatype_int_to_type(context, kind))
        }

        DataTypes::I32 => {
            utils::build_alloca_int(builder, utils::datatype_int_to_type(context, kind))
        }

        DataTypes::I64 => {
            utils::build_alloca_int(builder, utils::datatype_int_to_type(context, kind))
        }

        DataTypes::U8 => {
            utils::build_alloca_int(builder, utils::datatype_int_to_type(context, kind))
        }

        DataTypes::U16 => {
            utils::build_alloca_int(builder, utils::datatype_int_to_type(context, kind))
        }

        DataTypes::U32 => {
            utils::build_alloca_int(builder, utils::datatype_int_to_type(context, kind))
        }

        DataTypes::U64 => {
            utils::build_alloca_int(builder, utils::datatype_int_to_type(context, kind))
        }

        DataTypes::F32 => {
            utils::build_alloca_with_float(builder, utils::datatype_float_to_type(context, kind))
        }

        DataTypes::F64 => {
            utils::build_alloca_with_float(builder, utils::datatype_float_to_type(context, kind))
        }

        _ => context.ptr_type(AddressSpace::default()).const_null(),
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
            compile_integer_var(
                module,
                function,
                builder,
                context,
                value,
                kind,
                name,
                default_ptr,
                locals,
            );
        }
        DataTypes::F32 | DataTypes::F64 => {
            compile_float_var(builder, context, value, kind, name, default_ptr, locals);
        }
        DataTypes::String => {
            compile_string_var(module, builder, context, name, value, default_ptr, locals)
        }
        DataTypes::Bool => compile_boolean_var(module, builder, context, name, value, locals),
        DataTypes::Char => {
            compile_char_var(module, builder, context, name, value, default_ptr, locals)
        }

        _ => todo!(),
    }
}

pub fn compile_mut<'ctx>(
    module: &Module<'ctx>,
    builder: &Builder<'ctx>,
    context: &'ctx Context,
    locals: &mut CompilerLocals<'ctx>,
    name: &str,
    kind: &DataTypes,
    value: &Instruction<'ctx>,
) {
    let variable: BasicValueEnum<'ctx> = locals.find_and_get(name).unwrap();

    match kind {
        DataTypes::I8
        | DataTypes::I16
        | DataTypes::I32
        | DataTypes::I64
        | DataTypes::U8
        | DataTypes::U16
        | DataTypes::U32
        | DataTypes::U64 => {
            if let Instruction::Integer(_, value) = value {
                builder
                    .build_store(
                        variable.into_pointer_value(),
                        utils::build_const_integer(context, kind, *value as u64),
                    )
                    .unwrap();

                locals.insert(name.to_string(), variable);
            }
        }

        DataTypes::F32 | DataTypes::F64 => {
            if let Instruction::Integer(_, value) = value {
                builder
                    .build_store(
                        variable.into_pointer_value(),
                        utils::build_const_float(context, kind, *value),
                    )
                    .unwrap();

                locals.insert(name.to_string(), variable);
            }
        }

        DataTypes::Bool => {
            if let Instruction::Boolean(bool) = value {
                let boolean_value: PointerValue<'ctx> = emit_boolean(builder, context, *bool);

                locals.insert(name.to_string(), boolean_value.into());
            }
        }

        DataTypes::Char => {
            if let Instruction::Char(value) = value {
                let char_value: PointerValue<'_> = emit_char(builder, context, Some(*value));

                locals.insert(name.to_string(), char_value.into());
            }
        }

        DataTypes::String => {
            if let Instruction::String(string) = value {
                builder
                    .build_call(
                        module.get_function("Vec.realloc").unwrap(),
                        &[
                            variable.into_pointer_value().into(),
                            context
                                .i64_type()
                                .const_int(string.len() as u64, false)
                                .into(),
                            context.bool_type().const_int(1, true).into(),
                        ],
                        "",
                    )
                    .unwrap();

                string.as_bytes().iter().for_each(|byte| {
                    builder
                        .build_call(
                            module.get_function("Vec.push_i8").unwrap(),
                            &[
                                variable.into_pointer_value().into(),
                                context.i8_type().const_int(*byte as u64, false).into(),
                            ],
                            "",
                        )
                        .unwrap();
                });
            }
        }

        _ => unreachable!(),
    }
}

fn compile_string_var<'ctx>(
    module: &Module<'ctx>,
    builder: &Builder<'ctx>,
    context: &'ctx Context,
    name: &str,
    value: &Instruction<'ctx>,
    default_ptr: PointerValue<'ctx>,
    locals: &mut CompilerLocals<'ctx>,
) {
    if let Instruction::Null = value {
        locals.insert(name.to_string(), default_ptr.into());
        return;
    }

    if let Instruction::String(string) = value {
        let string_default: PointerValue<'ctx> = builder
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
                    string_default.into(),
                    context
                        .i64_type()
                        .const_int(string.len() as u64, false)
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

        for i in string.as_bytes() {
            builder
                .build_call(
                    module.get_function("Vec.push_i8").unwrap(),
                    &[
                        string_default.into(),
                        context.i8_type().const_int(*i as u64, false).into(),
                    ],
                    "",
                )
                .unwrap();
        }

        locals.insert(name.to_string(), string_default.into());

        return;
    }

    if let Instruction::RefVar {
        name: refvar_name, ..
    } = value
    {
        let variable: BasicValueEnum<'ctx> = locals.find_and_get(refvar_name).unwrap();

        let new_string: PointerValue<'_> = builder
            .build_call(
                module.get_function("String.clone").unwrap(),
                &[variable.into_pointer_value().into()],
                "",
            )
            .unwrap()
            .try_as_basic_value()
            .unwrap_left()
            .into_pointer_value();

        locals.insert(name.to_string(), new_string.into());
    }
}

fn compile_integer_var<'ctx>(
    module: &Module<'ctx>,
    function: &FunctionValue<'ctx>,
    builder: &Builder<'ctx>,
    context: &'ctx Context,
    value: &Instruction<'ctx>,
    kind: &DataTypes,
    name: &str,
    default_ptr: PointerValue<'ctx>,
    locals: &mut CompilerLocals<'ctx>,
) {
    if let Instruction::Null = value {
        let store: InstructionValue<'_> = builder
            .build_store(default_ptr, utils::build_const_integer(context, kind, 0))
            .unwrap();

        store.set_alignment(4).unwrap();

        locals.insert(name.to_string(), default_ptr.into());

        return;
    }

    if let Instruction::Integer(kind, num) = value {
        if let DataTypes::I8
        | DataTypes::I16
        | DataTypes::I32
        | DataTypes::I64
        | DataTypes::U8
        | DataTypes::U16
        | DataTypes::U32
        | DataTypes::U64 = kind
        {
            let store: InstructionValue<'_> = builder
                .build_store(
                    default_ptr,
                    utils::build_const_integer(context, kind, *num as u64),
                )
                .unwrap();

            store.set_alignment(4).unwrap();

            locals.insert(name.to_string(), default_ptr.into());

            return;
        }
    }

    if let Instruction::RefVar {
        name,
        kind: kind_refvar,
        ..
    } = value
    {
        let variable: BasicValueEnum<'_> = locals.find_and_get(name).unwrap();

        let load: BasicValueEnum<'_> = builder
            .build_load(
                utils::datatype_int_to_type(context, kind_refvar),
                variable.into_pointer_value(),
                "",
            )
            .unwrap();

        let store: InstructionValue<'_> = if kind != kind_refvar {
            let intcast: IntValue<'_> = builder
                .build_int_cast(
                    load.into_int_value(),
                    utils::datatype_int_to_type(context, kind),
                    "",
                )
                .unwrap();

            builder.build_store(default_ptr, intcast).unwrap()
        } else {
            builder.build_store(default_ptr, load).unwrap()
        };

        store.set_alignment(4).unwrap();

        locals.insert(name.to_string(), default_ptr.into());
    }

    if let Instruction::Binary {
        left,
        op,
        right,
        line,
        ..
    } = value
    {
        let result: StructValue<'_> =
            general::compile_binary_op(module, builder, context, left, op, right, kind)
                .into_struct_value();

        let overflowed: IntValue<'_> = builder
            .build_extract_value(result, 1, "")
            .unwrap()
            .into_int_value();

        let true_block: BasicBlock<'_> = context.append_basic_block(*function, "");
        let false_block: BasicBlock<'_> = context.append_basic_block(*function, "");

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
                    utils::build_string_constant(module, builder, context, "%s\0").into(),
                    utils::build_string_constant(
                        module,
                        builder,
                        context,
                        &format!(
                            "Thrush Panic - (Integer or Float Overflow Type)

Details:

    ● File: {}
    ● Line: {:?}
    ● Instruction: {} {} {}
    ● Operation: {}

● Help: Check that the limit of a primitive type has not been overflowed. \n\0",
                            NAME.lock().unwrap(),
                            line,
                            left.get_data_type(),
                            op,
                            right.get_data_type(),
                            op
                        ),
                    )
                    .into(),
                ],
                "",
            )
            .unwrap();

        builder.build_unreachable().unwrap();

        builder.position_at_end(false_block);

        let result: IntValue<'_> = builder
            .build_extract_value(result, 0, "")
            .unwrap()
            .into_int_value();

        builder.build_store(default_ptr, result).unwrap();

        locals.insert(name.to_string(), default_ptr.into());
    }
}

fn compile_float_var<'ctx>(
    builder: &Builder<'ctx>,
    context: &'ctx Context,
    value: &Instruction<'ctx>,
    kind: &DataTypes,
    name: &str,
    default_ptr: PointerValue<'ctx>,
    locals: &mut CompilerLocals<'ctx>,
) {
    if let Instruction::Null = value {
        let store: InstructionValue<'_> = builder
            .build_store(default_ptr, utils::build_const_float(context, kind, 0.0))
            .unwrap();

        store.set_alignment(4).unwrap();

        locals.insert(name.to_string(), default_ptr.into());

        return;
    }

    if let Instruction::Float(kind_value, num) = value {
        /*
            (!) FIX IN THE PARSER

            if ![DataTypes::F32, DataTypes::F64].contains(kind_value) {
                todo!()
            }
        */

        let store: InstructionValue<'_> = builder
            .build_store(default_ptr, utils::build_const_float(context, kind, *num))
            .unwrap();

        store.set_alignment(4).unwrap();

        locals.insert(name.to_string(), default_ptr.into());

        return;
    }

    if let Instruction::RefVar {
        name,
        kind: kind_refvar,
        ..
    } = value
    {
        let variable: BasicValueEnum<'ctx> = locals.find_and_get(name).unwrap();

        let load: BasicValueEnum<'ctx> = builder
            .build_load(
                utils::datatype_float_to_type(context, kind_refvar),
                variable.into_pointer_value(),
                "",
            )
            .unwrap();

        utils::float_autocast(kind, kind_refvar, builder, context, default_ptr, load);

        locals.insert(name.to_string(), default_ptr.into());
    }
}

fn compile_boolean_var<'ctx>(
    module: &Module<'ctx>,
    builder: &Builder<'ctx>,
    context: &'ctx Context,
    name: &str,
    value: &Instruction,
    locals: &mut CompilerLocals<'ctx>,
) {
    match value {
        Instruction::Null => {
            locals.insert(name.to_string(), context.bool_type().const_zero().into());
        }
        Instruction::Boolean(value) => {
            let default_boolean: PointerValue<'ctx> =
                builder.build_alloca(context.bool_type(), "").unwrap();

            if *value {
                builder
                    .build_store(default_boolean, context.bool_type().const_int(1, false))
                    .unwrap();
            } else {
                builder
                    .build_store(default_boolean, context.bool_type().const_int(0, false))
                    .unwrap();
            }

            locals.insert(name.to_string(), default_boolean.into());
        }
        Instruction::RefVar {
            name: refvar_name, ..
        } => {
            let variable: BasicValueEnum<'_> = locals.find_and_get(refvar_name).unwrap();

            let default_ptr: PointerValue<'_> = emit_boolean(builder, context, false);

            let load: BasicValueEnum<'ctx> = builder
                .build_load(context.bool_type(), variable.into_pointer_value(), "")
                .unwrap();

            let store: InstructionValue<'_> = builder.build_store(default_ptr, load).unwrap();

            store.set_alignment(4).unwrap();

            locals.insert(name.to_string(), default_ptr.into());
        }

        Instruction::Binary {
            left,
            op,
            right,
            kind,
            ..
        } => {
            let result =
                general::compile_binary_op(module, builder, context, left, op, right, kind)
                    .into_int_value();

            let default_ptr: PointerValue<'ctx> =
                builder.build_alloca(context.bool_type(), "").unwrap();

            let store: InstructionValue<'_> = builder.build_store(default_ptr, result).unwrap();

            store.set_alignment(4).unwrap();

            locals.insert(name.to_string(), default_ptr.into());
        }

        _ => unreachable!(),
    }
}

#[inline]
fn emit_char<'ctx>(
    builder: &Builder<'ctx>,
    context: &'ctx Context,
    value: Option<u8>,
) -> PointerValue<'ctx> {
    let char: PointerValue<'ctx> = builder.build_alloca(context.i8_type(), "").unwrap();

    if let Some(value) = value {
        let store: InstructionValue<'ctx> = builder
            .build_store(char, context.i8_type().const_int(value as u64, false))
            .unwrap();

        store.set_alignment(4).unwrap();
    }

    char
}

#[inline]
fn emit_boolean<'ctx>(
    builder: &Builder<'ctx>,
    context: &'ctx Context,
    value: bool,
) -> PointerValue<'ctx> {
    let boolean: PointerValue<'ctx> = builder.build_alloca(context.bool_type(), "").unwrap();

    if value {
        builder
            .build_store(boolean, context.bool_type().const_int(1, false))
            .unwrap();
    } else {
        builder
            .build_store(boolean, context.bool_type().const_int(0, false))
            .unwrap();
    }

    boolean
}

#[inline]
fn compile_char_var<'ctx>(
    module: &Module<'ctx>,
    builder: &Builder<'ctx>,
    context: &'ctx Context,
    name: &str,
    value: &Instruction,
    default_ptr: PointerValue<'ctx>,
    locals: &mut CompilerLocals<'ctx>,
) {
    match value {
        Instruction::Null => {
            locals.insert(name.to_string(), default_ptr.into());
        }

        Instruction::Char(char) => {
            emit_char(builder, context, Some(*char));
        }

        Instruction::RefVar {
            name: refvar_name, ..
        } => {
            let variable: BasicValueEnum<'_> = locals.find_and_get(refvar_name).unwrap();
            let ptr: PointerValue<'_> = emit_char(builder, context, None);

            let load: BasicValueEnum<'ctx> = builder
                .build_load(context.i8_type(), variable.into_pointer_value(), "")
                .unwrap();

            let store: InstructionValue<'_> = builder.build_store(ptr, load).unwrap();

            store.set_alignment(4).unwrap();
        }

        Instruction::Indexe {
            origin: origin_name,
            index,
            ..
        } => {
            let variable: BasicValueEnum<'_> = locals.find_and_get(origin_name).unwrap();

            let char: IntValue<'_> = builder
                .build_call(
                    module.get_function("Vec.get").unwrap(),
                    &[
                        variable.into_pointer_value().into(),
                        context.i64_type().const_int(*index, false).into(),
                    ],
                    "",
                )
                .unwrap()
                .try_as_basic_value()
                .unwrap_left()
                .into_int_value();

            let new_default_char: PointerValue<'ctx> =
                builder.build_alloca(context.i8_type(), "").unwrap();

            builder.build_store(new_default_char, char).unwrap();
        }

        _ => todo!(),
    }
}
