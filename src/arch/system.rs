#[derive(Debug)]
pub enum MemoryMapError {
    InvalidAlignment(u64),
    AlreadyMapped(u64, u64)
}

pub trait System {
    fn sleep();
    unsafe fn map(&self, from: usize, to: usize, length: usize) -> Result<(), MemoryMapError>;
    fn memory_barrier();
}
