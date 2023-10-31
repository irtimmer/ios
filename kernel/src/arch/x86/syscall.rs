use core::arch::asm;
use core::fmt::Write;
use core::slice;
use core::str;

use x86_64::VirtAddr;
use x86_64::registers::model_specific::{Efer, EferFlags, Star, LStar, SFMask};
use x86_64::registers::rflags::RFlags;

use crate::runtime::runtime;

use super::gdt::Selectors;

pub fn init(selectors: &Selectors) {
    unsafe {
        Efer::update(|flags| flags.insert(EferFlags::SYSTEM_CALL_EXTENSIONS));
    }

    SFMask::write(RFlags::INTERRUPT_FLAG);
    Star::write(selectors.user_code, selectors.user_data, selectors.code, selectors.data).unwrap();
    LStar::write(VirtAddr::new(_handle_syscall as *const () as u64));
}

#[naked]
unsafe extern fn _handle_syscall() {
    asm!(r#"
        push rax
        push rcx
        push rdx
        push r8
        push r9
        push r10
        push r11
        call {}
        pop r11
        pop r10
        pop r9
        pop r8
        pop rdx
        pop rcx
        pop rax
        sysretq
    "#, sym handle_syscall, options(noreturn));
}

extern "C" fn handle_syscall(instr: u64, arg1: u64, arg2: u64) {
    match instr {
        1 => {
            let buffer = unsafe { slice::from_raw_parts(arg1 as *const u8, arg2 as usize) };
            let string = str::from_utf8(buffer).unwrap();
            runtime().console.lock().write_str(string).unwrap();
        },
        _ => {
            writeln!(runtime().console.lock(), "HELP!! {}", instr).unwrap();
        }
    }
}
