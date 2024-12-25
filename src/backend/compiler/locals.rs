use {ahash::AHashMap as HashMap, inkwell::values::BasicValueEnum};

#[derive(Debug)]
pub struct CompilerLocals<'ctx> {
    pub blocks: Vec<HashMap<String, BasicValueEnum<'ctx>>>,
    pub scope: usize,
}

impl<'ctx> CompilerLocals<'ctx> {
    pub fn new() -> Self {
        Self {
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
    pub fn insert(&mut self, name: String, value: BasicValueEnum<'ctx>) {
        self.blocks[self.scope - 1].insert(name, value);
    }

    #[inline]
    pub fn get_in_current(&self, name: &str) -> Option<&BasicValueEnum<'ctx>> {
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
    pub fn get_in_block(&self, position: usize, name: &str) -> Option<&BasicValueEnum<'ctx>> {
        self.blocks[position].get(name)
    }

    #[inline]
    pub fn find_and_get(&self, name: &str) -> Option<BasicValueEnum<'ctx>> {
        for position in (0..self.scope).rev() {
            if self.blocks[position].contains_key(name) {
                return Some(*self.blocks[position].get(name).unwrap());
            }
        }

        None
    }
}
