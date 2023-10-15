use core::ptr::NonNull;

use acpi::AcpiHandler;

#[derive(Clone, Default)]
pub struct IdentityMappedAcpiMemory;

// We use an identity memory mapping for X86s
impl AcpiHandler for IdentityMappedAcpiMemory {
    unsafe fn map_physical_region<T>(&self, physical_address: usize, size: usize) -> acpi::PhysicalMapping<Self, T> {
        acpi::PhysicalMapping::new(physical_address, NonNull::new((physical_address) as *mut _).unwrap(), size, size, self.clone())
    }

    fn unmap_physical_region<T>(_: &acpi::PhysicalMapping<Self, T>) { }
}
