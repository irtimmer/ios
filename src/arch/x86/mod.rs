use spin::Mutex;

use x86_64::instructions;

use self::paging::PageMapper;

use super::system::{System, MemoryMapError};

pub mod acpi;
pub mod boot;
pub mod interrupts;
pub mod ioapic;
pub mod lapic;
pub mod paging;
pub mod pci;
pub mod gdt;

mod efi;

pub struct X86 {
    memory: Mutex<PageMapper>
}

impl System for X86 {
    fn sleep() {
        instructions::hlt();
    }

    unsafe fn map(&self, from: usize, to: usize, length: usize) -> Result<(), MemoryMapError> {
        self.memory.lock().map(from, to, length)?;
        Ok(())
    }
}
