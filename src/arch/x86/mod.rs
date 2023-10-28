use alloc::boxed::Box;

use spin::Mutex;

use x86_64::{instructions, registers::model_specific::KernelGsBase, VirtAddr};

use self::{paging::PageMapper, lapic::Interrupts};

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
    pub id: u32,
    pub interrupts: Interrupts
}

impl CpuData {
    pub fn new(id: u32) -> &'static Self {
        let data = Box::leak(Box::new(Self {
            id,
            interrupts: Interrupts {
                handlers: core::array::from_fn(|_| None)
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

    fn request_irq_handler(&self, handler: Box<dyn Fn()>) -> Option<u8> {
        let empty = CpuData::get().interrupts.handlers.iter_mut().enumerate().find(|(_, h)| h.is_none());
        empty.map(|(i, h)| {
            *h = Some(handler);
            (i+64) as u8
        })
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
