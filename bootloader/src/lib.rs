#![no_std]

pub use uefi::table::boot::{MemoryMap, MemoryType};

#[repr(C)]
pub struct BootInfo {
    pub framebuffer: Framebuffer,
    pub memory_map: MemoryMap<'static>,
    pub acpi_table: Option<usize>,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub stride: usize,
    pub bpp: usize,
    pub buffer: *mut u8,
}

unsafe impl Send for Framebuffer {}
unsafe impl Sync for Framebuffer {}
