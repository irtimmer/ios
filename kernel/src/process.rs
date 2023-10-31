use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;

use xmas_elf::{ElfFile, program};

use spin::RwLock;

use crate::arch::{PageTable, KERNEL_ADDRESS_BASE, ThreadState};
use crate::arch::system::{System, PageMapper, MemoryFlags, ThreadContext};
use crate::runtime::runtime;

const PROCCESS_ADDR: usize = 0x900000000;
const STACK_ADDR: usize = 0x1000000000;

pub struct Process {
    entry_point: usize,
    page_table: Option<PageTable>
}

pub struct Thread {
    pub name: String,
    process: Arc<RwLock<Process>>,
    pub state: ThreadState,
    _stack: Vec<u8>
}

impl Process {
    pub fn empty() -> Self {
        Self {
            entry_point: 0,
            page_table: None
        }
    }

    pub fn new() -> Self {
        Self {
            entry_point: 0,
            page_table: Some(runtime().system.new_user_page_table())
        }
    }

    pub fn load(&mut self, data: &[u8]) {
        let elf = ElfFile::new(data).unwrap();
        self.entry_point = PROCCESS_ADDR + elf.header.pt2.entry_point() as usize;

        let size = elf.program_iter().filter(|x| x.get_type().unwrap() == program::Type::Load).map(|x| x.virtual_addr() + x.mem_size()).max().unwrap();
        let pages = (size / 4096 + 1) as usize;

        let mut code = vec![0u8; pages * 4096];
        for program in elf.program_iter() {
            match program.get_type().unwrap() {
                program::Type::Load => {
                    let segment_address = program.virtual_addr() as usize;
                    let segment_size = program.file_size() as usize;
                    let segment_offset = program.offset() as usize;
                    let segment = &mut code[segment_address..segment_address + segment_size];
                    segment.copy_from_slice(&data[segment_offset..segment_offset + segment_size]);
                },
                _ => {}
            }
        }

        let code = code.leak();
        let address = code.as_ptr().wrapping_byte_sub(KERNEL_ADDRESS_BASE) as usize;
        unsafe { self.page_table.as_mut().unwrap().map(address, PROCCESS_ADDR, pages * 4096, MemoryFlags::EXECUTABLE | MemoryFlags::WRITABLE | MemoryFlags::USER).unwrap(); }
    }
}

impl Thread {
    pub fn new_current(process: Arc<RwLock<Process>>) -> Self {
        Self {
            name: "kernel".to_string(),
            process,
            state: ThreadState::running(),
            _stack: Vec::with_capacity(0)
        }
    }

    pub fn new(process: Arc<RwLock<Process>>, name: &str) -> Self {
        let mut stack = vec![0; 1024 * 1024 * 10];
        let address = stack.as_mut_ptr().wrapping_byte_sub(KERNEL_ADDRESS_BASE) as usize;
        unsafe {
            let mut guard = process.write();
            guard.page_table.as_mut().unwrap().map(address, STACK_ADDR, stack.len(), MemoryFlags::WRITABLE | MemoryFlags::USER).unwrap();
        }

        let state = ThreadState::new(process.read().entry_point as u64, STACK_ADDR as u64 + stack.len() as u64);

        Self {
            name: name.to_string(),
            process,
            state,
            _stack: stack
        }
    }

    pub fn activate(&self) -> ThreadState {
        unsafe {
            if let Some(page_table) = &self.process.read().page_table {
                page_table.activate();
            }
        }
        self.state.clone()
    }
}
