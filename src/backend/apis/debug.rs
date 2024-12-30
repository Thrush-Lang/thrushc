use {
    super::super::super::backend::{
        builder::{Clang, LLVMOptimizator},
        compiler::options::CompilerOptions,
    },
    inkwell::{
        basic_block::BasicBlock,
        builder::Builder,
        context::Context,
        module::{Linkage, Module},
        targets::{Target, TargetMachine},
        values::{FunctionValue, PointerValue},
        AddressSpace,
    },
    std::{
        fs,
        path::{Path, PathBuf},
    },
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

pub fn compile_debug_api(options: &mut CompilerOptions) {
    let debug_api_context: Context = Context::create();
    let debug_api_builder: Builder<'_> = debug_api_context.create_builder();
    let debug_api_module: Module<'_> = debug_api_context.create_module("debug.th");

    debug_api_module.set_triple(&options.target_triple);

    let machine: TargetMachine = Target::from_triple(&options.target_triple)
        .unwrap()
        .create_target_machine(
            &options.target_triple,
            "",
            "",
            options.optimization.to_llvm_opt(),
            options.reloc_mode,
            options.code_model,
        )
        .unwrap();

    debug_api_module.set_data_layout(&machine.get_target_data().get_data_layout());

    DebugAPI::include(&debug_api_module, &debug_api_builder, &debug_api_context);

    if !Path::new("output/").exists() {
        let _ = fs::create_dir_all("output/");
    }

    if !PathBuf::from("output/debug.o").exists() {
        debug_api_module.write_bitcode_to_path(Path::new("output/debug.bc"));

        LLVMOptimizator::optimize(
            "output/debug.bc",
            options.optimization.to_llvm_17_passes(),
            options.optimization.to_str(true, false),
        );

        let previous_library: bool = options.library;
        let previous_executable: bool = options.executable;
        let previous_static_library: bool = options.static_library;
        let previous_output: String = options.output.clone();

        options.library = true;
        options.static_library = false;
        options.executable = false;
        options.output = String::from("debug.o");

        Clang::new(&[PathBuf::from("output/debug.bc")], options).compile();

        options.library = previous_library;
        options.executable = previous_executable;
        options.static_library = previous_static_library;
        options.output = previous_output;

        let _ = fs::remove_file("output/debug.bc");
        let _ = fs::copy("debug.o", "output/debug.o");
        let _ = fs::remove_file("debug.o");
    }
}
