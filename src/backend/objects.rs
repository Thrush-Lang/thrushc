use {
    super::{
        super::{error::ThrushError, frontend::lexer::DataTypes},
        compiler::Instruction,
    },
    inkwell::values::BasicValueEnum,
};

#[derive(Debug, Clone)]
pub struct ThrushBasicValueEnum<'ctx> {
    pub kind: DataTypes,
    pub value: BasicValueEnum<'ctx>,
}

impl<'ctx> ThrushBasicValueEnum<'ctx> {
    pub fn new(kind: DataTypes, value: BasicValueEnum<'ctx>) -> Self {
        Self { kind, value }
    }
}
