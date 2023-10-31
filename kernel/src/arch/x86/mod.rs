use alloc::boxed::Box;

use spin::Mutex;

use x86_64::{instructions, registers::model_specific::KernelGsBase, VirtAddr};

use crate::arch::system::PageMapper;

use self::gdt::Selectors;
use self::lapic::Interrupts;
use self::paging::PageTable;

use super::system::{System, MemoryMapError, MemoryFlags};

pub mod acpi;
pub mod boot;
pub mod interrupts;
pub mod ioapic;
pub mod lapic;
pub mod paging;
pub mod pci;
pub mod gdt;
pub mod smp;
pub mod syscall;
pub mod threads;

pub const KERNEL_ADDRESS_BASE: usize = 0xffff800000000000;

pub struct CpuData {
    pub id: u32,
    pub interrupts: Interrupts,
    pub selectors: Selectors
}

impl CpuData {
    pub fn new(id: u32, selectors: Selectors) -> &'static Self {
        let data = Box::leak(Box::new(Self {
            id,
            selectors,
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
    memory: Mutex<PageTable>
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

    unsafe fn map(&self, from: usize, to: usize, length: usize, flags: MemoryFlags) -> Result<(), MemoryMapError> {
        self.memory.lock().map(from, to, length, flags)?;
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

    fn new_user_page_table(&self) -> super::PageTable {
        unsafe { self.memory.lock().clone() }
    }
}
