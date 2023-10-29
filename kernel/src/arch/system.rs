use alloc::boxed::Box;

use bitflags::bitflags;

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
}
