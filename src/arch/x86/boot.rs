use acpi::{AcpiTables, PlatformInfo};

use core::ffi::c_void;
use core::fmt::Write;

use crate::drivers::video::fb::FrameBuffer;
use crate::runtime::{Runtime, runtime};
use crate::main;

use super::acpi::IdentityMappedAcpiMemory;
use super::{gdt, interrupts, lapic};

pub fn boot(acpi_table: Option<*const c_void>, fb: FrameBuffer) -> ! {
    Runtime::init(fb);

    gdt::init();
    interrupts::init();
    lapic::init();

    if let Some(acpi_table) = acpi_table {
        let acpi_table = unsafe { AcpiTables::from_rsdp(IdentityMappedAcpiMemory::default(), acpi_table as usize).unwrap() };
        let platform_info = PlatformInfo::new(&acpi_table).unwrap();
        writeln!(runtime().console.lock(), "ACPI Processors {:#?}", platform_info.processor_info).unwrap();
    }

    main();
}
