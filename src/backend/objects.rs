use {super::super::frontend::lexer::DataTypes, inkwell::values::BasicValueEnum};

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
