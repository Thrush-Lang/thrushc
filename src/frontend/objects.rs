use {
    super::{
        super::backend::instruction::Instruction,
        super::error::{ThrushError, ThrushErrorKind},
        lexer::DataTypes,
    },
    ahash::AHashMap as HashMap,
};

/*

    LOCALS OBJECTS

    (DataTypes, bool, bool, bool, usize)---------> Number the References
     ^^^^^^^|   ^^^|    |____   |_______
    Main Type - Is Null? - is_freeded - Free Only

    GLOBALS OBJECTS

    (DataTypes, Vec<DataTypes>, bool, bool)
     ^^^^^^^|   ^^^|^^^^^^^^^^  ^|^^   ^^^ -------
    Main Type - Param Types? - Is Function? - Ignore Params?

*/

type Locals<'instr> = Vec<HashMap<&'instr str, (DataTypes, bool, bool, bool, usize)>>;
type Globals<'instr> = HashMap<&'instr str, (DataTypes, Vec<DataTypes>, bool, bool)>;

type FoundObject = (
    DataTypes,      // Main Type
    bool,           // is null?
    bool,           // is freeded?
    bool,           // is function?
    bool,           // ignore the params if is a function?
    Vec<DataTypes>, // params types
    usize,          // Number the references
);

pub struct ParserObjects<'instr> {
    locals: Locals<'instr>,
    globals: Globals<'instr>,
}

impl<'instr> ParserObjects<'instr> {
    pub fn new() -> Self {
        Self {
            locals: vec![HashMap::new()],
            globals: HashMap::new(),
        }
    }

    pub fn get_object(
        &mut self,
        name: &'instr str,
        line: usize,
    ) -> Result<FoundObject, ThrushError> {
        for scope in self.locals.iter_mut().rev() {
            if scope.contains_key(name) {
                // DataTypes, bool <- (is_null), bool <- (is_freeded), usize <- (number of references)
                let mut var: (DataTypes, bool, bool, bool, usize) = *scope.get(name).unwrap();

                var.4 += 1; // <---------------------- Update Reference Counter (+1)
                scope.insert(name, var); // ------^^^^^^

                return Ok((var.0, var.1, var.2, false, false, Vec::new(), var.4));
            }
        }

        if self.globals.contains_key(name) {
            let global: &(DataTypes, Vec<DataTypes>, bool, bool) = self.globals.get(name).unwrap();

            let mut possible_params: Vec<DataTypes> = Vec::new();

            possible_params.clone_from(&global.1);

            // type, //is null, //is_function  //ignore_params  //params
            return Ok((
                global.0,
                false,
                false,
                global.2,
                global.3,
                possible_params,
                0,
            ));
        }

        Err(ThrushError::Parse(
            ThrushErrorKind::ObjectNotDefined,
            String::from("Object don't Found"),
            format!(
                "Object with name \"{}\" is don't in this scope or the global scope.",
                name
            ),
            line,
        ))
    }

    #[inline]
    pub fn begin_local_scope(&mut self) {
        self.locals.push(HashMap::new());
    }

    #[inline]
    pub fn end_local_scope(&mut self) {
        self.locals.pop();
    }

    pub fn insert_new_local(
        &mut self,
        scope_pos: usize,
        name: &'instr str,
        value: (DataTypes, bool, bool, bool, usize),
    ) {
        self.locals[scope_pos].insert(name, value);
    }

    pub fn insert_new_global(
        &mut self,
        name: &'instr str,
        value: (DataTypes, Vec<DataTypes>, bool, bool),
    ) {
        self.globals.insert(name, value);
    }

    #[inline]
    pub fn modify_deallocation(&mut self, name: &'instr str, free_only: bool, freeded: bool) {
        for scope in self.locals.iter_mut().rev() {
            if scope.contains_key(name) {
                // DataTypes, bool <- (is_null), bool <- (is_freeded), bool <- (free_only), usize <- (number of references)

                let mut local_object: (DataTypes, bool, bool, bool, usize) =
                    *scope.get(name).unwrap();

                local_object.1 = freeded;
                local_object.3 = free_only;

                scope.insert(name, local_object);

                return;
            }
        }
    }

    pub fn create_deallocators(&mut self, in_scope_pos: usize) -> Vec<Instruction<'instr>> {
        let mut frees: Vec<Instruction> = Vec::new();

        self.locals[in_scope_pos].iter_mut().for_each(|stmt| {
            if let (_, (DataTypes::String, false, false, free_only, 0)) = stmt {
                frees.push(Instruction::Free {
                    name: stmt.0,
                    is_string: true,
                    free_only: *free_only,
                });

                stmt.1 .2 = true;
            }
        });

        frees
    }

    pub fn decrease_local_references(&mut self) {
        self.locals.iter_mut().for_each(|scope| {
            scope.values_mut().for_each(|variable| {
                if variable.4 > 0 {
                    variable.4 -= 1;
                }
            });
        });
    }
}
