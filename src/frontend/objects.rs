use super::super::frontend::lexer::DataTypes;

pub struct Variable {
    pub kind: DataTypes,
    pub is_null: bool,
}

impl Variable {
    pub fn new(kind: DataTypes, is_null: bool) -> Self {
        Self { kind, is_null }
    }
}
