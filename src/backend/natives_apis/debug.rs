use inkwell::{
    basic_block::BasicBlock,
    builder::Builder,
    context::Context,
    module::{Linkage, Module},
    values::{FunctionValue, PointerValue},
    AddressSpace,
};

pub struct DebugAPI<'a, 'ctx> {
    module: &'a Module<'ctx>,
    builder: &'a Builder<'ctx>,
    context: &'ctx Context,
}

impl<'a, 'ctx> DebugAPI<'a, 'ctx> {
    pub fn include(module: &'a Module<'ctx>, builder: &'a Builder<'ctx>, context: &'ctx Context) {
        Self {
            module,
            builder,
            context,
        }
        ._include();
    }

    pub fn define(module: &'a Module<'ctx>, builder: &'a Builder<'ctx>, context: &'ctx Context) {
        Self {
            module,
            builder,
            context,
        }
        ._define();
    }

    fn _include(&self) {
        self.needed_functions();
        self.panic();
    }

    fn panic(&self) {
        let panic: FunctionValue<'_> = self.module.add_function(
            "panic",
            self.context.void_type().fn_type(
                &[
                    self.context.ptr_type(AddressSpace::default()).into(),
                    self.context.ptr_type(AddressSpace::default()).into(),
                    self.context.ptr_type(AddressSpace::default()).into(),
                ],
                true,
            ),
            None,
        );

        let block_panic: BasicBlock<'_> = self.context.append_basic_block(panic, "");

        self.builder.position_at_end(block_panic);

        let stderr: PointerValue<'ctx> = self
            .builder
            .build_load(
                panic.get_first_param().unwrap().get_type(),
                panic.get_first_param().unwrap().into_pointer_value(),
                "",
            )
            .unwrap()
            .into_pointer_value();

        self.builder
            .build_call(
                self.module.get_function("fprintf").unwrap(),
                &[
                    stderr.into(),
                    panic.get_nth_param(1).unwrap().into_pointer_value().into(),
                    panic.get_last_param().unwrap().into_pointer_value().into(),
                ],
                "",
            )
            .unwrap();

        self.builder.build_unreachable().unwrap();
    }

    fn needed_functions(&self) {
        self.module.add_function(
            "fprintf",
            self.context.i32_type().fn_type(
                &[
                    self.context.ptr_type(AddressSpace::default()).into(),
                    self.context.ptr_type(AddressSpace::default()).into(),
                ],
                true,
            ),
            Some(Linkage::External),
        );
    }

    fn _define(&self) {
        self.module.add_function(
            "panic",
            self.context.void_type().fn_type(
                &[
                    self.context.ptr_type(AddressSpace::default()).into(),
                    self.context.ptr_type(AddressSpace::default()).into(),
                    self.context.ptr_type(AddressSpace::default()).into(),
                ],
                true,
            ),
            Some(Linkage::External),
        );
    }
}
