use std::vec;

use inkwell::{
    basic_block::BasicBlock,
    builder::Builder,
    context::Context,
    module::{Linkage, Module},
    types::{FunctionType, StructType},
    values::{FunctionValue, IntValue, PointerValue},
    AddressSpace, IntPredicate,
};

/* -----------------------------------------------------------------------

 Vector Backend Infraestructure

 Functions:

    - {ptr, i64, i64} @Vec.init(i64)
    - void @Vec.push(i64)

----------------------------------------------------------------------- */

pub struct VectorInfraestructure<'a, 'ctx> {
    module: &'a Module<'ctx>,
    builder: &'a Builder<'ctx>,
    context: &'ctx Context,
    vector_type: StructType<'ctx>,
}

impl<'a, 'ctx> VectorInfraestructure<'a, 'ctx> {
    pub fn new(
        module: &'a Module<'ctx>,
        builder: &'a Builder<'ctx>,
        context: &'ctx Context,
    ) -> Self {
        Self {
            module,
            builder,
            context,
            vector_type: context.struct_type(
                &[
                    context.i64_type().into(),                        // size
                    context.i64_type().into(),                        // capacity
                    context.i64_type().into(),                        // element_size
                    context.ptr_type(AddressSpace::default()).into(), // data
                ],
                false,
            ),
        }
    }

    pub fn define(&mut self) {
        self.needed_functions();
        self.init();
        self.should_grow();
        self.size_at_bytes();
        self.realloc();
        self.adjust_capacity();
        self.vector_offset();
        self.vector_assign();
        self.push_back();
        self.destroy();
    }

    fn init(&mut self) {
        let vector_init_type: FunctionType<'_> = self.context.void_type().fn_type(
            &[
                self.context.ptr_type(AddressSpace::default()).into(),
                self.context.i64_type().into(),
                self.context.i64_type().into(),
            ],
            true,
        );

        let vector_init: FunctionValue<'_> =
            self.module
                .add_function("Vec.init", vector_init_type, Some(Linkage::LinkerPrivate));

        let vector_init_block: BasicBlock<'_> = self.context.append_basic_block(vector_init, "");

        self.builder.position_at_end(vector_init_block);

        let alloca_capacity: PointerValue<'ctx> = self
            .builder
            .build_alloca(self.context.i64_type(), "")
            .unwrap();

        let alloca_element_size: PointerValue<'ctx> = self
            .builder
            .build_alloca(self.context.i64_type(), "")
            .unwrap();

        let alloca_vector: PointerValue<'ctx> = self
            .builder
            .build_alloca(self.context.ptr_type(AddressSpace::default()), "")
            .unwrap();

        self.builder
            .build_store(
                alloca_capacity,
                vector_init.get_nth_param(1).unwrap().into_int_value(),
            )
            .unwrap();

        self.builder
            .build_store(
                alloca_element_size,
                vector_init.get_last_param().unwrap().into_int_value(),
            )
            .unwrap();

        self.builder
            .build_store(
                alloca_vector,
                vector_init.get_first_param().unwrap().into_pointer_value(),
            )
            .unwrap();

        let vector: PointerValue<'_> = self
            .builder
            .build_load(alloca_vector.get_type(), alloca_vector, "")
            .unwrap()
            .into_pointer_value();

        let size: PointerValue<'ctx> = self
            .builder
            .build_struct_gep(self.vector_type, vector, 0, "")
            .unwrap();

        self.builder
            .build_store(size, self.context.i64_type().const_zero())
            .unwrap();

        let capacity: IntValue<'ctx> = self
            .builder
            .build_load(self.context.i64_type(), alloca_capacity, "")
            .unwrap()
            .into_int_value();

        let idiomatic_capacity = self
            .builder
            .build_call(
                self.module.get_function("llvm.umax.i64").unwrap(),
                &[
                    self.context.i64_type().const_int(2, false).into(),
                    capacity.into(),
                ],
                "",
            )
            .unwrap()
            .try_as_basic_value()
            .unwrap_left()
            .into_int_value();

        let vector_1 = self
            .builder
            .build_load(alloca_vector.get_type(), alloca_vector, "")
            .unwrap()
            .into_pointer_value();

        let capacity: PointerValue<'ctx> = self
            .builder
            .build_struct_gep(self.vector_type, vector_1, 1, "")
            .unwrap();

        self.builder
            .build_store(capacity, idiomatic_capacity)
            .unwrap();

        let element_size: IntValue<'_> = self
            .builder
            .build_load(self.context.i64_type(), alloca_element_size, "")
            .unwrap()
            .into_int_value();

        let vector_2: PointerValue<'_> = self
            .builder
            .build_load(alloca_vector.get_type(), alloca_vector, "")
            .unwrap()
            .into_pointer_value();

        let element_size_2: PointerValue<'_> = self
            .builder
            .build_struct_gep(self.vector_type, vector_2, 2, "")
            .unwrap();

        self.builder
            .build_store(element_size_2, element_size)
            .unwrap();

        let capacity: IntValue<'_> = self
            .builder
            .build_load(self.context.i64_type(), alloca_capacity, "")
            .unwrap()
            .into_int_value();

        let element_size_3: IntValue<'_> = self
            .builder
            .build_load(self.context.i64_type(), alloca_element_size, "")
            .unwrap()
            .into_int_value();

        let malloc_size: IntValue<'ctx> = self
            .builder
            .build_int_mul(capacity, element_size_3, "")
            .unwrap();

        let data_allocated: PointerValue<'_> = self
            .builder
            .build_call(
                self.module.get_function("malloc").unwrap(),
                &[malloc_size.into()],
                "",
            )
            .unwrap()
            .try_as_basic_value()
            .unwrap_left()
            .into_pointer_value();

        let vector_4: PointerValue<'_> = self
            .builder
            .build_load(alloca_vector.get_type(), alloca_vector, "")
            .unwrap()
            .into_pointer_value();

        let data: PointerValue<'_> = self
            .builder
            .build_struct_gep(self.vector_type, vector_4, 3, "")
            .unwrap();

        self.builder.build_store(data, data_allocated).unwrap();

        self.builder.build_return(None).unwrap();
    }

    fn should_grow(&mut self) {
        let should_grow: FunctionValue<'_> = self.module.add_function(
            "_Vec.should_grow",
            self.context.bool_type().fn_type(
                &[self.context.ptr_type(AddressSpace::default()).into()],
                true,
            ),
            Some(Linkage::LinkerPrivate),
        );

        let should_grow_block: BasicBlock<'_> = self.context.append_basic_block(should_grow, "");

        self.builder.position_at_end(should_grow_block);

        let alloc_vector: PointerValue<'_> = self
            .builder
            .build_alloca(self.context.ptr_type(AddressSpace::default()), "")
            .unwrap();

        self.builder
            .build_store(alloc_vector, should_grow.get_first_param().unwrap())
            .unwrap();

        let vector: PointerValue<'_> = self
            .builder
            .build_load(alloc_vector.get_type(), alloc_vector, "")
            .unwrap()
            .into_pointer_value();

        let size_get: PointerValue<'_> = self
            .builder
            .build_struct_gep(self.vector_type, vector, 0, "")
            .unwrap();

        let size: IntValue<'_> = self
            .builder
            .build_load(self.context.i64_type(), size_get, "")
            .unwrap()
            .into_int_value();

        let vector_2: PointerValue<'_> = self
            .builder
            .build_load(alloc_vector.get_type(), alloc_vector, "")
            .unwrap()
            .into_pointer_value();

        let capacity_get: PointerValue<'_> = self
            .builder
            .build_struct_gep(self.vector_type, vector_2, 2, "")
            .unwrap();

        let capacity: IntValue<'_> = self
            .builder
            .build_load(self.context.i64_type(), capacity_get, "")
            .unwrap()
            .into_int_value();

        let cmp = self
            .builder
            .build_int_compare(IntPredicate::EQ, size, capacity, "")
            .unwrap();

        self.builder.build_return(Some(&cmp)).unwrap();
    }

    fn adjust_capacity(&mut self) {
        let adjust_capacity: FunctionValue<'_> = self.module.add_function(
            "_Vec.adjust_capacity",
            self.context.void_type().fn_type(
                &[self.context.ptr_type(AddressSpace::default()).into()],
                true,
            ),
            Some(Linkage::LinkerPrivate),
        );

        let adjust_capacity_block: BasicBlock<'_> =
            self.context.append_basic_block(adjust_capacity, "");

        self.builder.position_at_end(adjust_capacity_block);

        let alloca_vector: PointerValue<'_> = self
            .builder
            .build_alloca(self.context.ptr_type(AddressSpace::default()), "")
            .unwrap();

        self.builder
            .build_store(alloca_vector, adjust_capacity.get_first_param().unwrap())
            .unwrap();

        let vector: PointerValue<'_> = self
            .builder
            .build_load(alloca_vector.get_type(), alloca_vector, "")
            .unwrap()
            .into_pointer_value();

        let size_get: PointerValue<'_> = self
            .builder
            .build_struct_gep(self.vector_type, vector, 0, "")
            .unwrap();

        let size: IntValue<'_> = self
            .builder
            .build_load(self.context.i64_type(), size_get, "")
            .unwrap()
            .into_int_value();

        let grow: IntValue<'ctx> = self
            .builder
            .build_int_mul(size, self.context.i64_type().const_int(2, false), "")
            .unwrap();

        let vector_2: PointerValue<'_> = self
            .builder
            .build_load(alloca_vector.get_type(), alloca_vector, "")
            .unwrap()
            .into_pointer_value();

        let size_get_2: PointerValue<'_> = self
            .builder
            .build_struct_gep(self.vector_type, vector_2, 0, "")
            .unwrap();

        let size_2: IntValue<'_> = self
            .builder
            .build_load(self.context.i64_type(), size_get_2, "")
            .unwrap()
            .into_int_value();

        let new_capacity: IntValue<'_> = self
            .builder
            .build_call(
                self.module.get_function("llvm.umax.i64").unwrap(),
                &[size_2.into(), grow.into()],
                "",
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        let vector_3: PointerValue<'_> = self
            .builder
            .build_load(alloca_vector.get_type(), alloca_vector, "")
            .unwrap()
            .into_pointer_value();

        self.builder
            .build_call(
                self.module.get_function("Vec.realloc").unwrap(),
                &[vector_3.into(), new_capacity.into()],
                "",
            )
            .unwrap();

        self.builder.build_return(None).unwrap();
    }

    fn size_at_bytes(&mut self) {
        let size_at_bytes: FunctionValue<'_> = self.module.add_function(
            "_Vec.size_at_bytes",
            self.context.i64_type().fn_type(
                &[self.context.ptr_type(AddressSpace::default()).into()],
                true,
            ),
            Some(Linkage::LinkerPrivate),
        );

        let size_at_bytes_block: BasicBlock<'_> =
            self.context.append_basic_block(size_at_bytes, "");

        self.builder.position_at_end(size_at_bytes_block);

        let alloca_vector: PointerValue<'_> = self
            .builder
            .build_alloca(self.context.ptr_type(AddressSpace::default()), "")
            .unwrap();

        self.builder
            .build_store(alloca_vector, size_at_bytes.get_first_param().unwrap())
            .unwrap();

        let vector: PointerValue<'_> = self
            .builder
            .build_load(alloca_vector.get_type(), alloca_vector, "")
            .unwrap()
            .into_pointer_value();

        let size_get: PointerValue<'_> = self
            .builder
            .build_struct_gep(self.vector_type, vector, 0, "")
            .unwrap();

        let vector_2: PointerValue<'_> = self
            .builder
            .build_load(alloca_vector.get_type(), alloca_vector, "")
            .unwrap()
            .into_pointer_value();

        let element_size_get = self
            .builder
            .build_struct_gep(self.vector_type, vector_2, 2, "")
            .unwrap();

        let size: IntValue<'_> = self
            .builder
            .build_load(self.context.i64_type(), size_get, "")
            .unwrap()
            .into_int_value();

        let element_size: IntValue<'_> = self
            .builder
            .build_load(self.context.i64_type(), element_size_get, "")
            .unwrap()
            .into_int_value();

        let size_in_bytes: IntValue<'_> =
            self.builder.build_int_mul(size, element_size, "").unwrap();

        self.builder.build_return(Some(&size_in_bytes)).unwrap();
    }

    fn realloc(&mut self) {
        let realloc: FunctionValue<'_> = self.module.add_function(
            "Vec.realloc",
            self.context.void_type().fn_type(
                &[
                    self.context.ptr_type(AddressSpace::default()).into(),
                    self.context.i64_type().into(),
                ],
                true,
            ),
            Some(Linkage::LinkerPrivate),
        );

        let realloc_block: BasicBlock<'_> = self.context.append_basic_block(realloc, "");

        self.builder.position_at_end(realloc_block);

        let alloca_vector: PointerValue<'_> = self
            .builder
            .build_alloca(self.context.ptr_type(AddressSpace::default()), "")
            .unwrap();

        let alloca_new_capacity: PointerValue<'ctx> = self
            .builder
            .build_alloca(self.context.i64_type(), "")
            .unwrap();

        self.builder
            .build_store(alloca_vector, realloc.get_first_param().unwrap())
            .unwrap();

        self.builder
            .build_store(alloca_new_capacity, realloc.get_last_param().unwrap())
            .unwrap();

        let vector: PointerValue<'_> = self
            .builder
            .build_load(alloca_vector.get_type(), alloca_vector, "")
            .unwrap()
            .into_pointer_value();

        let get_capacity: PointerValue<'ctx> = self
            .builder
            .build_struct_gep(self.vector_type, vector, 1, "")
            .unwrap();

        let capacity: IntValue<'_> = self
            .builder
            .build_load(self.context.i64_type(), get_capacity, "")
            .unwrap()
            .into_int_value();

        let new_capacity: IntValue<'_> = self
            .builder
            .build_load(self.context.i64_type(), alloca_new_capacity, "")
            .unwrap()
            .into_int_value();

        let new_capacity_in_bytes: PointerValue<'ctx> = self
            .builder
            .build_alloca(self.context.i64_type(), "")
            .unwrap();

        let old_data: PointerValue<'ctx> = self
            .builder
            .build_alloca(self.context.ptr_type(AddressSpace::default()), "")
            .unwrap();

        let cmp_capacity = self
            .builder
            .build_int_compare(
                IntPredicate::ULT,
                new_capacity,
                self.context.i64_type().const_int(2, false),
                "",
            )
            .unwrap();

        let is_capacity_true: BasicBlock<'_> = self.context.append_basic_block(realloc, "");
        let is_capacity_false: BasicBlock<'_> = self.context.append_basic_block(realloc, "");

        self.builder
            .build_conditional_branch(cmp_capacity, is_capacity_true, is_capacity_false)
            .unwrap();

        self.builder.position_at_end(is_capacity_true);

        let cmp_capacity_2: IntValue<'ctx> = self
            .builder
            .build_int_compare(
                IntPredicate::UGT,
                capacity,
                self.context.i64_type().const_int(2, false),
                "",
            )
            .unwrap();

        let is_capacity_true_2: BasicBlock<'_> = self.context.append_basic_block(realloc, "");
        let is_capacity_false_2: BasicBlock<'_> = self.context.append_basic_block(realloc, "");

        self.builder
            .build_conditional_branch(cmp_capacity_2, is_capacity_true_2, is_capacity_false_2)
            .unwrap();

        self.builder.position_at_end(is_capacity_true_2);

        self.builder
            .build_store(
                alloca_new_capacity,
                self.context.i64_type().const_int(2, false),
            )
            .unwrap();

        self.builder
            .build_unconditional_branch(is_capacity_false)
            .unwrap();

        self.builder.position_at_end(is_capacity_false_2);
        self.builder.build_return(None).unwrap();

        self.builder.position_at_end(is_capacity_false);

        let vector_2 = self
            .builder
            .build_load(alloca_vector.get_type(), alloca_vector, "")
            .unwrap()
            .into_pointer_value();

        let get_element_size = self
            .builder
            .build_struct_gep(self.vector_type, vector_2, 2, "")
            .unwrap();

        let element_size = self
            .builder
            .build_load(self.context.i64_type(), get_element_size, "")
            .unwrap()
            .into_int_value();

        let get_new_capacity_2: IntValue<'_> = self
            .builder
            .build_load(self.context.i64_type(), alloca_new_capacity, "")
            .unwrap()
            .into_int_value();

        let new_capacity_in_bytes_to_allocate = self
            .builder
            .build_int_mul(get_new_capacity_2, element_size, "")
            .unwrap();

        self.builder
            .build_store(new_capacity_in_bytes, new_capacity_in_bytes_to_allocate)
            .unwrap();

        let vector_3 = self
            .builder
            .build_load(alloca_vector.get_type(), alloca_vector, "")
            .unwrap()
            .into_pointer_value();

        let get_data = self
            .builder
            .build_struct_gep(self.vector_type, vector_3, 3, "")
            .unwrap();

        self.builder.build_store(old_data, get_data).unwrap();

        let vector_4 = self
            .builder
            .build_load(alloca_vector.get_type(), alloca_vector, "")
            .unwrap()
            .into_pointer_value();

        let get_data_2 = self
            .builder
            .build_struct_gep(self.vector_type, vector_4, 3, "")
            .unwrap();

        let load_new_capacity_in_bytes: IntValue<'_> = self
            .builder
            .build_load(self.context.i64_type(), new_capacity_in_bytes, "")
            .unwrap()
            .into_int_value();

        let new_data = self
            .builder
            .build_call(
                self.module.get_function("malloc").unwrap(),
                &[load_new_capacity_in_bytes.into()],
                "",
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        self.builder.build_store(get_data_2, new_data).unwrap();

        let vector_5 = self
            .builder
            .build_load(alloca_vector.get_type(), alloca_vector, "")
            .unwrap()
            .into_pointer_value();

        let get_data_3 = self
            .builder
            .build_struct_gep(self.vector_type, vector_5, 3, "")
            .unwrap();

        let get_data_4 = self
            .builder
            .build_load(get_data_3.get_type(), get_data_3, "")
            .unwrap()
            .into_pointer_value();

        let get_data_5 = self
            .builder
            .build_struct_gep(self.vector_type, vector_5, 3, "")
            .unwrap();

        let get_data_6 = self
            .builder
            .build_load(get_data_5.get_type(), get_data_5, "")
            .unwrap()
            .into_pointer_value();

        let size_at_bytes: IntValue<'_> = self
            .builder
            .build_call(
                self.module.get_function("_Vec.size_at_bytes").unwrap(),
                &[get_data_4.into()],
                "",
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        let vector_7 = self
            .builder
            .build_load(alloca_vector.get_type(), alloca_vector, "")
            .unwrap()
            .into_pointer_value();

        let get_capacity = self
            .builder
            .build_struct_gep(self.vector_type, vector_7, 1, "")
            .unwrap();

        self.builder
            .build_store(get_capacity, new_capacity)
            .unwrap();

        self.builder
            .build_call(
                self.module.get_function("memcpy").unwrap(),
                &[get_data_6.into(), old_data.into(), size_at_bytes.into()],
                "",
            )
            .unwrap();

        self.builder.build_return(None).unwrap();
    }

    fn push_back(&mut self) {
        let push_back: FunctionValue<'_> = self.module.add_function(
            "Vec.push_back",
            self.context.void_type().fn_type(
                &[
                    self.context.ptr_type(AddressSpace::default()).into(),
                    self.context.ptr_type(AddressSpace::default()).into(),
                ],
                true,
            ),
            Some(Linkage::LinkerPrivate),
        );

        let block_push_back: BasicBlock<'_> = self.context.append_basic_block(push_back, "");

        self.builder.position_at_end(block_push_back);

        let alloca_vector: PointerValue<'ctx> = self
            .builder
            .build_alloca(self.context.ptr_type(AddressSpace::default()), "")
            .unwrap();

        let alloca_element = self
            .builder
            .build_alloca(self.context.ptr_type(AddressSpace::default()), "")
            .unwrap();

        self.builder
            .build_store(
                alloca_vector,
                push_back.get_first_param().unwrap().into_pointer_value(),
            )
            .unwrap();

        self.builder
            .build_store(
                alloca_element,
                push_back.get_last_param().unwrap().into_pointer_value(),
            )
            .unwrap();

        let vector = self
            .builder
            .build_load(alloca_vector.get_type(), alloca_vector, "")
            .unwrap()
            .into_pointer_value();

        let should_grow: IntValue<'_> = self
            .builder
            .build_call(
                self.module.get_function("_Vec.should_grow").unwrap(),
                &[vector.into()],
                "",
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        let cmp = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                should_grow,
                self.context.bool_type().const_int(1, false),
                "",
            )
            .unwrap();

        let then_block: BasicBlock<'_> = self.context.append_basic_block(push_back, "");
        let else_block: BasicBlock<'_> = self.context.append_basic_block(push_back, "");

        self.builder
            .build_conditional_branch(cmp, then_block, else_block)
            .unwrap();

        self.builder.position_at_end(then_block);

        let vector_2 = self
            .builder
            .build_load(alloca_vector.get_type(), alloca_vector, "")
            .unwrap()
            .into_pointer_value();

        self.builder
            .build_call(
                self.module.get_function("_Vec.adjust_capacity").unwrap(),
                &[vector_2.into()],
                "",
            )
            .unwrap();

        self.builder.build_unconditional_branch(else_block).unwrap();

        self.builder.position_at_end(else_block);

        let vector_3: PointerValue<'_> = self
            .builder
            .build_load(alloca_vector.get_type(), alloca_vector, "")
            .unwrap()
            .into_pointer_value();

        let get_size: PointerValue<'_> = self
            .builder
            .build_struct_gep(self.vector_type, vector_3, 0, "")
            .unwrap();

        let size: IntValue<'_> = self
            .builder
            .build_load(self.context.i64_type(), get_size, "")
            .unwrap()
            .into_int_value();

        let vector_4: PointerValue<'_> = self
            .builder
            .build_load(alloca_vector.get_type(), alloca_vector, "")
            .unwrap()
            .into_pointer_value();

        let element: PointerValue<'_> = self
            .builder
            .build_load(alloca_element.get_type(), alloca_element, "")
            .unwrap()
            .into_pointer_value();

        self.builder
            .build_call(
                self.module.get_function("_Vec.assign").unwrap(),
                &[vector_4.into(), size.into(), element.into()],
                "",
            )
            .unwrap();

        let vector_5: PointerValue<'_> = self
            .builder
            .build_load(alloca_vector.get_type(), alloca_vector, "")
            .unwrap()
            .into_pointer_value();

        let get_size: PointerValue<'_> = self
            .builder
            .build_struct_gep(self.vector_type, vector_5, 0, "")
            .unwrap();

        let size: IntValue<'_> = self
            .builder
            .build_load(self.context.i64_type(), get_size, "")
            .unwrap()
            .into_int_value();

        let new_size: IntValue<'_> = self
            .builder
            .build_int_add(size, self.context.i64_type().const_int(1, false), "")
            .unwrap();

        let vector_6: PointerValue<'_> = self
            .builder
            .build_load(alloca_vector.get_type(), alloca_vector, "")
            .unwrap()
            .into_pointer_value();

        let get_size: PointerValue<'ctx> = self
            .builder
            .build_struct_gep(self.vector_type, vector_6, 0, "")
            .unwrap();

        self.builder.build_store(get_size, new_size).unwrap();

        self.builder.build_return(None).unwrap();
    }

    fn vector_offset(&mut self) {
        let vector_offset: FunctionValue<'_> = self.module.add_function(
            "_Vec.offset",
            self.context.ptr_type(AddressSpace::default()).fn_type(
                &[
                    self.context.ptr_type(AddressSpace::default()).into(),
                    self.context.i64_type().into(),
                ],
                true,
            ),
            Some(Linkage::LinkerPrivate),
        );

        let block_vector_offset: BasicBlock<'_> =
            self.context.append_basic_block(vector_offset, "");

        self.builder.position_at_end(block_vector_offset);

        let alloca_vector: PointerValue<'ctx> = self
            .builder
            .build_alloca(self.context.ptr_type(AddressSpace::default()), "")
            .unwrap();

        let alloca_offset = self
            .builder
            .build_alloca(self.context.i64_type(), "")
            .unwrap();

        self.builder
            .build_store(alloca_offset, vector_offset.get_last_param().unwrap())
            .unwrap();

        self.builder
            .build_store(alloca_vector, vector_offset.get_first_param().unwrap())
            .unwrap();

        let vector: PointerValue<'_> = self
            .builder
            .build_load(alloca_vector.get_type(), alloca_vector, "")
            .unwrap()
            .into_pointer_value();

        let get_data: PointerValue<'ctx> = self
            .builder
            .build_struct_gep(self.vector_type, vector, 3, "")
            .unwrap();

        let data = self
            .builder
            .build_load(get_data.get_type(), get_data, "")
            .unwrap()
            .into_pointer_value();

        let vector_2: PointerValue<'_> = self
            .builder
            .build_load(alloca_vector.get_type(), alloca_vector, "")
            .unwrap()
            .into_pointer_value();

        let get_element_size: PointerValue<'ctx> = self
            .builder
            .build_struct_gep(self.vector_type, vector_2, 2, "")
            .unwrap();

        let element_size: IntValue<'_> = self
            .builder
            .build_load(self.context.i64_type(), get_element_size, "")
            .unwrap()
            .into_int_value();

        let offset: IntValue<'_> = self
            .builder
            .build_load(self.context.i64_type(), alloca_offset, "")
            .unwrap()
            .into_int_value();

        let offset_calc: IntValue<'_> = self
            .builder
            .build_int_mul(offset, element_size, "")
            .unwrap();

        unsafe {
            let offset: PointerValue<'ctx> = self
                .builder
                .build_in_bounds_gep(self.context.i8_type(), data, &[offset_calc], "")
                .unwrap();

            self.builder.build_return(Some(&offset)).unwrap();
        }
    }

    fn vector_assign(&mut self) {
        let vector_assign: FunctionValue<'_> = self.module.add_function(
            "_Vec.assign",
            self.context.void_type().fn_type(
                &[
                    self.context.ptr_type(AddressSpace::default()).into(),
                    self.context.i64_type().into(),
                    self.context.ptr_type(AddressSpace::default()).into(),
                ],
                true,
            ),
            Some(Linkage::LinkerPrivate),
        );

        let block_vector_assign: BasicBlock<'_> =
            self.context.append_basic_block(vector_assign, "");

        self.builder.position_at_end(block_vector_assign);

        let alloca_vector: PointerValue<'ctx> = self
            .builder
            .build_alloca(self.context.ptr_type(AddressSpace::default()), "")
            .unwrap();

        let alloca_element: PointerValue<'ctx> = self
            .builder
            .build_alloca(self.context.ptr_type(AddressSpace::default()), "")
            .unwrap();

        let alloca_index = self
            .builder
            .build_alloca(self.context.i64_type(), "")
            .unwrap();

        let alloc_offset = self
            .builder
            .build_alloca(self.context.ptr_type(AddressSpace::default()), "")
            .unwrap();

        self.builder
            .build_store(alloca_vector, vector_assign.get_first_param().unwrap())
            .unwrap();

        self.builder
            .build_store(alloca_element, vector_assign.get_last_param().unwrap())
            .unwrap();

        self.builder
            .build_store(alloca_index, vector_assign.get_nth_param(1).unwrap())
            .unwrap();

        let vector: PointerValue<'_> = self
            .builder
            .build_load(alloca_vector.get_type(), alloca_vector, "")
            .unwrap()
            .into_pointer_value();

        let index = self
            .builder
            .build_load(self.context.i64_type(), alloca_index, "")
            .unwrap()
            .into_int_value();

        let offset: PointerValue<'_> = self
            .builder
            .build_call(
                self.module.get_function("_Vec.offset").unwrap(),
                &[vector.into(), index.into()],
                "",
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        self.builder.build_store(alloc_offset, offset).unwrap();

        let vector_2: PointerValue<'_> = self
            .builder
            .build_load(alloca_vector.get_type(), alloca_vector, "")
            .unwrap()
            .into_pointer_value();

        let get_element_size: PointerValue<'ctx> = self
            .builder
            .build_struct_gep(self.vector_type, vector_2, 2, "")
            .unwrap();

        let element_size: IntValue<'_> = self
            .builder
            .build_load(self.context.i64_type(), get_element_size, "")
            .unwrap()
            .into_int_value();

        let element = self
            .builder
            .build_load(alloca_element.get_type(), alloca_element, "")
            .unwrap()
            .into_pointer_value();

        let offset: PointerValue<'_> = self
            .builder
            .build_load(
                self.context.ptr_type(AddressSpace::default()),
                alloc_offset,
                "",
            )
            .unwrap()
            .into_pointer_value();

        self.builder
            .build_call(
                self.module.get_function("memcpy").unwrap(),
                &[offset.into(), element.into(), element_size.into()],
                "",
            )
            .unwrap();

        self.builder.build_return(None).unwrap();
    }

    fn destroy(&mut self) {
        let destroy: FunctionValue<'_> = self.module.add_function(
            "Vec.destroy",
            self.context.void_type().fn_type(
                &[self.context.ptr_type(AddressSpace::default()).into()],
                false,
            ),
            Some(Linkage::LinkerPrivate),
        );

        let block_destroy: BasicBlock<'_> = self.context.append_basic_block(destroy, "");

        self.builder.position_at_end(block_destroy);

        let vector: PointerValue<'_> = self
            .builder
            .build_alloca(self.context.ptr_type(AddressSpace::default()), "")
            .unwrap();

        self.builder
            .build_store(vector, destroy.get_first_param().unwrap())
            .unwrap();

        let vector_2: PointerValue<'_> = self
            .builder
            .build_load(vector.get_type(), vector, "")
            .unwrap()
            .into_pointer_value();

        let get_data = self
            .builder
            .build_struct_gep(self.vector_type, vector_2, 3, "")
            .unwrap();

        let data = self
            .builder
            .build_load(get_data.get_type(), get_data, "")
            .unwrap()
            .into_pointer_value();

        self.builder
            .build_call(
                self.module.get_function("free").unwrap(),
                &[data.into()],
                "",
            )
            .unwrap();

        let vector_3: PointerValue<'_> = self
            .builder
            .build_load(vector.get_type(), vector, "")
            .unwrap()
            .into_pointer_value();

        let get_data_2: PointerValue<'ctx> = self
            .builder
            .build_struct_gep(self.vector_type, vector_3, 3, "")
            .unwrap();

        self.builder
            .build_store(
                get_data_2,
                self.context.ptr_type(AddressSpace::default()).const_null(),
            )
            .unwrap();

        self.builder.build_return(None).unwrap();
    }

    fn needed_functions(&self) {
        self.module.add_function(
            "llvm.umax.i64",
            self.context.i64_type().fn_type(
                &[
                    self.context.i64_type().into(),
                    self.context.i64_type().into(),
                ],
                false,
            ),
            Some(Linkage::External),
        );

        self.module.add_function(
            "memcpy",
            self.context.ptr_type(AddressSpace::default()).fn_type(
                &[
                    self.context.ptr_type(AddressSpace::default()).into(),
                    self.context.ptr_type(AddressSpace::default()).into(),
                    self.context.i32_type().into(),
                ],
                false,
            ),
            Some(Linkage::External),
        );

        self.module.add_function(
            "free",
            self.context.ptr_type(AddressSpace::default()).fn_type(
                &[self.context.ptr_type(AddressSpace::default()).into()],
                false,
            ),
            Some(Linkage::External),
        );
    }
}

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

/* -----------------------------------------------------------------------

 Basic Infraestructure

 Functions:

    - ptr @malloc(i64)

----------------------------------------------------------------------- */

pub struct BasicInfraestructure<'a, 'ctx> {
    module: &'a Module<'ctx>,
    context: &'ctx Context,
}

impl<'a, 'ctx> BasicInfraestructure<'a, 'ctx> {
    pub fn new(module: &'a Module<'ctx>, context: &'ctx Context) -> Self {
        Self { module, context }
    }

    pub fn define(&mut self) {
        self.define_malloc();
    }

    fn define_malloc(&mut self) {
        self.module.add_function(
            "malloc",
            self.context
                .ptr_type(AddressSpace::default())
                .fn_type(&[self.context.i64_type().into()], false),
            Some(Linkage::External),
        );
    }
}
