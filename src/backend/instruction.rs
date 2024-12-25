use {
    super::super::frontend::lexer::{DataTypes, TokenKind},
    inkwell::values::BasicValueEnum,
};

#[derive(Debug, Clone, Default)]
pub enum Instruction<'ctx> {
    BasicValueEnum(BasicValueEnum<'ctx>),
    Println(Vec<Instruction<'ctx>>),
    Print(Vec<Instruction<'ctx>>),
    String(String),
    Char(u8),
    ForLoop {
        variable: Option<Box<Instruction<'ctx>>>,
        cond: Option<Box<Instruction<'ctx>>>,
        actions: Option<Box<Instruction<'ctx>>>,
        block: Box<Instruction<'ctx>>,
    },
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
        comptime: bool,
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

    #[default]
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

impl<'ctx> Instruction<'ctx> {
    pub fn get_data_type(&self) -> DataTypes {
        match self {
            Instruction::Integer(data_type, _) => *data_type,
            Instruction::Float(data_type, _) => *data_type,
            Instruction::String(_) => DataTypes::String,
            Instruction::Boolean(_) => DataTypes::Bool,
            Instruction::Char(_) => DataTypes::Char,
            Instruction::RefVar { kind, .. } => *kind,
            Instruction::Group { instr } => instr.get_data_type(),
            Instruction::Binary { left, right, .. } => {
                if left.get_data_type() as u8 > right.get_data_type() as u8 {
                    return left.get_data_type();
                }

                right.get_data_type()
            }
            Instruction::Unary { value, .. } => value.get_data_type(),

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

    pub fn as_basic_value(&self) -> &BasicValueEnum<'ctx> {
        match self {
            Instruction::BasicValueEnum(value) => value,
            _ => unreachable!(),
        }
    }
}
