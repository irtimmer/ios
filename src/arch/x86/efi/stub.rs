use uefi::proto::console::gop::GraphicsOutput;
use uefi::table::boot::{MemoryType, PAGE_SIZE};
use uefi::table::{Boot, SystemTable};
use uefi::{entry, Handle, Status};

use crate::drivers::video::fb::FrameBuffer;
use crate::{main, ALLOCATOR};

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

    main(fb);
}
