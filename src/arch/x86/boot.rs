use acpi::{AcpiTables, PlatformInfo, InterruptModel};
use x86_64::VirtAddr;

use core::ffi::c_void;

use spin::Mutex;

use crate::arch::system::{System, MemoryFlags};
use crate::drivers::i8042::PcKeyboard;
use crate::drivers::video::fb::FrameBuffer;
use crate::runtime::{Runtime, runtime};
use crate::main;

use super::acpi::IdentityMappedAcpiMemory;
use super::paging::PageMapper;
use super::{gdt, interrupts, lapic, ioapic, pci, X86, CpuData};

pub fn boot(page_mapper: PageMapper, acpi_table: Option<*const c_void>, fb: FrameBuffer) -> ! {
    let keyboard = PcKeyboard::new();
    keyboard.init();

    let system = X86 {
        memory: Mutex::new(page_mapper)
    };

    // Map memory of framebuffer
    let fb_len = fb.height * fb.stride * (fb.bpp / 8);
    unsafe {
        system.map(fb.buffer as usize, fb.buffer as usize, fb_len, MemoryFlags::WRITABLE).unwrap();
    }

    unsafe { system.memory.lock().activate(); }

    Runtime::init(system, fb, keyboard);
    CpuData::new(0);

    gdt::init();
    interrupts::init();
    lapic::init();

    if let Some(acpi_table) = acpi_table {
        let acpi_table = unsafe { AcpiTables::from_rsdp(IdentityMappedAcpiMemory::default(), acpi_table as usize).unwrap() };
        let platform_info = PlatformInfo::new(&acpi_table).unwrap();

        if let InterruptModel::Apic(model) = platform_info.interrupt_model {
            for ioapic in model.io_apics.iter() {
                unsafe {
                    runtime().system.map(ioapic.address as usize, ioapic.address as usize, 4096, MemoryFlags::WRITABLE).unwrap();
                    x86_64::instructions::tlb::flush(VirtAddr::new(ioapic.address as u64));
                }
                ioapic::init(ioapic.address as u64, ioapic.id);
            }
        }
    }

    pci::init();

    main();
}
