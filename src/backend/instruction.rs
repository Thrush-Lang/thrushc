use {
    super::{
        super::frontend::lexer::{DataTypes, TokenKind},
        compiler::{general, objects::CompilerObjects},
    },
    inkwell::{
        builder::Builder,
        context::Context,
        module::Module,
        values::{BasicValueEnum, FunctionValue},
    },
};

#[derive(Debug, Clone, Default)]
pub enum Instruction<'ctx> {
    BasicValueEnum(BasicValueEnum<'ctx>),
    Println(Vec<Instruction<'ctx>>),
    Print(Vec<Instruction<'ctx>>),
    String(String, bool),
    Char(u8),
    ForLoop {
        variable: Option<Box<Instruction<'ctx>>>,
        cond: Option<Box<Instruction<'ctx>>>,
        actions: Option<Box<Instruction<'ctx>>>,
        block: Box<Instruction<'ctx>>,
    },
    Integer(DataTypes, f64, bool),
    Float(DataTypes, f64, bool),
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
        external_name: &'ctx str,
        params: Vec<Instruction<'ctx>>,
        body: Option<Box<Instruction<'ctx>>>,
        return_kind: Option<DataTypes>,
        is_public: bool,
        is_external: bool,
    },
    Return(Box<Instruction<'ctx>>, DataTypes),
    Var {
        name: &'ctx str,
        kind: DataTypes,
        value: Box<Instruction<'ctx>>,
        line: usize,
        only_comptime: bool,
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
        index: u64,
        kind: DataTypes,
    },
    Call {
        name: &'ctx str,
        args: Vec<Instruction<'ctx>>,
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
        line: usize,
    },
    Group {
        instr: Box<Instruction<'ctx>>,
        kind: DataTypes,
    },
    Free {
        name: &'ctx str,
        free_only: bool,
        is_string: bool,
    },
    Boolean(bool),
    Pass,

    #[default]
    Null,
}

impl PartialEq for Instruction<'_> {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Instruction::Integer(_, _, _) => {
                matches!(other, Instruction::Integer(_, _, _))
            }

            Instruction::Float(_, _, _) => {
                matches!(other, Instruction::Float(_, _, _))
            }

            Instruction::String(_, _) => {
                matches!(other, Instruction::String(_, _))
            }

            _ => self == other,
        }
    }
}

impl<'ctx> Instruction<'ctx> {
    #[inline]
    pub fn is_binary(&self) -> bool {
        if let Instruction::Binary { .. } = self {
            return true;
        }

        false
    }

    #[inline]
    pub fn is_var(&self) -> bool {
        if let Instruction::Var { .. } | Instruction::RefVar { .. } = self {
            return true;
        }
        false
    }

    #[inline]
    pub fn is_indexe_return_of_string(&self) -> bool {
        if self.is_return() {
            if let Instruction::Return(indexe, DataTypes::Char) = self {
                return indexe.is_indexe();
            }

            return false;
        }

        false
    }

    #[inline]
    pub fn is_indexe(&self) -> bool {
        if let Instruction::Indexe { .. } = self {
            return true;
        }

        false
    }

    #[inline]
    pub fn is_return(&self) -> bool {
        if let Instruction::Return(_, _) = self {
            return true;
        }

        false
    }

    pub fn as_binary(&self) -> (&Instruction, &TokenKind, &Instruction, &DataTypes) {
        if let Instruction::Binary {
            left,
            op,
            right,
            kind,
            ..
        } = self
        {
            return (&**left, op, &**right, kind);
        }

        if let Instruction::Group { instr, .. } = self {
            return instr.as_binary();
        }

        unreachable!()
    }

    pub fn get_binary_data_types(&self) -> (DataTypes, &TokenKind, DataTypes, usize) {
        if let Instruction::Binary {
            left,
            op,
            right,
            line,
            ..
        } = self
        {
            return (left.get_data_type(), op, right.get_data_type(), *line);
        }

        unreachable!()
    }

    pub fn get_unary_data_for_overflow(&self) -> (DataTypes, &TokenKind, DataTypes, usize) {
        if let Instruction::Unary {
            op, value, line, ..
        } = self
        {
            return (value.get_data_type(), op, op.get_possible_datatype(), *line);
        }

        unreachable!()
    }

    pub fn get_data_type(&self) -> DataTypes {
        match self {
            Instruction::Integer(datatype, _, _) => *datatype,
            Instruction::Float(datatype, _, _) => *datatype,
            Instruction::String(_, _) => DataTypes::String,
            Instruction::Boolean(_) => DataTypes::Bool,
            Instruction::Char(_) => DataTypes::Char,
            Instruction::RefVar { kind, .. } => *kind,
            Instruction::Group { kind, .. } => *kind,
            Instruction::Binary { kind, .. } => *kind,
            Instruction::Unary { value, .. } => value.get_data_type(),
            Instruction::Param { kind, .. } => *kind,
            Instruction::Call { kind, .. } => *kind,
            Instruction::Indexe { kind, .. } => *kind,
            e => {
                println!("{:?}", e);

                unimplemented!()
            }
        }
    }

    pub fn compile_group_as_binary(
        &'ctx self,
        module: &Module<'ctx>,
        builder: &Builder<'ctx>,
        context: &'ctx Context,
        locals: &CompilerObjects<'ctx>,
        function: FunctionValue<'ctx>,
    ) -> BasicValueEnum<'ctx> {
        let instr: (&Instruction<'_>, &TokenKind, &Instruction<'_>, &DataTypes) = self.as_binary();

        general::compile_binary_op(
            module, builder, context, instr.0, instr.1, instr.2, instr.3, locals, function,
        )
    }

    pub fn as_basic_value(&self) -> &BasicValueEnum<'ctx> {
        match self {
            Instruction::BasicValueEnum(value) => value,
            _ => unreachable!(),
        }
    }
}
