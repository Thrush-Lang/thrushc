use {
    super::{
        super::{error::ThrushError, frontend::lexer::DataTypes},
        compiler::Instruction,
    },
    inkwell::values::BasicValueEnum,
    std::mem,
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

pub struct ThrushScoper<'ctx> {
    blocks: Vec<ThrushBlock<'ctx>>,
    count: usize,
}

pub struct ThrushBlock<'ctx> {
    instructions: Vec<ThrushInstruction<'ctx>>,
    position: usize,
    in_function: bool,
    in_loop: bool,
}

#[derive(Debug)]
pub struct ThrushInstruction<'ctx> {
    pub instr: Instruction<'ctx>,
    pub depth: usize,
}

impl<'ctx> ThrushScoper<'ctx> {
    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
            count: 0,
        }
    }

    pub fn begin_scope(&mut self, instr: Instruction<'ctx>, in_function: bool, in_loop: bool) {
        self.count += 1;

        if let Instruction::Block(body) = instr {
            let mut instructions: Vec<ThrushInstruction> = Vec::with_capacity(body.len());

            for (depth, instr) in body.iter().enumerate() {
                instructions.push(ThrushInstruction {
                    instr: instr.clone(),
                    depth,
                });
            }

            self.blocks.push(ThrushBlock::new(
                instructions,
                self.count,
                in_function,
                in_loop,
            ));
        }
    }

    pub fn analyze(&self) -> Result<(), ThrushError> {
        for instr in &self.blocks[self.count - 1].instructions {
            self.analyze_instruction(&instr.instr, instr.depth)?;
        }

        Ok(())
    }

    fn analyze_instruction(
        &self,
        instr: &Instruction<'ctx>,
        depth: usize,
    ) -> Result<(), ThrushError> {
        if let Instruction::Block(body) = instr {
            for instr in body {
                self.analyze_instruction(instr, depth + 1)?;
            }
        }

        if let Instruction::Function { body, .. } = instr {
            self.analyze_instruction(body, depth)?;
        }

        if let Instruction::EntryPoint { body } = instr {
            self.analyze_instruction(body, depth)?;
        }

        if let Instruction::Print(params) = instr {
            for instr in params {
                self.analyze_instruction(instr, depth)?;
            }
        }

        match instr {
            Instruction::String(_) => Ok(()),
            Instruction::Var { .. } => Ok(()),
            Instruction::RefVar { name, .. } => {
                if !self.is_at_current_scope(name) && !self.is_at_top_scope(name) {
                    return Err(ThrushError::Compile(format!(
                        "Variable: `{}` is not defined.",
                        name
                    )));
                }

                if self.is_at_current_scope(name)
                    && !self.is_reacheable_at_curret_scope(name, depth)
                {
                    return Err(ThrushError::Compile(format!(
                        "Variable: `{}` is unreacheable in this scope.",
                        name
                    )));
                }

                if self.is_at_top_scope(name) && !self.is_reacheable_at_top_scope(name, depth) {
                    return Err(ThrushError::Compile(format!(
                        "Variable: `{}` is unreacheable in the top scope.",
                        name
                    )));
                }

                Ok(())
            }

            e => {
                println!("{:?}", e);

                todo!()
            }
        }
    }

    fn is_reacheable_at_curret_scope(&self, name: &str, depth: usize) -> bool {
        self.blocks[self.count - 1]
            .instructions
            .iter()
            .any(|instr| match instr.instr {
                Instruction::Var { name: n, .. } => *n == *name && depth >= instr.depth,
                _ => false,
            })
    }

    fn is_at_current_scope(&self, name: &str) -> bool {
        self.blocks[self.count - 1]
            .instructions
            .iter()
            .any(|instr| match instr.instr {
                Instruction::Var { name: n, .. } => *n == *name,
                _ => false,
            })
    }

    fn is_reacheable_at_top_scope(&self, name: &str, depth: usize) -> bool {
        for block in &self.blocks {
            if block.in_function || block.in_loop {
                continue;
            }

            if block.position == self.blocks[self.count - 1].position {
                return false;
            }

            if block.instructions.iter().any(|instr| match instr.instr {
                Instruction::Var { name: n, .. } => *n == *name && depth >= instr.depth,
                _ => false,
            }) {
                return true;
            }
        }

        false
    }

    fn is_at_top_scope(&self, name: &str) -> bool {
        for block in &self.blocks {
            if block.in_function || block.in_loop {
                continue;
            }

            if block.position == self.blocks[self.count - 1].position {
                return false;
            }

            if block.instructions.iter().any(|instr| match instr.instr {
                Instruction::Var { name: n, .. } => *n == *name,
                _ => false,
            }) {
                return true;
            }
        }

        false
    }

    pub fn end_scope(&mut self) {
        self.blocks.pop();
        self.count -= 1;
    }
}

impl<'ctx> ThrushBlock<'ctx> {
    pub fn new(
        instructions: Vec<ThrushInstruction<'ctx>>,
        position: usize,
        in_function: bool,
        in_loop: bool,
    ) -> Self {
        Self {
            instructions,
            position,
            in_function,
            in_loop,
        }
    }
}
