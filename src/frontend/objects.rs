use super::super::frontend::lexer::DataTypes;

pub struct Variable {
    pub kind: DataTypes,
    pub is_null: bool,
    pub references: usize,
}

impl Variable {
    pub fn new(kind: DataTypes, is_null: bool, references: usize) -> Self {
        Self {
            kind,
            is_null,
            references,
        }
    }

    #[inline]
    pub fn increase_ref(&mut self) {
        self.references += 1;
    }

    #[inline]
    pub fn decrease_ref(&mut self) {
        self.references -= 1;
    }
}
