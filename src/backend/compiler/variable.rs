#![allow(clippy::too_many_arguments)]

use {
    super::{
        super::super::frontend::lexer::DataTypes, codegen, functions, general,
        objects::CompilerObjects, utils, Instruction,
    },
    inkwell::{
        basic_block::BasicBlock,
        builder::Builder,
        context::Context,
        module::Module,
        values::{BasicValueEnum, FloatValue, FunctionValue, IntValue, PointerValue},
        AddressSpace, IntPredicate,
    },
};

pub fn compile<'ctx>(
    module: &Module<'ctx>,
    builder: &Builder<'ctx>,
    context: &'ctx Context,
    name: &str,
    kind: &'ctx DataTypes,
    value: &'ctx Instruction<'ctx>,
    objects: &mut CompilerObjects<'ctx>,
    function: FunctionValue<'ctx>,
) {
    let ptr: PointerValue<'_> = utils::build_ptr(context, builder, *kind);

    match kind {
        kind if kind.is_integer() => {
            compile_integer_var(
                module, builder, context, value, kind, name, objects, function, ptr,
            );
        }
        kind if kind.is_float() => {
            compile_float_var(
                module, builder, context, value, kind, name, objects, function, ptr,
            );
        }

        DataTypes::String => {
            compile_string_var(module, builder, context, name, value, objects, function);
        }

        _ => todo!(),
    }
}

pub fn compile_mut<'ctx>(
    module: &Module<'ctx>,
    builder: &Builder<'ctx>,
    context: &'ctx Context,
    objects: &mut CompilerObjects<'ctx>,
    name: &str,
    kind: &'ctx DataTypes,
    value: &'ctx Instruction<'ctx>,
    function: FunctionValue<'ctx>,
) {
    let var: PointerValue<'ctx> = objects.find_and_get(name).unwrap();

    if kind.is_integer() {
        compile_integer_var(
            module, builder, context, value, kind, name, objects, function, var,
        );
    }

    if kind.is_float() {
        compile_float_var(
            module, builder, context, value, kind, name, objects, function, var,
        );
    }

    if *kind == DataTypes::String {
        if let Instruction::String(str, _) = value {
            builder
                .build_call(
                    module.get_function("Vec.realloc").unwrap(),
                    &[
                        var.into(),
                        context.i64_type().const_int(str.len() as u64, false).into(),
                        context.bool_type().const_int(1, true).into(),
                    ],
                    "",
                )
                .unwrap();

            // HACERLO CON UN LOOP EN EL FUTURO, PARA EMITIR MENOS INSTRUCCIONES

            str.as_bytes().iter().for_each(|byte| {
                builder
                    .build_call(
                        module.get_function("Vec.push_i8").unwrap(),
                        &[
                            var.into(),
                            context.i8_type().const_int(*byte as u64, false).into(),
                        ],
                        "",
                    )
                    .unwrap();
            });
        }

        if let Instruction::RefVar {
            name: refvar_name, ..
        } = value
        {
            let string_from_mut: PointerValue<'_> = objects.find_and_get(refvar_name).unwrap();

            let new_size: IntValue<'_> = builder
                .build_call(
                    module.get_function("Vec.size").unwrap(),
                    &[string_from_mut.into()],
                    "",
                )
                .unwrap()
                .try_as_basic_value()
                .unwrap_left()
                .into_int_value();

            let alloca_idx: PointerValue<'ctx> =
                builder.build_alloca(context.i64_type(), "").unwrap();

            builder
                .build_store(alloca_idx, context.i64_type().const_zero())
                .unwrap();

            builder
                .build_call(
                    module.get_function("Vec.realloc").unwrap(),
                    &[
                        var.into(),
                        new_size.into(),
                        context.bool_type().const_zero().into(),
                    ],
                    "",
                )
                .unwrap();

            let start_block: BasicBlock<'_> = context.append_basic_block(function, "");

            builder.build_unconditional_branch(start_block).unwrap();

            builder.position_at_end(start_block);

            let get_idx: IntValue<'_> = builder
                .build_load(context.i64_type(), alloca_idx, "")
                .unwrap()
                .into_int_value();

            let cmp: IntValue<'_> = builder
                .build_int_compare(IntPredicate::UGT, get_idx, new_size, "")
                .unwrap();

            let then_block: BasicBlock<'_> = context.append_basic_block(function, "");
            let else_block: BasicBlock<'_> = context.append_basic_block(function, "");

            builder
                .build_conditional_branch(cmp, then_block, else_block)
                .unwrap();

            builder.position_at_end(else_block);

            let get_idx: IntValue<'_> = builder
                .build_load(context.i64_type(), alloca_idx, "")
                .unwrap()
                .into_int_value();

            let char: IntValue<'_> = builder
                .build_call(
                    module.get_function("Vec.get_i8").unwrap(),
                    &[string_from_mut.into(), get_idx.into()],
                    "",
                )
                .unwrap()
                .try_as_basic_value()
                .unwrap_left()
                .into_int_value();

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

            let data: PointerValue<'_> = builder
                .build_load(get_data.get_type(), get_data, "")
                .unwrap()
                .into_pointer_value();

            let get_space: PointerValue<'ctx> = unsafe {
                builder
                    .build_in_bounds_gep(context.i8_type(), data, &[get_idx], "")
                    .unwrap()
            };

            builder.build_store(get_space, char).unwrap();

            let get_idx: IntValue<'_> = builder
                .build_load(context.i64_type(), alloca_idx, "")
                .unwrap()
                .into_int_value();

            let new_idx: IntValue<'_> = builder
                .build_int_add(get_idx, context.i64_type().const_int(1, false), "")
                .unwrap();

            builder.build_store(alloca_idx, new_idx).unwrap();

            builder.build_unconditional_branch(start_block).unwrap();

            builder.position_at_end(then_block);
        }
    }
}

fn compile_string_var<'ctx>(
    module: &Module<'ctx>,
    builder: &Builder<'ctx>,
    context: &'ctx Context,
    name: &str,
    value: &'ctx Instruction<'ctx>,
    objects: &mut CompilerObjects<'ctx>,
    function: FunctionValue<'ctx>,
) {
    let ptr: PointerValue<'_> = utils::build_ptr(context, builder, DataTypes::String);

    if let Instruction::Null = value {
        objects.insert(name.to_string(), ptr);
    }

    if let Instruction::String(_, _) = value {
        objects.insert(
            name.to_string(),
            codegen::compile_instr_as_basic_value_enum(
                module,
                builder,
                context,
                value,
                &[],
                true,
                objects,
            )
            .into_pointer_value(),
        );
    }

    if let Instruction::RefVar { .. } = value {
        objects.insert(
            name.to_string(),
            codegen::compile_instr_as_basic_value_enum(
                module,
                builder,
                context,
                value,
                &[],
                true,
                objects,
            )
            .into_pointer_value(),
        );
    }

    if let Instruction::Call {
        name: call_name,
        args,
        kind: kind_call,
    } = value
    {
        objects.insert(
            name.to_string(),
            functions::compile_call(
                module, builder, context, call_name, args, kind_call, objects,
            )
            .unwrap()
            .into_pointer_value(),
        );
    }

    if let Instruction::Binary {
        left,
        op,
        right,
        kind,
        ..
    } = value
    {
        objects.insert(
            name.to_string(),
            general::compile_binary_op(
                module, builder, context, left, op, right, kind, objects, function,
            )
            .into_pointer_value(),
        );
    }
}

fn compile_integer_var<'ctx>(
    module: &Module<'ctx>,
    builder: &Builder<'ctx>,
    context: &'ctx Context,
    value: &'ctx Instruction<'ctx>,
    kind: &'ctx DataTypes,
    name: &str,
    objects: &mut CompilerObjects<'ctx>,
    function: FunctionValue<'ctx>,
    ptr: PointerValue<'ctx>,
) {
    if let Instruction::Null = value {
        builder
            .build_store(ptr, utils::build_const_integer(context, kind, 0, false))
            .unwrap();

        objects.insert(name.to_string(), ptr);
    }

    if let Instruction::Boolean(bool) = value {
        builder
            .build_store(
                ptr,
                utils::build_const_integer(context, kind, *bool as u64, false),
            )
            .unwrap();

        objects.insert(name.to_string(), ptr);
    }

    if let Instruction::Char(byte) = value {
        builder
            .build_store(
                ptr,
                utils::build_const_integer(context, kind, *byte as u64, false),
            )
            .unwrap();

        objects.insert(name.to_string(), ptr);
    }

    if let Instruction::Indexe {
        origin: from,
        index,
        ..
    } = value
    {
        let var: PointerValue<'_> = objects.find_and_get(from).unwrap();

        let char: IntValue<'_> = builder
            .build_call(
                module.get_function("Vec.get_i8").unwrap(),
                &[
                    var.into(),
                    context.i64_type().const_int(*index, false).into(),
                ],
                "",
            )
            .unwrap()
            .try_as_basic_value()
            .unwrap_left()
            .into_int_value();

        builder.build_store(ptr, char).unwrap();

        objects.insert(name.to_string(), ptr);
    }

    if let Instruction::Integer(_, num, is_signed) = value {
        builder
            .build_store(
                ptr,
                utils::build_const_integer(context, kind, *num as u64, *is_signed),
            )
            .unwrap();

        objects.insert(name.to_string(), ptr);
    }

    if let Instruction::RefVar {
        name: refvar_name,
        kind: kind_refvar,
        ..
    } = value
    {
        let var: PointerValue<'ctx> = objects.find_and_get(refvar_name).unwrap();

        let load: BasicValueEnum<'_> = builder
            .build_load(
                utils::datatype_integer_to_llvm_type(context, kind_refvar),
                var,
                "",
            )
            .unwrap();

        if utils::integer_autocast(kind_refvar, kind, Some(ptr), load, builder, context).is_none() {
            builder.build_store(ptr, load).unwrap();
        }

        objects.insert(name.to_string(), ptr);
    }

    if let Instruction::Binary {
        left, op, right, ..
    } = value
    {
        let mut result: BasicValueEnum<'_> = general::compile_binary_op(
            module, builder, context, left, op, right, kind, objects, function,
        );

        if result.is_struct_value() {
            result = utils::build_possible_overflow(
                module,
                context,
                builder,
                result.into_struct_value(),
                value.get_binary_data_types(),
                function,
            )
        }

        builder.build_store(ptr, result.into_int_value()).unwrap();

        objects.insert(name.to_string(), ptr);
    }

    if let Instruction::Call {
        name: call_name,
        args,
        kind: kind_call,
    } = value
    {
        let value: BasicValueEnum<'_> =
            functions::compile_call(module, builder, context, call_name, args, kind, objects)
                .unwrap();

        if utils::integer_autocast(kind_call, kind, Some(ptr), value, builder, context).is_none() {
            builder.build_store(ptr, value).unwrap();
        };

        objects.insert(name.to_string(), ptr);
    }

    if let Instruction::Group { instr, .. } = value {
        compile_integer_var(
            module, builder, context, instr, kind, name, objects, function, ptr,
        );
    }
}

fn compile_float_var<'ctx>(
    module: &Module<'ctx>,
    builder: &Builder<'ctx>,
    context: &'ctx Context,
    value: &'ctx Instruction<'ctx>,
    kind: &'ctx DataTypes,
    name: &str,
    objects: &mut CompilerObjects<'ctx>,
    function: FunctionValue<'ctx>,
    ptr: PointerValue<'ctx>,
) {
    if let Instruction::Null = value {
        builder
            .build_store(ptr, utils::build_const_float(context, kind, 0.0))
            .unwrap();

        objects.insert(name.to_string(), ptr);

        return;
    }

    if let Instruction::Float(_, num, _) = value {
        builder
            .build_store(ptr, utils::build_const_float(context, kind, *num))
            .unwrap();

        objects.insert(name.to_string(), ptr);

        return;
    }

    if let Instruction::RefVar {
        name: name_refvar,
        kind: kind_refvar,
        ..
    } = value
    {
        let var: PointerValue<'ctx> = objects.find_and_get(name_refvar).unwrap();

        let load = builder
            .build_load(
                utils::datatype_float_to_llvm_type(context, kind_refvar),
                var,
                "",
            )
            .unwrap();

        if utils::float_autocast(kind_refvar, kind, Some(ptr), var.into(), builder, context)
            .is_none()
        {
            builder.build_store(ptr, load).unwrap();
        }

        objects.insert(name.to_string(), ptr);
    }

    if let Instruction::Binary {
        left, op, right, ..
    } = value
    {
        let result: FloatValue<'_> = general::compile_binary_op(
            module, builder, context, left, op, right, kind, objects, function,
        )
        .into_float_value();

        builder.build_store(ptr, result).unwrap();

        objects.insert(name.to_string(), ptr);
    }

    if let Instruction::Group { instr, .. } = value {
        compile_float_var(
            module, builder, context, instr, kind, name, objects, function, ptr,
        );
    }
}
