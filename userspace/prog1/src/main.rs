#![no_main]
#![no_std]

use core::arch::asm;
use core::panic::PanicInfo;

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
extern "C" fn _start() -> ! {
    print("Hello World!\n");
    loop {}
}

fn print(str: &str) {
    unsafe { asm!("syscall", in("rdi") 1, in("rsi") str.as_ptr(), in("rdx") str.len()) }
}
