use acpi::{AcpiTables, PlatformInfo, InterruptModel};

use core::ffi::c_void;

use crate::drivers::i8042::PcKeyboard;
use crate::drivers::video::fb::FrameBuffer;
use crate::runtime::Runtime;
use crate::main;

use super::acpi::IdentityMappedAcpiMemory;
use super::{gdt, interrupts, lapic, ioapic, pci};

pub fn boot(acpi_table: Option<*const c_void>, fb: FrameBuffer) -> ! {
    let keyboard = PcKeyboard::new();
    keyboard.init();

    Runtime::init(fb, keyboard);

    gdt::init();
    interrupts::init();
    lapic::init();

    if let Some(acpi_table) = acpi_table {
        let acpi_table = unsafe { AcpiTables::from_rsdp(IdentityMappedAcpiMemory::default(), acpi_table as usize).unwrap() };
        let platform_info = PlatformInfo::new(&acpi_table).unwrap();

        if let InterruptModel::Apic(model) = platform_info.interrupt_model {
            for ioapic in model.io_apics.iter() {
                ioapic::init(ioapic.address as u64, ioapic.id);
            }
        }
    }

    pci::init();

    main();
}
