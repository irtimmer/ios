use acpi::{AcpiTables, PlatformInfo, InterruptModel};

use core::ops::DerefMut;

use bootloader::{BootInfo, MemoryType};

use x86_64::VirtAddr;

use spin::Mutex;

use crate::arch::system::{PageMapper, System, MemoryFlags};
use crate::drivers::i8042::PcKeyboard;
use crate::drivers::video::fb::FrameBuffer;
use crate::runtime::{Runtime, runtime};
use crate::{main, main_cpu, ALLOCATOR};

use super::acpi::IdentityMappedAcpiMemory;
use super::paging::PageTable;
use super::smp::{boot_cpu, setup_boot_code};
use super::{gdt, interrupts, lapic, ioapic, pci, X86, CpuData, KERNEL_ADDRESS_BASE, syscall};

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
                    unsafe { ALLOCATOR.lock().init(entry.phys_start as usize + KERNEL_ADDRESS_BASE, entry.page_count as usize * PAGE_SIZE) };
                }
            }
            _ => {}
        }
    }

    // Initialize new page table
    let mut page_mapper = PageTable::new(KERNEL_ADDRESS_BASE as u64);
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
        unsafe { page_mapper.map(entry.phys_start as usize, entry.phys_start as usize + KERNEL_ADDRESS_BASE, entry.page_count as usize * PAGE_SIZE, flags).unwrap(); }
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
        buffer: info.framebuffer.buffer.wrapping_byte_add(KERNEL_ADDRESS_BASE)
    };

    let fb_len = fb.height * fb.stride * (fb.bpp / 8);
    unsafe {
        system.map(info.framebuffer.buffer as usize, fb.buffer as usize, fb_len, MemoryFlags::WRITABLE).unwrap();
    }

    unsafe { system.memory.lock().activate(); }

    Runtime::init(system, fb, keyboard);
    let selectors = gdt::init();
    CpuData::new(0, selectors);

    interrupts::init();
    lapic::init();
    syscall::init(&CpuData::get().selectors);

    if let Some(acpi_table) = info.acpi_table {
        let acpi_table = unsafe { AcpiTables::from_rsdp(IdentityMappedAcpiMemory::default(), acpi_table as usize).unwrap() };
        let platform_info = PlatformInfo::new(&acpi_table).unwrap();

        setup_boot_code(runtime().system.memory.lock().deref_mut());
        for proc in platform_info.processor_info.unwrap().application_processors.iter() {
            boot_cpu(proc.processor_uid, proc.local_apic_id);
        }

        if let InterruptModel::Apic(model) = platform_info.interrupt_model {
            for ioapic in model.io_apics.iter() {
                let ioapic_address = ioapic.address as u64 + KERNEL_ADDRESS_BASE as u64;
                unsafe {
                    runtime().system.map(ioapic.address as usize, ioapic_address as usize, 4096, MemoryFlags::WRITABLE).unwrap();
                    x86_64::instructions::tlb::flush(VirtAddr::new(ioapic_address));
                }
                ioapic::init(ioapic_address, ioapic.id);
            }
        }
    }

    pci::init();

    main();
}

pub fn start_cpu(cpu_id: u32) -> ! {
    let selectors = gdt::init();
    CpuData::new(cpu_id, selectors);

    interrupts::init();
    lapic::init();
    syscall::init(&CpuData::get().selectors);

    main_cpu(cpu_id);
}
