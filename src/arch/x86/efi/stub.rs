use uefi::proto::console::gop::GraphicsOutput;
use uefi::table::boot::{MemoryType, PAGE_SIZE};
use uefi::table::cfg::ACPI2_GUID;
use uefi::table::{Boot, SystemTable};
use uefi::{entry, Handle, Status};

use crate::arch::x86::boot::boot;
use crate::arch::x86::paging::PageMapper;
use crate::drivers::video::fb::FrameBuffer;
use crate::ALLOCATOR;

#[entry]
#[allow(named_asm_labels)]
fn efi_main(_handle: Handle, system_table: SystemTable<Boot>) -> Status {
    let fb = {
        let bt = system_table.boot_services();
        let gop_handle = bt.get_handle_for_protocol::<GraphicsOutput>().unwrap();
        let mut gop = bt.open_protocol_exclusive::<GraphicsOutput>(gop_handle).unwrap();
        FrameBuffer {
            width: gop.current_mode_info().resolution().0,
            height: gop.current_mode_info().resolution().1,
            stride: gop.current_mode_info().stride(),
            bpp: 32,
            buffer: gop.frame_buffer().as_mut_ptr()
        }
    };

    // Get location of ACPI tables
    let cfg_tbl = system_table.config_table();
    let acpi_table = cfg_tbl.iter().find(|entry| entry.guid == ACPI2_GUID).map(|entry| entry.address);

    let (_, memory_map) = system_table.exit_boot_services();

    // Initialize allocator
    for entry in memory_map.entries() {
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
    let mut mapper = PageMapper::new(0);
    for entry in memory_map.entries() {
        unsafe {
            if entry.ty == MemoryType::LOADER_CODE || entry.ty == MemoryType::LOADER_DATA || entry.ty == MemoryType::CONVENTIONAL {
                mapper.map(entry.phys_start as usize, entry.phys_start as usize, entry.page_count as usize * PAGE_SIZE);
            } else {
                mapper.map(entry.phys_start as usize, entry.phys_start as usize, entry.page_count as usize * PAGE_SIZE);
            }
        };
    }

    boot(mapper, acpi_table, fb);
}
