#![feature(abi_x86_interrupt)]
#![feature(pointer_byte_offsets)]

#![no_main]
#![no_std]

mod arch;
mod drivers;
mod runtime;
mod tasks;

extern crate alloc;

use core::arch::asm;
use core::fmt::Write;
use core::panic::PanicInfo;

use linked_list_allocator::LockedHeap;

use runtime::runtime;

use tasks::Task;
use tasks::executor::Executor;

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

fn main() -> ! {
    writeln!(runtime().console.lock(), "Booting Iwan's OS!").unwrap();

    let mut executor = Executor::new();

    loop {
        executor.run_ready_tasks();
        unsafe { asm!("hlt") }
    }
}
