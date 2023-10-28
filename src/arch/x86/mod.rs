use alloc::boxed::Box;

use spin::Mutex;

use x86_64::{instructions, registers::model_specific::KernelGsBase, VirtAddr};

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

pub struct CpuData {
    pub id: u32
}

impl CpuData {
    pub fn new(id: u32) -> &'static Self {
        let data = Box::leak(Box::new(Self {
            id
            }
        }));

        KernelGsBase::write(VirtAddr::from_ptr(data as *const Self));
        data
    }

    #[inline(always)]
    pub fn get() -> &'static mut Self {
        let ptr = KernelGsBase::read();
        return unsafe { &mut *ptr.as_mut_ptr() };
    }
}

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

    /// Force strict CPU ordering, serializes load and store operations.
    #[allow(dead_code)]
    #[inline(always)]
    fn memory_barrier() {
        use core::arch::asm;
        unsafe {
            asm!("mfence", options(nostack, nomem, preserves_flags));
        }
    }
}
