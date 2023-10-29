mod x86;

pub use x86::KERNEL_ADDRESS_BASE;

pub mod system;

pub type Arch = x86::X86;
pub type PciConfigRegion = x86::pci::PciConfigRegion;
pub type PageTable = x86::paging::PageTable;
