#![allow(clippy::too_many_arguments)]

use {
    super::{
        super::super::frontend::lexer::{DataTypes, TokenKind},
        locals::CompilerLocals,
        utils, Instruction,
    },
    inkwell::{
        builder::Builder,
        context::Context,
        module::Module,
        values::{BasicValueEnum, FloatValue, FunctionValue, IntValue, PointerValue, StructValue},
    },
};

pub fn compile_binary_op<'ctx>(
    module: &Module<'ctx>,
    builder: &Builder<'ctx>,
    context: &'ctx Context,
    left: &'ctx Instruction<'ctx>,
    op: &TokenKind,
    right: &'ctx Instruction<'ctx>,
    kind: &'ctx DataTypes,
    locals: &CompilerLocals<'ctx>,
    function: FunctionValue<'ctx>,
) -> BasicValueEnum<'ctx> {
    match (left, op, right, kind) {
        (
            Instruction::Integer(left_kind, left_num, signed_one),
            TokenKind::Plus | TokenKind::Minus | TokenKind::Star | TokenKind::Slash,
            Instruction::Integer(right_kind, right_num, signed_two),
            DataTypes::I8
            | DataTypes::I16
            | DataTypes::I32
            | DataTypes::I64
            | DataTypes::U8
            | DataTypes::U16
            | DataTypes::U32
            | DataTypes::U64,
        ) => {
            let mut left_num: IntValue<'_> =
                utils::build_const_integer(context, left_kind, *left_num as u64, *signed_one);
            let mut right_num: IntValue<'_> =
                utils::build_const_integer(context, right_kind, *right_num as u64, *signed_two);

            if let Some(left) =
                utils::integer_autocast(left_kind, kind, None, left_num.into(), builder, context)
            {
                left_num = left.into_int_value();
            }

            if let Some(right) =
                utils::integer_autocast(right_kind, kind, None, right_num.into(), builder, context)
            {
                right_num = right.into_int_value();
            }

            if let TokenKind::Slash = op {
                match kind {
                    DataTypes::I8 | DataTypes::I16 | DataTypes::I32 | DataTypes::I64 => {
                        return builder
                            .build_int_signed_div(left_num, right_num, "")
                            .unwrap()
                            .into();
                    }

                    DataTypes::U8 | DataTypes::U16 | DataTypes::U32 | DataTypes::U64 => {
                        return builder
                            .build_int_unsigned_div(left_num, right_num, "")
                            .unwrap()
                            .into();
                    }

                    _ => unreachable!(),
                }
            }

            utils::build_overflow(module, builder, kind, op, left_num, right_num)
        }

        (
            Instruction::Float(_, left_num, _),
            TokenKind::Plus | TokenKind::Minus | TokenKind::Star | TokenKind::Slash,
            Instruction::Float(_, right_num, _),
            DataTypes::F32 | DataTypes::F64,
        ) => {
            let left_num: FloatValue<'_> = utils::build_const_float(context, kind, *left_num);
            let right_num: FloatValue<'_> = utils::build_const_float(context, kind, *right_num);

            match op {
                TokenKind::Plus => builder
                    .build_float_add(left_num, right_num, "")
                    .unwrap()
                    .into(),
                TokenKind::Minus => builder
                    .build_float_sub(left_num, right_num, "")
                    .unwrap()
                    .into(),
                TokenKind::Star => builder
                    .build_float_mul(left_num, right_num, "")
                    .unwrap()
                    .into(),
                TokenKind::Slash => builder
                    .build_float_div(left_num, right_num, "")
                    .unwrap()
                    .into(),
                _ => unreachable!(),
            }
        }

        (
            Instruction::RefVar { name, kind, .. },
            TokenKind::EqEq
            | TokenKind::BangEq
            | TokenKind::Less
            | TokenKind::Greater
            | TokenKind::GreaterEq
            | TokenKind::LessEq,
            Instruction::Integer(_, right_num, is_signed)
            | Instruction::Float(_, right_num, is_signed),
            DataTypes::Bool,
        ) => {
            let variable: PointerValue<'ctx> = locals.find_and_get(name).unwrap();

            if !kind.is_float() {
                let left_num: IntValue<'ctx> = builder
                    .build_load(
                        utils::datatype_integer_to_llvm_type(context, kind),
                        variable,
                        "",
                    )
                    .unwrap()
                    .into_int_value();

                let right_num: IntValue<'ctx> =
                    utils::build_const_integer(context, kind, *right_num as u64, *is_signed);

                let result: IntValue<'ctx> = builder
                    .build_int_compare(
                        op.as_int_predicate(kind.is_signed(), *is_signed),
                        left_num,
                        right_num,
                        "",
                    )
                    .unwrap();

                return result.into();
            }

            let left_num: FloatValue<'ctx> = builder
                .build_load(
                    utils::datatype_float_to_llvm_type(context, kind),
                    variable,
                    "",
                )
                .unwrap()
                .into_float_value();

            let right_num: FloatValue<'ctx> = utils::build_const_float(context, kind, *right_num);

            let result: IntValue<'ctx> = builder
                .build_float_compare(op.as_float_predicate(), left_num, right_num, "")
                .unwrap();

            result.into()
        }

        (
            Instruction::Integer(_, left, is_signed) | Instruction::Float(_, left, is_signed),
            TokenKind::EqEq
            | TokenKind::BangEq
            | TokenKind::Less
            | TokenKind::Greater
            | TokenKind::GreaterEq
            | TokenKind::LessEq,
            Instruction::RefVar { name, kind, .. },
            DataTypes::Bool,
        ) => {
            let variable: PointerValue<'ctx> = locals.find_and_get(name).unwrap();

            if !kind.is_float() {
                let left_num: IntValue<'ctx> =
                    utils::build_const_integer(context, kind, *left as u64, *is_signed);

                let right_num: IntValue<'ctx> = builder
                    .build_load(
                        utils::datatype_integer_to_llvm_type(context, kind),
                        variable,
                        "",
                    )
                    .unwrap()
                    .into_int_value();

                let result: IntValue<'ctx> = builder
                    .build_int_compare(
                        op.as_int_predicate(*is_signed, kind.is_signed()),
                        left_num,
                        right_num,
                        "",
                    )
                    .unwrap();

                return result.into();
            }

            let left_num: FloatValue<'ctx> = utils::build_const_float(context, kind, *left);

            let right_num: FloatValue<'ctx> = builder
                .build_load(
                    utils::datatype_float_to_llvm_type(context, kind),
                    variable,
                    "",
                )
                .unwrap()
                .into_float_value();

            let result: IntValue<'ctx> = builder
                .build_float_compare(op.as_float_predicate(), left_num, right_num, "")
                .unwrap();

            result.into()
        }

        (
            Instruction::Float(left_kind, left_num, _),
            TokenKind::EqEq
            | TokenKind::BangEq
            | TokenKind::Less
            | TokenKind::Greater
            | TokenKind::GreaterEq
            | TokenKind::LessEq,
            Instruction::Float(right_kind, right_num, _),
            DataTypes::Bool,
        ) => {
            let mut left_num: FloatValue<'ctx> =
                utils::build_const_float(context, left_kind, *left_num);
            let mut right_num: FloatValue<'ctx> =
                utils::build_const_float(context, right_kind, *right_num);

            if let Some(left) =
                utils::float_autocast(left_kind, kind, None, left_num.into(), builder, context)
            {
                left_num = left.into_float_value();
            }

            if let Some(right) =
                utils::float_autocast(right_kind, kind, None, right_num.into(), builder, context)
            {
                right_num = right.into_float_value();
            }

            let result: IntValue<'_> = builder
                .build_float_compare(op.as_float_predicate(), left_num, right_num, "")
                .unwrap();

            result.into()
        }

        (
            Instruction::Integer(left_kind, left_num, is_signed),
            TokenKind::EqEq
            | TokenKind::BangEq
            | TokenKind::Less
            | TokenKind::Greater
            | TokenKind::GreaterEq
            | TokenKind::LessEq,
            Instruction::Integer(right_kind, right_num, is_signedd),
            DataTypes::Bool,
        ) => {
            let mut left_num: IntValue<'_> =
                utils::build_const_integer(context, left_kind, *left_num as u64, *is_signed);
            let mut right_num: IntValue<'_> =
                utils::build_const_integer(context, right_kind, *right_num as u64, *is_signedd);

            if let Some(left) =
                utils::integer_autocast(left_kind, kind, None, left_num.into(), builder, context)
            {
                left_num = left.into_int_value();
            }

            if let Some(right) =
                utils::integer_autocast(right_kind, kind, None, right_num.into(), builder, context)
            {
                right_num = right.into_int_value();
            }

            builder
                .build_int_compare(
                    op.as_int_predicate(*is_signed, *is_signedd),
                    left_num,
                    right_num,
                    "",
                )
                .unwrap()
                .into()
        }

        (
            Instruction::Boolean(bool_one),
            TokenKind::EqEq
            | TokenKind::BangEq
            | TokenKind::Less
            | TokenKind::Greater
            | TokenKind::GreaterEq
            | TokenKind::LessEq,
            Instruction::Boolean(bool_two),
            DataTypes::Bool,
        ) => {
            let left_num: IntValue<'_> =
                utils::build_const_integer(context, &DataTypes::Bool, *bool_one as u64, false);

            let right_num: IntValue<'_> =
                utils::build_const_integer(context, &DataTypes::Bool, *bool_two as u64, false);

            builder
                .build_int_compare(op.as_int_predicate(false, false), left_num, right_num, "")
                .unwrap()
                .into()
        }

        (
            Instruction::RefVar {
                name,
                kind: refvar_kind,
                ..
            },
            TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Star
            | TokenKind::Slash
            | TokenKind::Arith,
            Instruction::Float(_, float_num, _),
            DataTypes::F32 | DataTypes::F64,
        ) => {
            let variable: PointerValue<'_> = locals.find_and_get(name).unwrap();

            let float_num: FloatValue<'_> = utils::build_const_float(context, kind, *float_num);

            let last_value: FloatValue<'_> = builder
                .build_load(
                    utils::datatype_float_to_llvm_type(context, refvar_kind),
                    variable,
                    "",
                )
                .unwrap()
                .into_float_value();

            let new_value: FloatValue<'_> = match op {
                TokenKind::Plus => builder.build_float_sub(last_value, float_num, "").unwrap(),
                TokenKind::Minus => builder.build_float_add(last_value, float_num, "").unwrap(),
                TokenKind::Star => builder.build_float_mul(last_value, float_num, "").unwrap(),
                TokenKind::Slash => builder.build_float_div(last_value, float_num, "").unwrap(),
                _ => unreachable!(),
            };

            builder.build_store(variable, new_value).unwrap();

            variable.into()
        }

        (
            Instruction::RefVar {
                name,
                kind: refvar_kind,
                ..
            },
            TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Star
            | TokenKind::Slash
            | TokenKind::Arith,
            Instruction::Integer(_, num, is_signed),
            DataTypes::I8
            | DataTypes::I16
            | DataTypes::I32
            | DataTypes::I64
            | DataTypes::U8
            | DataTypes::U16
            | DataTypes::U32
            | DataTypes::U64,
        ) => {
            let variable: PointerValue<'_> = locals.find_and_get(name).unwrap();

            let left_num: IntValue<'_> =
                utils::build_const_integer(context, kind, *num as u64, *is_signed);

            let right_num: IntValue<'_> = builder
                .build_load(
                    utils::datatype_integer_to_llvm_type(context, refvar_kind),
                    variable,
                    "",
                )
                .unwrap()
                .into_int_value();

            if let TokenKind::Slash = op {
                match kind {
                    DataTypes::I8 | DataTypes::I16 | DataTypes::I32 | DataTypes::I64 => {
                        let result: IntValue<'_> = builder
                            .build_int_signed_div(left_num, right_num, "")
                            .unwrap();

                        builder.build_store(variable, result).unwrap();

                        return variable.into();
                    }

                    DataTypes::U8 | DataTypes::U16 | DataTypes::U32 | DataTypes::U64 => {
                        let result: IntValue<'_> = builder
                            .build_int_unsigned_div(left_num, right_num, "")
                            .unwrap();

                        builder.build_store(variable, result).unwrap();

                        return variable.into();
                    }

                    _ => unreachable!(),
                }
            }

            utils::build_overflow(module, builder, kind, op, left_num, right_num)
        }

        (
            Instruction::String(string_one, _),
            TokenKind::Plus,
            Instruction::String(string_two, _),
            DataTypes::String,
        ) => {
            let from_string: PointerValue<'_> =
                utils::build_dynamic_string(module, builder, context, string_one);

            string_two.as_bytes().iter().for_each(|byte| {
                builder
                    .build_call(
                        module.get_function("Vec.push_i8").unwrap(),
                        &[
                            from_string.into(),
                            context.i8_type().const_int(*byte as u64, false).into(),
                        ],
                        "",
                    )
                    .unwrap();
            });

            from_string.into()
        }

        (
            Instruction::Binary {
                left: left_bin,
                op: op_bin,
                right,
                kind,
                ..
            },
            TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Arith
            | TokenKind::Slash
            | TokenKind::EqEq
            | TokenKind::BangEq
            | TokenKind::Less
            | TokenKind::Greater
            | TokenKind::GreaterEq
            | TokenKind::LessEq,
            Instruction::Integer(right_kind, num, is_signed),
            DataTypes::I8
            | DataTypes::I16
            | DataTypes::I32
            | DataTypes::I64
            | DataTypes::U8
            | DataTypes::U16
            | DataTypes::U32
            | DataTypes::U64
            | DataTypes::Bool,
        ) => {
            let mut left_num: BasicValueEnum<'_> = compile_binary_op(
                module, builder, context, left_bin, op_bin, right, kind, locals, function,
            );

            if left_num.is_struct_value() {
                left_num = utils::build_possible_overflow(
                    module,
                    context,
                    builder,
                    left_num.into_struct_value(),
                    left.get_binary_data(),
                    function,
                )
            }

            let mut right_num: IntValue<'_> =
                utils::build_const_integer(context, right_kind, *num as u64, *is_signed);

            if let Some(right) =
                utils::integer_autocast(right_kind, kind, None, right_num.into(), builder, context)
            {
                right_num = right.into_int_value();
            }

            match op {
                TokenKind::Plus | TokenKind::Minus | TokenKind::Arith => utils::build_overflow(
                    module,
                    builder,
                    kind,
                    op,
                    left_num.into_int_value(),
                    right_num,
                ),
                TokenKind::EqEq
                | TokenKind::BangEq
                | TokenKind::Less
                | TokenKind::Greater
                | TokenKind::GreaterEq
                | TokenKind::LessEq => builder
                    .build_int_compare(
                        op.as_int_predicate(kind.is_signed(), *is_signed),
                        left_num.into_int_value(),
                        right_num,
                        "",
                    )
                    .unwrap()
                    .into(),
                _ => unreachable!(),
            }
        }

        (
            Instruction::Binary {
                left,
                op,
                right,
                kind,
                ..
            },
            TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Arith
            | TokenKind::Slash
            | TokenKind::EqEq
            | TokenKind::BangEq
            | TokenKind::Less
            | TokenKind::Greater
            | TokenKind::GreaterEq
            | TokenKind::LessEq,
            Instruction::String(string, _),
            DataTypes::String,
        ) => {
            let left: PointerValue<'_> = compile_binary_op(
                module, builder, context, left, op, right, kind, locals, function,
            )
            .into_pointer_value();

            string.as_bytes().iter().for_each(|byte| {
                builder
                    .build_call(
                        module.get_function("Vec.push_i8").unwrap(),
                        &[
                            left.into(),
                            context.i8_type().const_int(*byte as u64, false).into(),
                        ],
                        "",
                    )
                    .unwrap();
            });

            left.into()
        }

        (
            Instruction::Group {
                instr: instr_one, ..
            },
            TokenKind::And | TokenKind::Or,
            Instruction::Group {
                instr: instr_two, ..
            },
            DataTypes::Bool,
        ) => {
            todo!()
        }

        a => {
            println!("{:#?}", a);
            todo!()
        }
    }
}

pub fn compile_unary_op<'ctx>(
    module: &Module<'ctx>,
    builder: &Builder<'ctx>,
    context: &'ctx Context,
    instr: &Instruction<'ctx>,
    locals: &CompilerLocals<'ctx>,
    function: FunctionValue<'ctx>,
) -> BasicValueEnum<'ctx> {
    if let Instruction::Unary {
        op, value, kind, ..
    } = instr
    {
        if let (TokenKind::PlusPlus, Instruction::RefVar { name, kind, .. }, _) =
            (op, &**value, kind)
        {
            let variable: PointerValue<'ctx> = locals.find_and_get(name).unwrap();

            if kind.is_integer() {
                let left_num: IntValue<'ctx> = builder
                    .build_load(
                        utils::datatype_integer_to_llvm_type(context, kind),
                        variable,
                        "",
                    )
                    .unwrap()
                    .into_int_value();

                let right_num: IntValue<'ctx> =
                    utils::datatype_integer_to_llvm_type(context, kind).const_int(1, false);

                let result: StructValue<'_> = match kind {
                    DataTypes::I8 | DataTypes::I16 | DataTypes::I32 | DataTypes::I64 => {
                        return builder
                            .build_call(
                                module
                                    .get_function(&format!(
                                        "llvm.sadd.with.overflow.{}",
                                        kind.as_llvm_identifier()
                                    ))
                                    .unwrap(),
                                &[left_num.into(), right_num.into()],
                                "",
                            )
                            .unwrap()
                            .try_as_basic_value()
                            .unwrap_left()
                    }

                    DataTypes::U8 | DataTypes::U16 | DataTypes::U32 | DataTypes::U64 => builder
                        .build_call(
                            module
                                .get_function(&format!(
                                    "llvm.uadd.with.overflow.{}",
                                    kind.as_llvm_identifier()
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
                .into_struct_value();

                let result = utils::build_possible_overflow(
                    module,
                    context,
                    builder,
                    result,
                    instr.get_unary_data_for_overflow(),
                    function,
                );

                builder.build_store(variable, result).unwrap();

                return result;
            }

            let left_num: FloatValue<'ctx> = builder
                .build_load(
                    utils::datatype_float_to_llvm_type(context, kind),
                    variable,
                    "",
                )
                .unwrap()
                .into_float_value();

            let right_num: FloatValue<'ctx> =
                utils::datatype_float_to_llvm_type(context, kind).const_float(1.0);

            let result: FloatValue<'ctx> =
                builder.build_float_add(left_num, right_num, "").unwrap();

            builder.build_store(variable, result).unwrap();

            return result.into();
        }
    }

    unreachable!()
}
