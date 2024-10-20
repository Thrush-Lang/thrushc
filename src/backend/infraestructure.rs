use inkwell::{
    basic_block::BasicBlock,
    builder::Builder,
    context::Context,
    module::{Linkage, Module},
    values::{BasicValueEnum, FunctionValue, IntValue, PointerValue},
    AddressSpace, IntPredicate,
};

/* -----------------------------------------------------------------------

 String Backend Infraestructure

 Functions:

    - void @String.append(ptr, char, i64)
    - char @String.extract(ptr, index)
    - i64 @String.size(ptr)

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
        self.define_string_extract();
        self.define_string_size();
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

        append.set_param_alignment(0, 4);
        append.set_param_alignment(1, 4);
        append.set_param_alignment(2, 4);

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

    fn define_string_extract(&mut self) {
        let string_extract: FunctionValue<'_> = self.module.add_function(
            "String.extract",
            self.context.i8_type().fn_type(
                &[
                    self.context.ptr_type(AddressSpace::default()).into(),
                    self.context.i64_type().into(),
                ],
                true,
            ),
            Some(Linkage::LinkerPrivate),
        );

        string_extract.set_param_alignment(0, 4);
        string_extract.set_param_alignment(1, 4);

        let block_string_extract: BasicBlock<'_> =
            self.context.append_basic_block(string_extract, "");

        self.builder.position_at_end(block_string_extract);

        let string: PointerValue<'_> = self
            .builder
            .build_load(
                string_extract.get_first_param().unwrap().get_type(),
                string_extract
                    .get_first_param()
                    .unwrap()
                    .into_pointer_value(),
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
                    &[string_extract.get_last_param().unwrap().into_int_value()],
                    "",
                )
                .unwrap();

            let cmp: IntValue<'ctx> = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    ptr,
                    self.context.ptr_type(AddressSpace::default()).const_null(),
                    "",
                )
                .unwrap();

            let is_true: BasicBlock<'_> = self.context.append_basic_block(string_extract, "");
            let is_false: BasicBlock<'_> = self.context.append_basic_block(string_extract, "");

            self.builder
                .build_conditional_branch(cmp, is_true, is_false)
                .unwrap();

            self.builder.position_at_end(is_true);
            self.builder
                .build_return(Some(&self.context.i8_type().const_int(0, false)))
                .unwrap();

            self.builder.position_at_end(is_false);

            let char: BasicValueEnum<'ctx> = self
                .builder
                .build_load(self.context.i8_type(), ptr, "")
                .unwrap();

            self.builder
                .build_return(Some(&char.into_int_value()))
                .unwrap();
        }
    }

    fn define_string_size(&mut self) {
        let string_size: FunctionValue<'_> = self.module.add_function(
            "String.size",
            self.context.i64_type().fn_type(
                &[self.context.ptr_type(AddressSpace::default()).into()],
                true,
            ),
            Some(Linkage::LinkerPrivate),
        );

        string_size.set_param_alignment(0, 4);

        let string_size_block: BasicBlock<'_> = self.context.append_basic_block(string_size, "");

        self.builder.position_at_end(string_size_block);

        let counter: PointerValue<'ctx> = self
            .builder
            .build_alloca(self.context.i64_type(), "")
            .unwrap();

        self.builder
            .build_store(counter, self.context.i64_type().const_int(1, false))
            .unwrap();

        let loop_block: BasicBlock<'_> = self.context.append_basic_block(string_size, "");

        self.builder.build_unconditional_branch(loop_block).unwrap();

        unsafe {
            self.builder.position_at_end(loop_block);

            let get_count: IntValue<'ctx> = self
                .builder
                .build_load(self.context.i64_type(), counter, "")
                .unwrap()
                .into_int_value();

            let index: IntValue<'ctx> = self
                .builder
                .build_int_sub(get_count, self.context.i64_type().const_int(1, false), "")
                .unwrap();

            let get_element: PointerValue<'ctx> = self
                .builder
                .build_in_bounds_gep(
                    string_size.get_first_param().unwrap().get_type(),
                    string_size.get_last_param().unwrap().into_pointer_value(),
                    &[index],
                    "",
                )
                .unwrap();

            let cmp: IntValue<'ctx> = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    get_element,
                    self.context.ptr_type(AddressSpace::default()).const_null(),
                    "",
                )
                .unwrap();

            let done: BasicBlock<'_> = self.context.append_basic_block(string_size, "");
            let not: BasicBlock<'_> = self.context.append_basic_block(string_size, "");

            self.builder
                .build_conditional_branch(cmp, done, not)
                .unwrap();

            self.builder.position_at_end(done);

            let size: IntValue<'_> = self
                .builder
                .build_load(self.context.i64_type(), counter, "")
                .unwrap()
                .into_int_value();

            self.builder.build_return(Some(&size)).unwrap();

            self.builder.position_at_end(not);

            let load_old_size: IntValue<'_> = self
                .builder
                .build_load(self.context.i64_type(), counter, "")
                .unwrap()
                .into_int_value();

            let counter_new_size: IntValue<'_> = self
                .builder
                .build_int_add(
                    load_old_size,
                    self.context.i64_type().const_int(1, false),
                    "",
                )
                .unwrap();

            self.builder.build_store(counter, counter_new_size).unwrap();

            self.builder.build_unconditional_branch(loop_block).unwrap();
        }
    }
}
