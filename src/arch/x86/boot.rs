use acpi::{AcpiTables, PlatformInfo, InterruptModel};

use core::ffi::c_void;

use crate::drivers::video::fb::FrameBuffer;
use crate::runtime::Runtime;
use crate::main;

use super::acpi::IdentityMappedAcpiMemory;
use super::{gdt, interrupts, lapic, ioapic};

pub fn boot(acpi_table: Option<*const c_void>, fb: FrameBuffer) -> ! {
    Runtime::init(fb);

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

    main();
}
