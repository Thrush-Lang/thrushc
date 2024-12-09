use {
    super::{
        super::super::frontend::lexer::{DataTypes, TokenKind},
        utils, Instruction,
    },
    inkwell::{
        builder::Builder,
        context::Context,
        module::Module,
        values::{BasicValueEnum, FloatValue, IntValue},
    },
};

pub fn compile_binary_op<'ctx>(
    module: &'ctx Module,
    builder: &'ctx Builder,
    context: &'ctx Context,
    left: &'ctx Instruction<'ctx>,
    op: &TokenKind,
    right: &'ctx Instruction<'ctx>,
    kind: &DataTypes,
) -> BasicValueEnum<'ctx> {
    match (left, op, right, kind) {
        (
            Instruction::Integer(left_kind, left_num),
            TokenKind::Plus | TokenKind::Minus | TokenKind::Star,
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
            TokenKind::Eq
            | TokenKind::BangEq
            | TokenKind::Less
            | TokenKind::Greater
            | TokenKind::GreaterEq
            | TokenKind::LessEq,
            Instruction::Float(right_kind, right_num),
            DataTypes::Bool,
        ) => {
            let mut left_num: FloatValue<'_> =
                utils::build_const_float(context, left_kind, *left_num);
            let mut right_num: FloatValue<'_> =
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

            builder
                .build_float_compare(op.to_float_predicate(), left_num, right_num, "")
                .unwrap()
                .into()
        }

        (
            Instruction::Integer(left_kind, left_num),
            TokenKind::Eq
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
        _ => {
            todo!()
        }
    }
}
