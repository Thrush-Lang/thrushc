use {
    super::{
        super::super::frontend::lexer::DataTypes, codegen, locals::CompilerLocals, Instruction,
    },
    inkwell::{
        builder::Builder,
        context::Context,
        module::Module,
        values::{BasicMetadataValueEnum, BasicValueEnum},
    },
};

pub fn compile_call<'ctx>(
    module: &Module<'ctx>,
    builder: &Builder<'ctx>,
    context: &'ctx Context,
    name: &str,
    args: &'ctx [Instruction<'ctx>],
    kind: &DataTypes,
    locals: &CompilerLocals<'ctx>,
) -> Option<BasicValueEnum<'ctx>> {
    let mut compiled_args: Vec<BasicMetadataValueEnum> = Vec::with_capacity(args.len());

    args.iter().for_each(|arg| {
        compiled_args.push(
            codegen::compile_instr_as_basic_value_enum(
                module,
                builder,
                context,
                arg,
                &[],
                arg.is_var(),
                locals,
            )
            .into(),
        );
    });

    if kind != &DataTypes::Void {
        Some(
            builder
                .build_call(module.get_function(name).unwrap(), &compiled_args, "")
                .unwrap()
                .try_as_basic_value()
                .unwrap_left(),
        )
    } else {
        builder
            .build_call(module.get_function(name).unwrap(), &compiled_args, "")
            .unwrap();

        None
    }
}
