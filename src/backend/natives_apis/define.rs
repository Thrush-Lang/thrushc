use inkwell::{
    basic_block::BasicBlock,
    builder::Builder,
    context::Context,
    module::{Linkage, Module},
    values::{FunctionValue, PointerValue},
    AddressSpace,
};

/* -----------------------------------------------------------------------

 String Backend Infraestructure

 Functions:

    - void @String.append(ptr, char, i64)

----------------------------------------------------------------------- */

pub struct StringInfraestructure<'a, 'ctx> {
    module: &'a Module<'ctx>,
    builder: &'a Builder<'ctx>,
    context: &'ctx Context,
}

impl<'a, 'ctx> StringInfraestructure<'a, 'ctx> {
    pub fn new(
        module: &'a Module<'ctx>,
        builder: &'a Builder<'ctx>,
        context: &'ctx Context,
    ) -> Self {
        Self {
            module,
            builder,
            context,
        }
    }

    pub fn define(&mut self) {
        self.define_string_append();
    }

    fn define_string_append(&mut self) {
        let append: FunctionValue<'_> = self.module.add_function(
            "String.append",
            self.context.void_type().fn_type(
                &[
                    self.context.ptr_type(AddressSpace::default()).into(),
                    self.context.i8_type().into(),
                    self.context.i64_type().into(),
                ],
                true,
            ),
            Some(Linkage::LinkerPrivate),
        );

        let block_append: BasicBlock<'_> = self.context.append_basic_block(append, "");

        self.builder.position_at_end(block_append);

        let string: PointerValue<'_> = self
            .builder
            .build_load(
                append.get_first_param().unwrap().get_type(),
                append.get_first_param().unwrap().into_pointer_value(),
                "",
            )
            .unwrap()
            .into_pointer_value();

        unsafe {
            let ptr: PointerValue<'ctx> = self
                .builder
                .build_in_bounds_gep(
                    self.context.i8_type(),
                    string,
                    &[append.get_last_param().unwrap().into_int_value()],
                    "",
                )
                .unwrap();

            self.builder
                .build_store(ptr, append.get_nth_param(1).unwrap().into_int_value())
                .unwrap();
        }

        self.builder.build_return(None).unwrap();
    }
}
