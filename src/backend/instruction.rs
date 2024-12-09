use {
    super::super::frontend::lexer::{DataTypes, TokenKind},
    inkwell::values::BasicValueEnum,
};

#[derive(Debug, Clone)]
pub enum Instruction<'ctx> {
    BasicValueEnum(BasicValueEnum<'ctx>),
    Println(Vec<Instruction<'ctx>>),
    Print(Vec<Instruction<'ctx>>),
    String(String),
    Char(u8),
    Integer(DataTypes, f64),
    Float(DataTypes, f64),
    Block {
        stmts: Vec<Instruction<'ctx>>,
    },
    EntryPoint {
        body: Box<Instruction<'ctx>>,
    },
    Param {
        name: &'ctx str,
        kind: DataTypes,
    },
    Function {
        name: &'ctx str,
        params: Vec<Instruction<'ctx>>,
        body: Box<Instruction<'ctx>>,
        return_kind: Option<DataTypes>,
        is_public: bool,
    },
    Return(Box<Instruction<'ctx>>),
    Var {
        name: &'ctx str,
        kind: DataTypes,
        value: Box<Instruction<'ctx>>,
        line: usize,
    },
    RefVar {
        name: &'ctx str,
        line: usize,
        kind: DataTypes,
    },
    MutVar {
        name: &'ctx str,
        kind: DataTypes,
        value: Box<Instruction<'ctx>>,
    },
    Indexe {
        origin: &'ctx str,
        name: Option<&'ctx str>,
        index: u64,
        kind: DataTypes,
    },
    Binary {
        left: Box<Instruction<'ctx>>,
        op: &'ctx TokenKind,
        right: Box<Instruction<'ctx>>,
        kind: DataTypes,
        line: usize,
    },
    Unary {
        op: &'ctx TokenKind,
        value: Box<Instruction<'ctx>>,
        kind: DataTypes,
    },
    Group {
        instr: Box<Instruction<'ctx>>,
    },
    Free {
        name: &'ctx str,
        is_string: bool,
    },
    Boolean(bool),

    Null,
}

impl PartialEq for Instruction<'_> {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Instruction::Integer(_, _) => {
                matches!(other, Instruction::Integer(_, _))
            }

            Instruction::Float(_, _) => {
                matches!(other, Instruction::Float(_, _))
            }

            Instruction::String(_) => {
                matches!(other, Instruction::String(_))
            }

            _ => self == other,
        }
    }
}

impl Instruction<'_> {
    pub fn get_data_type(&self) -> DataTypes {
        match self {
            Instruction::Integer(data_type, _) => match data_type {
                DataTypes::U8 => DataTypes::U8,
                DataTypes::U16 => DataTypes::U16,
                DataTypes::U32 => DataTypes::U32,
                DataTypes::U64 => DataTypes::U64,

                DataTypes::I8 => DataTypes::I8,
                DataTypes::I16 => DataTypes::I16,
                DataTypes::I32 => DataTypes::I32,
                DataTypes::I64 => DataTypes::I64,

                DataTypes::F32 => DataTypes::F32,
                DataTypes::F64 => DataTypes::F64,

                _ => unreachable!(),
            },

            Instruction::Float(data_type, _) => match data_type {
                DataTypes::F32 => DataTypes::F32,
                DataTypes::F64 => DataTypes::F64,
                _ => unreachable!(),
            },

            Instruction::String(_) => DataTypes::String,
            Instruction::Boolean(_) => DataTypes::Bool,
            Instruction::Char(_) => DataTypes::Char,
            Instruction::RefVar { kind, .. } => *kind,
            Instruction::Group { instr } => instr.get_data_type(),
            Instruction::Binary { kind, .. } => *kind,
            Instruction::Unary { kind, .. } => *kind,

            e => {
                println!("{:?}", e);

                unimplemented!()
            }
        }
    }

    pub fn get_kind(&self) -> Option<DataTypes> {
        match self {
            Instruction::Var { kind, .. } => Some(*kind),
            Instruction::Char(_) => Some(DataTypes::Char),
            Instruction::Integer(kind, _) => Some(*kind),
            Instruction::Float(kind, _) => Some(*kind),
            _ => None,
        }
    }
}
