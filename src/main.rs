#![no_main]
#![no_std]

mod arch;
mod drivers;

extern crate alloc;

use core::arch::asm;
use core::fmt::Write;
use core::panic::PanicInfo;

use drivers::video::console::Console;
use drivers::video::fb::FrameBuffer;

use linked_list_allocator::LockedHeap;

use uart_16550::SerialPort;

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

fn main(fb: FrameBuffer) -> ! {
    let mut console = Console::new(fb);
    writeln!(console, "Booting Iwan's OS!").unwrap();
    loop {
        unsafe { asm!("hlt") }
    }
}
