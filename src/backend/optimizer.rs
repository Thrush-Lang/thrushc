use {super::compiler::Instruction, inkwell::module::Module};

pub struct Optimizer<'a, 'ctx> {
    module: &'a Module<'ctx>,
    stmts: &'a [Instruction<'ctx>],
}

impl<'a, 'ctx> Optimizer<'a, 'ctx> {
    pub fn new(module: &'a Module<'ctx>, stmts: &'a [Instruction<'ctx>]) -> Self {
        Self { module, stmts }
    }

    pub fn optimize(&mut self) -> &'a Module<'ctx> {
        self.module.strip_debug_info();

        self.destroy_unused_standard_functions();

        self.module
    }

    fn destroy_unused_standard_functions(&mut self) {
        if !self
            .stmts
            .iter()
            .any(|stmt| matches!(stmt, Instruction::Print(_)))
        {
            unsafe {
                self.module.get_function("printf").unwrap().delete();
            }
        }
    }
}
