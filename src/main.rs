#![no_main]
#![no_std]

mod drivers;

extern crate alloc;

use core::arch::asm;
use core::fmt::Write;
use core::panic::PanicInfo;

use drivers::video::console::Console;
use drivers::video::fb::FrameBuffer;

use linked_list_allocator::LockedHeap;

use uart_16550::SerialPort;

use uefi::proto::console::gop::GraphicsOutput;
use uefi::table::boot::{MemoryType, PAGE_SIZE};
use uefi::table::{Boot, SystemTable};
use uefi::{entry, Handle, Status};

#[global_allocator]
pub static ALLOCATOR: LockedHeap = LockedHeap::empty();

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let mut serial = unsafe { SerialPort::new(0x3f8) };
    serial.init();
    serial.write_fmt(format_args!("{:#?}\n", info)).unwrap();

    loop {
        unsafe { asm!("hlt") }
    }
}

#[entry]
#[allow(named_asm_labels)]
fn main(_handle: Handle, system_table: SystemTable<Boot>) -> Status {
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

    let mut console = Console::new(fb);
    writeln!(console, "Booting Iwan's OS!").unwrap();
    loop {
        unsafe { asm!("hlt") }
    }
}
