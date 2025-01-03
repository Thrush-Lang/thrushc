use {
    ahash::AHashMap as HashMap,
    inkwell::values::{FunctionValue, PointerValue},
};

#[derive(Debug)]
pub struct CompilerObjects<'ctx> {
    pub functions: HashMap<&'ctx str, FunctionValue<'ctx>>,
    pub blocks: Vec<HashMap<String, PointerValue<'ctx>>>,
    pub scope: usize,
}

impl<'ctx> CompilerObjects<'ctx> {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            blocks: Vec::new(),
            scope: 0,
        }
    }

    #[inline]
    pub fn push(&mut self) {
        self.blocks.push(HashMap::new());
        self.scope += 1;
    }

    #[inline]
    pub fn pop(&mut self) {
        self.blocks.pop();
        self.scope -= 1;
    }

    #[inline]
    pub fn insert(&mut self, name: String, value: PointerValue<'ctx>) {
        self.blocks[self.scope - 1].insert(name, value);
    }

    #[inline]
    pub fn insert_function(&mut self, name: &'ctx str, function: FunctionValue<'ctx>) {
        self.functions.insert(name, function);
    }

    /*  #[inline]
        pub fn get_in_current(&self, name: &str) -> Option<&PointerValue<'ctx>> {
            self.blocks[self.scope - 1].get(name)
        }

        #[inline]
        pub fn contains_in_current(&self, name: &str) -> bool {
            self.blocks[self.scope - 1].contains_key(name)
        }

        #[inline]
        pub fn contains_in_block(&self, position: usize, name: &str) -> bool {
            self.blocks[position].contains_key(name)
        }

        #[inline]
        pub fn get_in_block(&self, position: usize, name: &str) -> Option<&PointerValue<'ctx>> {
            self.blocks[position].get(name)
        }
    */

    #[inline]
    pub fn find_and_get(&self, name: &str) -> Option<PointerValue<'ctx>> {
        for position in (0..self.scope).rev() {
            if self.blocks[position].contains_key(name) {
                return Some(*self.blocks[position].get(name).unwrap());
            }
        }

        None
    }

    #[inline]
    pub fn find_and_get_function(&self, name: &str) -> Option<FunctionValue<'ctx>> {
        if self.functions.contains_key(name) {
            return Some(*self.functions.get(name).unwrap());
        }

        None
    }
}
