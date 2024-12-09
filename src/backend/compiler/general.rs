use {
    super::{
        super::super::frontend::lexer::{DataTypes, TokenKind},
        utils, Instruction,
    },
    inkwell::{
        builder::Builder,
        context::Context,
        module::Module,
        values::{BasicValueEnum, IntValue},
        IntPredicate,
    },
};

const INTEGER_VALID_TYPES: [DataTypes; 8] = [
    DataTypes::I8,
    DataTypes::I16,
    DataTypes::I32,
    DataTypes::I64,
    DataTypes::U8,
    DataTypes::U16,
    DataTypes::U32,
    DataTypes::U64,
];

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
            Instruction::Integer(left_kind, left_num),
            TokenKind::Eq
            | TokenKind::BangEqual
            | TokenKind::Less
            | TokenKind::Greater
            | TokenKind::GreaterEqual
            | TokenKind::LessEqual,
            Instruction::Integer(right_kind, right_num),
            DataTypes::Bool,
        ) => {
            if !INTEGER_VALID_TYPES.contains(left_kind) || !INTEGER_VALID_TYPES.contains(right_kind)
            {
                todo!()
            }

            let left_num: IntValue<'_> =
                utils::build_const_integer(context, left_kind, *left_num as u64);
            let right_num: IntValue<'_> =
                utils::build_const_integer(context, right_kind, *right_num as u64);

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
