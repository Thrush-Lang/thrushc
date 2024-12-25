#![allow(clippy::too_many_arguments)]

use {
    super::{
        super::super::{
            diagnostic::{self, Diagnostic},
            frontend::lexer::{DataTypes, TokenKind},
        },
        locals::CompilerLocals,
        options::ThrushFile,
        utils, Instruction,
    },
    inkwell::{
        basic_block::BasicBlock,
        builder::Builder,
        context::Context,
        module::Module,
        values::{
            BasicValueEnum, FloatValue, FunctionValue, InstructionValue, IntValue, PointerValue,
            StructValue,
        },
    },
};

pub fn compile_binary_op<'ctx>(
    module: &Module<'ctx>,
    builder: &Builder<'ctx>,
    context: &'ctx Context,
    left: &'ctx Instruction<'ctx>,
    op: &TokenKind,
    right: &'ctx Instruction<'ctx>,
    kind: &DataTypes,
    locals: &CompilerLocals<'ctx>,
    _file: &ThrushFile,
) -> BasicValueEnum<'ctx> {
    match (left, op, right, kind) {
        (
            Instruction::Integer(left_kind, left_num),
            TokenKind::Plus | TokenKind::Minus | TokenKind::Star | TokenKind::Slash,
            Instruction::Integer(right_kind, right_num),
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
                utils::build_const_integer(context, left_kind, *left_num as u64);
            let mut right_num: IntValue<'_> =
                utils::build_const_integer(context, right_kind, *right_num as u64);

            if utils::datatype_int_to_type(context, kind) != left_num.get_type() {
                left_num = builder
                    .build_int_cast(left_num, utils::datatype_int_to_type(context, kind), "")
                    .unwrap();
            }

            if utils::datatype_int_to_type(context, kind) != right_num.get_type() {
                right_num = builder
                    .build_int_cast(right_num, utils::datatype_int_to_type(context, kind), "")
                    .unwrap();
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

        (
            Instruction::Float(left_kind, left_num),
            TokenKind::Plus | TokenKind::Minus | TokenKind::Star | TokenKind::Slash,
            Instruction::Float(right_kind, right_num),
            DataTypes::F32 | DataTypes::F64,
        ) => {
            let mut left_num: FloatValue<'_> =
                utils::build_const_float(context, left_kind, *left_num);
            let mut right_num: FloatValue<'_> =
                utils::build_const_float(context, right_kind, *right_num);

            if right_kind != kind {
                left_num = builder
                    .build_float_cast(left_num, utils::datatype_float_to_type(context, kind), "")
                    .unwrap();
            }

            if left_kind != kind {
                right_num = builder
                    .build_float_cast(right_num, utils::datatype_float_to_type(context, kind), "")
                    .unwrap();
            }

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
            Instruction::Integer(_, right_num),
            DataTypes::Bool,
        ) => {
            let variable: PointerValue<'ctx> =
                locals.find_and_get(name).unwrap().into_pointer_value();

            let left_num: IntValue<'ctx> = builder
                .build_load(utils::datatype_int_to_type(context, kind), variable, "")
                .unwrap()
                .into_int_value();

            let right_num: IntValue<'ctx> =
                utils::build_const_integer(context, kind, *right_num as u64);

            let result: IntValue<'ctx> = builder
                .build_int_compare(op.to_int_predicate(), left_num, right_num, "")
                .unwrap();

            result.into()
        }

        (
            Instruction::Float(left_kind, left_num),
            TokenKind::EqEq
            | TokenKind::BangEq
            | TokenKind::Less
            | TokenKind::Greater
            | TokenKind::GreaterEq
            | TokenKind::LessEq,
            Instruction::Float(right_kind, right_num),
            DataTypes::Bool,
        ) => {
            let mut left_num: FloatValue<'ctx> =
                utils::build_const_float(context, left_kind, *left_num);
            let mut right_num: FloatValue<'ctx> =
                utils::build_const_float(context, right_kind, *right_num);

            if right_num.get_type() != left_num.get_type() {
                right_num = builder
                    .build_float_cast(right_num, left_num.get_type(), "")
                    .unwrap();
            }

            if left_num.get_type() != right_num.get_type() {
                left_num = builder
                    .build_float_cast(left_num, right_num.get_type(), "")
                    .unwrap();
            }

            let result: IntValue<'_> = builder
                .build_float_compare(op.to_float_predicate(), left_num, right_num, "")
                .unwrap();

            result.into()
        }

        (
            Instruction::Integer(left_kind, left_num),
            TokenKind::EqEq
            | TokenKind::BangEq
            | TokenKind::Less
            | TokenKind::Greater
            | TokenKind::GreaterEq
            | TokenKind::LessEq,
            Instruction::Integer(right_kind, right_num),
            DataTypes::Bool,
        ) => {
            let mut left_num: IntValue<'_> =
                utils::build_const_integer(context, left_kind, *left_num as u64);
            let mut right_num: IntValue<'_> =
                utils::build_const_integer(context, right_kind, *right_num as u64);

            if right_num.get_type() != left_num.get_type() {
                right_num = builder
                    .build_int_cast(right_num, left_num.get_type(), "")
                    .unwrap();
            }

            if left_num.get_type() != right_num.get_type() {
                left_num = builder
                    .build_int_cast(left_num, right_num.get_type(), "")
                    .unwrap();
            }

            builder
                .build_int_compare(op.to_int_predicate(), left_num, right_num, "")
                .unwrap()
                .into()
        }
        (
            Instruction::Binary {
                left,
                op,
                right,
                kind,
                ..
            },
            TokenKind::EqEq
            | TokenKind::BangEq
            | TokenKind::Less
            | TokenKind::Greater
            | TokenKind::GreaterEq
            | TokenKind::LessEq,
            Instruction::Boolean(value),
            DataTypes::Bool,
        ) => {
            let left: BasicValueEnum<'_> = compile_binary_op(
                module, builder, context, left, op, right, kind, locals, _file,
            );

            let mut right: BasicValueEnum<'_> = if *value {
                context.bool_type().const_int(1, false).into()
            } else {
                context.bool_type().const_int(0, false).into()
            };

            if left.is_float_value() {
                right = if *value {
                    context.f64_type().const_float(1.0).into()
                } else {
                    context.f64_type().const_float(0.0).into()
                };

                return builder
                    .build_float_compare(
                        op.to_float_predicate(),
                        left.into_float_value(),
                        right.into_float_value(),
                        "",
                    )
                    .unwrap()
                    .into();
            }

            builder
                .build_int_compare(
                    op.to_int_predicate(),
                    left.into_int_value(),
                    right.into_int_value(),
                    "",
                )
                .unwrap()
                .into()
        }

        (
            Instruction::Boolean(value),
            TokenKind::EqEq
            | TokenKind::BangEq
            | TokenKind::Less
            | TokenKind::Greater
            | TokenKind::GreaterEq
            | TokenKind::LessEq,
            Instruction::Binary {
                left,
                op,
                right,
                kind,
                ..
            },
            DataTypes::Bool,
        ) => {
            let left: BasicValueEnum<'_> = compile_binary_op(
                module, builder, context, left, op, right, kind, locals, _file,
            );

            let mut right: BasicValueEnum<'_> = if *value {
                context.bool_type().const_int(1, false).into()
            } else {
                context.bool_type().const_int(0, false).into()
            };

            if left.is_float_value() {
                right = if *value {
                    context.f64_type().const_float(1.0).into()
                } else {
                    context.f64_type().const_float(0.0).into()
                };

                return builder
                    .build_float_compare(
                        op.to_float_predicate(),
                        left.into_float_value(),
                        right.into_float_value(),
                        "",
                    )
                    .unwrap()
                    .into();
            }

            builder
                .build_int_compare(
                    op.to_int_predicate(),
                    left.into_int_value(),
                    right.into_int_value(),
                    "",
                )
                .unwrap()
                .into()
        }

        (
            Instruction::RefVar { name, kind, .. },
            TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Star
            | TokenKind::Slash
            | TokenKind::Arith,
            Instruction::Float(float_kind, float_num),
            DataTypes::Integer,
        ) => {
            let variable: PointerValue<'_> =
                locals.find_and_get(name).unwrap().into_pointer_value();

            let mut float_num: FloatValue<'_> =
                utils::build_const_float(context, float_kind, *float_num);

            if float_num.get_type() != utils::datatype_float_to_type(context, kind) {
                float_num = builder
                    .build_float_cast(float_num, utils::datatype_float_to_type(context, kind), "")
                    .unwrap();
            }

            let last_value: FloatValue<'_> = builder
                .build_load(utils::datatype_float_to_type(context, kind), variable, "")
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
            Instruction::RefVar { name, kind, .. },
            TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Star
            | TokenKind::Slash
            | TokenKind::Arith,
            Instruction::Integer(integer_kind, num),
            DataTypes::Integer,
        ) => {
            let variable: PointerValue<'_> =
                locals.find_and_get(name).unwrap().into_pointer_value();

            let mut left_value: IntValue<'_> =
                utils::build_const_integer(context, integer_kind, *num as u64);

            if left_value.get_type() != utils::datatype_int_to_type(context, kind) {
                left_value = builder
                    .build_int_cast(left_value, utils::datatype_int_to_type(context, kind), "")
                    .unwrap();
            }

            let right_value: IntValue<'_> = builder
                .build_load(utils::datatype_int_to_type(context, kind), variable, "")
                .unwrap()
                .into_int_value();

            if let TokenKind::Slash = op {
                match kind {
                    DataTypes::I8 | DataTypes::I16 | DataTypes::I32 | DataTypes::I64 => {
                        let result: IntValue<'_> = builder
                            .build_int_signed_div(left_value, right_value, "")
                            .unwrap();

                        builder.build_store(variable, result).unwrap();

                        return variable.into();
                    }

                    DataTypes::U8 | DataTypes::U16 | DataTypes::U32 | DataTypes::U64 => {
                        let result: IntValue<'_> = builder
                            .build_int_unsigned_div(left_value, right_value, "")
                            .unwrap();

                        builder.build_store(variable, result).unwrap();

                        return variable.into();
                    }

                    _ => unreachable!(),
                }
            }

            match kind {
                DataTypes::I8 => builder
                    .build_call(
                        module
                            .get_function(&format!(
                                "llvm.s{}.with.overflow.i8",
                                op.to_llvm_intrinsic_identifier()
                            ))
                            .unwrap(),
                        &[left_value.into(), right_value.into()],
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
                        &[left_value.into(), right_value.into()],
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
                        &[left_value.into(), right_value.into()],
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
                        &[left_value.into(), right_value.into()],
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
                        &[left_value.into(), right_value.into()],
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
                        &[left_value.into(), right_value.into()],
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
                        &[left_value.into(), right_value.into()],
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
                        &[left_value.into(), right_value.into()],
                        "",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .unwrap_left(),
                _ => unreachable!(),
            }
        }

        a => {
            println!("{:?}", a);
            todo!()
        }
    }
}

pub fn compile_unary_op<'ctx>(
    module: &Module<'ctx>,
    builder: &Builder<'ctx>,
    context: &'ctx Context,
    op: &TokenKind,
    value: &Instruction<'ctx>,
    kind: &DataTypes,
    locals: &CompilerLocals<'ctx>,
    function: &FunctionValue<'ctx>,
    diagnostics: &mut Diagnostic,
    file: &ThrushFile,
) -> BasicValueEnum<'ctx> {
    match (op, value, kind) {
        (TokenKind::PlusPlus, Instruction::RefVar { name, kind, line }, _) => {
            let variable: PointerValue<'ctx> =
                locals.find_and_get(name).unwrap().into_pointer_value();

            if let DataTypes::I8
            | DataTypes::I16
            | DataTypes::I32
            | DataTypes::I64
            | DataTypes::U8
            | DataTypes::U16
            | DataTypes::U32
            | DataTypes::U64 = *kind
            {
                let left_num: IntValue<'ctx> = builder
                    .build_load(utils::datatype_int_to_type(context, kind), variable, "")
                    .unwrap()
                    .into_int_value();

                let right_num: IntValue<'ctx> =
                    utils::datatype_int_to_type(context, kind).const_int(1, false);

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
                            "{}

Details:

    ● File: {}
    ● Line: {}
    ● Instruction: {} {} {}
    ● Operation: {}

    Code:

        {}

{} \n\0",
                            diagnostic::create_panic_message("Integer / Float Overflow"),
                            file.path.to_string_lossy(),
                            line,
                            kind,
                            TokenKind::Plus,
                            kind,
                            op,
                            diagnostics.draw_only_line(*line),
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

                diagnostics.clear();

                builder.build_unreachable().unwrap();

                builder.position_at_end(false_block);

                let result: IntValue<'_> = builder
                    .build_extract_value(result, 0, "")
                    .unwrap()
                    .into_int_value();

                builder.build_store(variable, result).unwrap();

                return result.into();
            }

            let left_num: FloatValue<'ctx> = builder
                .build_load(utils::datatype_float_to_type(context, kind), variable, "")
                .unwrap()
                .into_float_value();

            let right_num: FloatValue<'ctx> =
                utils::datatype_float_to_type(context, kind).const_float(1.0);

            let result: FloatValue<'ctx> =
                builder.build_float_add(left_num, right_num, "").unwrap();

            let store: InstructionValue<'ctx> = builder.build_store(variable, result).unwrap();

            store.set_alignment(4).unwrap();

            result.into()
        }

        _ => unreachable!(),
    }
}
