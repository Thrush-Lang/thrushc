use super::super::{
    backend::compiler::Instruction,
    diagnostic::Diagnostic,
    error::{ThrushError, ThrushErrorKind},
    PATH,
};

#[derive(Debug)]
pub struct ThrushScoper<'ctx> {
    blocks: Vec<ThrushBlock<'ctx>>,
    errors: Vec<ThrushError>,
    diagnostic: Diagnostic,
}

#[derive(Debug)]
struct ThrushBlock<'ctx> {
    stmts: Vec<Instruction<'ctx>>,
}

impl<'ctx> ThrushScoper<'ctx> {
    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
            errors: Vec::new(),
            diagnostic: Diagnostic::new(&PATH.lock().unwrap()),
        }
    }

    pub fn add_scope(&mut self, stmts: Vec<Instruction<'ctx>>) {
        self.blocks.push(ThrushBlock { stmts });
    }

    pub fn analyze(&mut self) -> Result<(), String> {
        if self.blocks.is_empty() {
            return Ok(());
        }

        for depth in (0..=self.blocks.len() - 1).rev() {
            for instr in self.blocks[depth].stmts.iter().rev() {
                match self.analyze_instruction(instr, depth) {
                    Ok(()) => {}
                    Err(e) => {
                        self.errors.push(e);
                    }
                }
            }
        }

        if !self.errors.is_empty() {
            self.errors.iter().for_each(|error| {
                self.diagnostic.report(error);
            });

            return Err(String::from("Compilation terminated."));
        }

        Ok(())
    }

    fn analyze_instruction(
        &self,
        instr: &Instruction<'ctx>,
        depth: usize,
    ) -> Result<(), ThrushError> {
        if let Instruction::Block { stmts, .. } = instr {
            stmts
                .iter()
                .try_for_each(|instr| match self.analyze_instruction(instr, depth) {
                    Ok(()) => Ok(()),
                    Err(e) => Err(e),
                })?;
        }

        if let Instruction::Function { body, .. } = instr {
            self.analyze_instruction(body, depth)?;
        }

        if let Instruction::EntryPoint { body } = instr {
            self.analyze_instruction(body, depth)?;
        }

        if let Instruction::Println(params) = instr {
            params
                .iter()
                .try_for_each(|instr| match self.analyze_instruction(instr, depth) {
                    Ok(()) => Ok(()),
                    Err(e) => Err(e),
                })?;
        }

        match instr {
            Instruction::RefVar { name, line, .. } => {
                if !self.is_at_current_scope(name, None, depth) {
                    return Err(ThrushError::Scope(
                        ThrushErrorKind::VariableNotDefined,
                        String::from("Undefined Variable"),
                        format!("The variable `{}` not found in this scope.", name),
                        *line,
                    ));
                }

                if self.is_at_current_scope(name, None, depth)
                    && !self.is_reacheable_at_current_scope(name, *line, None, depth)
                {
                    return Err(ThrushError::Scope(
                        ThrushErrorKind::UnreachableVariable,
                        String::from("Unreacheable Variable"),
                        format!(
                            "The variable `{}` is unreacheable to the current scope.",
                            name
                        ),
                        *line,
                    ));
                }

                Ok(())
            }

            Instruction::Println(params) => {
                params.iter().try_for_each(|instr| {
                    match self.analyze_instruction(instr, depth) {
                        Ok(()) => Ok(()),
                        Err(e) => Err(e),
                    }
                })?;

                Ok(())
            }

            Instruction::Print(params) => {
                params.iter().try_for_each(|instr| {
                    match self.analyze_instruction(instr, depth) {
                        Ok(()) => Ok(()),
                        Err(e) => Err(e),
                    }
                })?;

                Ok(())
            }

            Instruction::Block { stmts, .. } => {
                stmts.iter().try_for_each(|instr| {
                    match self.analyze_instruction(instr, depth) {
                        Ok(()) => Ok(()),
                        Err(e) => Err(e),
                    }
                })?;

                Ok(())
            }

            _ => Ok(()),
        }
    }

    fn is_reacheable_at_current_scope(
        &self,
        name: &str,
        refvar_line: usize,
        block: Option<&Instruction<'ctx>>,
        mut depth: usize,
    ) -> bool {
        if depth > self.blocks.len() {
            return false;
        }

        if block.is_some() {
            if let Instruction::Block { stmts, .. } = block.as_ref().unwrap() {
                return stmts.iter().rev().any(|instr| match instr {
                    Instruction::Var { name: n, line, .. } if *n == name => {
                        if *line > refvar_line {
                            return false;
                        }

                        true
                    }
                    Instruction::Block { .. } => {
                        self.is_reacheable_at_current_scope(name, refvar_line, Some(instr), depth)
                    }
                    _ => {
                        depth += 1;
                        self.is_reacheable_at_current_scope(name, refvar_line, None, depth)
                    }
                });
            }
        }

        if self.blocks.len() == 1 || depth == 0 {
            self.blocks[0].stmts.iter().rev().any(|instr| match instr {
                Instruction::Var { name: n, line, .. } if **n == *name => {
                    if *line > refvar_line {
                        return false;
                    }

                    true
                }
                Instruction::Block { .. } => {
                    self.is_reacheable_at_current_scope(name, refvar_line, Some(instr), depth)
                }
                _ => {
                    depth += 1;
                    self.is_reacheable_at_current_scope(name, refvar_line, None, depth)
                }
            })
        } else {
            self.blocks[depth - 1]
                .stmts
                .iter()
                .rev()
                .any(|instr| match instr {
                    Instruction::Var { name: n, line, .. } if **n == *name => {
                        if *line > refvar_line {
                            return false;
                        }

                        true
                    }
                    Instruction::Block { .. } => {
                        self.is_reacheable_at_current_scope(name, refvar_line, Some(instr), depth)
                    }
                    _ => {
                        depth += 1;
                        self.is_reacheable_at_current_scope(name, refvar_line, None, depth)
                    }
                })
        }
    }

    fn is_at_current_scope(
        &self,
        name: &str,
        block: Option<&Instruction<'ctx>>,
        mut depth: usize,
    ) -> bool {
        if depth > self.blocks.len() {
            return false;
        }

        if block.is_some() {
            if let Instruction::Block { stmts, .. } = block.as_ref().unwrap() {
                return stmts.iter().rev().any(|instr| match instr {
                    Instruction::Var { name: n, .. } if *n == name => true,
                    Instruction::Block { .. } => self.is_at_current_scope(name, Some(instr), depth),
                    _ => {
                        depth += 1;
                        self.is_at_current_scope(name, None, depth)
                    }
                });
            }
        }

        if self.blocks.len() == 1 || depth == 0 {
            self.blocks[0].stmts.iter().rev().any(|instr| match &instr {
                Instruction::Var { name: n, .. } => *n == name,
                Instruction::Block { .. } => self.is_at_current_scope(name, Some(instr), depth),
                _ => {
                    depth += 1;
                    self.is_at_current_scope(name, None, depth)
                }
            })
        } else {
            self.blocks[depth - 1]
                .stmts
                .iter()
                .rev()
                .any(|instr| match &instr {
                    Instruction::Var { name: n, .. } => *n == name,
                    Instruction::Block { .. } => self.is_at_current_scope(name, Some(instr), depth),
                    _ => {
                        depth += 1;
                        self.is_at_current_scope(name, None, depth)
                    }
                })
        }
    }
}
