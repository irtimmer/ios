use alloc::boxed::Box;

use bitflags::bitflags;

use super::PageTable;

bitflags! {
    pub struct MemoryFlags: u8 {
        const WRITABLE = 1 << 0;
        const EXECUTABLE = 2 << 0;
    }
}

#[derive(Debug)]
pub enum MemoryMapError {
    InvalidAlignment(u64),
    AlreadyMapped(u64, u64)
}

pub trait System {
    fn sleep();
    fn request_irq_handler(&self, handler: Box<dyn Fn()>) -> Option<u8>;
    unsafe fn map(&self, from: usize, to: usize, length: usize, flags: MemoryFlags) -> Result<(), MemoryMapError>;
    fn memory_barrier();
    fn new_user_page_table(&self) -> PageTable;
}

pub trait PageMapper {
    unsafe fn map(&mut self, from: usize, to: usize, length: usize, map_flags: MemoryFlags) -> Result<(), MemoryMapError>;
    unsafe fn activate(&self);
}
