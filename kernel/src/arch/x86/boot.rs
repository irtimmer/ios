use acpi::{AcpiTables, PlatformInfo, InterruptModel};

use bootloader::{BootInfo, MemoryType};

use x86_64::VirtAddr;

use spin::Mutex;

use crate::arch::system::{System, MemoryFlags};
use crate::drivers::i8042::PcKeyboard;
use crate::drivers::video::fb::FrameBuffer;
use crate::runtime::{Runtime, runtime};
use crate::{main, ALLOCATOR};

use super::acpi::IdentityMappedAcpiMemory;
use super::paging::PageMapper;
use super::{gdt, interrupts, lapic, ioapic, pci, X86, CpuData};

const PAGE_SIZE: usize = 4096;

#[no_mangle]
extern "C" fn _start(info: &BootInfo) -> ! {
    // Initialize allocator
    for entry in info.memory_map.entries() {
        match entry.ty {
            MemoryType::CONVENTIONAL
            | MemoryType::BOOT_SERVICES_CODE
            | MemoryType::BOOT_SERVICES_DATA => {
                if entry.page_count > 0x1000 {
                    unsafe { ALLOCATOR.lock().init(entry.phys_start as usize, entry.page_count as usize * PAGE_SIZE) };
                }
            }
            _ => {}
        }
    }

    // Initialize new page table
    let mut page_mapper = PageMapper::new(0);
    for entry in info.memory_map.entries() {
        let flags = match entry.ty {
            MemoryType::LOADER_CODE => MemoryFlags::WRITABLE | MemoryFlags::EXECUTABLE,
            MemoryType::RUNTIME_SERVICES_CODE => MemoryFlags::EXECUTABLE,
            MemoryType::CONVENTIONAL
            | MemoryType::BOOT_SERVICES_CODE
            | MemoryType::BOOT_SERVICES_DATA
            | MemoryType::RUNTIME_SERVICES_DATA
            | MemoryType::LOADER_DATA => MemoryFlags::WRITABLE,
            MemoryType::ACPI_NON_VOLATILE
            | MemoryType::ACPI_RECLAIM => MemoryFlags::empty(),
            _ => continue
        };
        unsafe { page_mapper.map(entry.phys_start as usize, entry.phys_start as usize, entry.page_count as usize * PAGE_SIZE, flags).unwrap(); }
    }

    let keyboard = PcKeyboard::new();
    keyboard.init();

    let system = X86 {
        memory: Mutex::new(page_mapper)
    };

    // Map memory of framebuffer
    let fb = FrameBuffer {
        width: info.framebuffer.width,
        height: info.framebuffer.height,
        stride: info.framebuffer.stride,
        bpp: info.framebuffer.bpp,
        buffer: info.framebuffer.buffer
    };

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

    if let Some(acpi_table) = info.acpi_table {
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