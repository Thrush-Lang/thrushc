use {super::super::frontend::lexer::DataTypes, inkwell::values::BasicValueEnum};

#[derive(Debug, Clone)]
pub struct ThrushBasicValueEnum<'ctx> {
    pub kind: DataTypes,
    pub value: BasicValueEnum<'ctx>,
}
