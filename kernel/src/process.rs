use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;

use core::arch::asm;

use spin::RwLock;

use crate::arch::{PageTable, KERNEL_ADDRESS_BASE, ThreadState};
use crate::arch::system::{System, PageMapper, MemoryFlags, ThreadContext};
use crate::runtime::runtime;

const PROCCESS_ADDR: usize = 0x900000000;
const STACK_ADDR: usize = 0x1000000000;

pub struct Process {
    page_table: Option<PageTable>
}

pub struct Thread {
    process: Arc<RwLock<Process>>,
    pub state: ThreadState,
    _stack: Vec<u8>
}

impl Process {
    pub fn empty() -> Self {
        Self {
            page_table: None
        }
    }

    pub fn new() -> Self {
        Self {
            page_table: Some(runtime().system.new_user_page_table())
        }
    }

    pub fn load(&mut self) {
        let address = (userspace_prog_1 as *const u8).wrapping_byte_sub(KERNEL_ADDRESS_BASE) as usize;
        unsafe { self.page_table.as_mut().unwrap().map(address, PROCCESS_ADDR, 4096, MemoryFlags::EXECUTABLE | MemoryFlags::USER).unwrap(); }
    }
}

impl Thread {
    pub fn new_current(process: Arc<RwLock<Process>>) -> Self {
        Self {
            process,
            state: ThreadState::running(),
            _stack: Vec::with_capacity(0)
        }
    }

    pub fn new(process: Arc<RwLock<Process>>) -> Self {
        let stack = vec![0; 1024 * 1024 * 10];
        let address = (userspace_prog_1 as *const u8).wrapping_byte_sub(KERNEL_ADDRESS_BASE) as usize;
        unsafe {
            let mut guard = process.write();
            guard.page_table.as_mut().unwrap().map(address, STACK_ADDR, 4096, MemoryFlags::WRITABLE | MemoryFlags::USER).unwrap();
        }

        let state = ThreadState::new(PROCCESS_ADDR as u64, STACK_ADDR as u64 + stack.len() as u64);

        Self {
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

#[naked]
#[repr(align(4096))]
unsafe extern "C" fn userspace_prog_1() {
    asm!("\
    2:
        inc rax
        nop
        nop
        jmp 2b
    ", options(noreturn));
}