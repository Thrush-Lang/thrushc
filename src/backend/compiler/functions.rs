use {
    super::{
        super::super::frontend::lexer::DataTypes, codegen, objects::CompilerObjects, Instruction,
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
    objects: &CompilerObjects<'ctx>,
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
                objects,
            )
            .into(),
        );
    });

    if *kind != DataTypes::Void {
        Some(
            builder
                .build_call(
                    objects.find_and_get_function(name).unwrap(),
                    &compiled_args,
                    "",
                )
                .unwrap()
                .try_as_basic_value()
                .unwrap_left(),
        )
    } else {
        builder
            .build_call(
                objects.find_and_get_function(name).unwrap(),
                &compiled_args,
                "",
            )
            .unwrap();

        None
    }
}
