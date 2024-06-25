use super::{MemoryOperations, NonMbcOperations};

pub struct InputOutput {
    memory: Vec<u8>,
}

impl InputOutput {
    pub fn new(size: usize) -> Self {
        Self {
            memory: vec![0; size],
        }
    }
}

impl MemoryOperations for InputOutput {
    fn read_byte(&self, address: u16) -> u8 {
        self.memory[address as usize]
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        self.memory[address as usize] = value;
    }
}

impl NonMbcOperations for InputOutput {
    fn fill_from_slice(&mut self, data: &[u8]) {
        self.memory.copy_from_slice(data);
    }
}