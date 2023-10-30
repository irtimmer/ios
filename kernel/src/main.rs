#![feature(abi_x86_interrupt)]
#![feature(core_intrinsics)]
#![feature(fn_align)]
#![feature(naked_functions)]
#![feature(pointer_byte_offsets)]

#![no_main]
#![no_std]

mod arch;
mod block;
mod drivers;
mod fs;
mod process;
mod runtime;
mod scheduler;
mod shell;
mod tasks;

extern crate alloc;

use arch::Arch;
use arch::system::System;

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
        Arch::sleep();
    }
}

fn main() -> ! {
    writeln!(runtime().console.lock(), "Booting Iwan's OS!").unwrap();

    let mut executor = Executor::new();
    executor.spawn(Task::new(shell::ios_shell()));

    loop {
        executor.run_ready_tasks();
        Arch::sleep();
    }
}
