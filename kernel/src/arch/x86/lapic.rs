use core::arch::asm;
use core::fmt::Write;
use core::mem;

use alloc::boxed::Box;
use spin::Once;

use x2apic::lapic::{LocalApic, LocalApicBuilder, xapic_base};

use x86_64::structures::idt::InterruptStackFrame;

use crate::runtime::runtime;

use super::{interrupts, CpuData};
use super::threads::Context;

static LOCAL_APIC: Once<LocalApic> = Once::new();

pub struct Interrupts {
    pub handlers: [Option<Box<dyn Fn()>>; 240 - 64]
}

pub fn init() {
    LOCAL_APIC.call_once(|| {
        let apic_address: u64 = unsafe { xapic_base() };

        LocalApicBuilder::new()
            .spurious_vector(interrupts::SPURIOUS_INTERRUPT_INDEX)
            .timer_vector(interrupts::TIMER_INTERRUPT_INDEX)
            .error_vector(interrupts::ERROR_INTERRUPT_INDEX)
            .set_xapic_base(apic_address)
            .build()
            .unwrap_or_else(|err| panic!("{}", err))
    });
    let lapic = local_apic();

    unsafe {
        lapic.enable();
        lapic.set_timer_mode(x2apic::lapic::TimerMode::Periodic);
        lapic.set_timer_divide(x2apic::lapic::TimerDivide::Div128);
        //lapic.disable_timer();
        lapic.enable_timer();
    }
}

#[allow(mutable_transmutes)]
pub(super) fn local_apic() -> &'static mut LocalApic {
    // It's safe as LAPIC is per-cpu.
    unsafe { mem::transmute(LOCAL_APIC.call_once(|| unreachable!())) }
}

pub extern "x86-interrupt" fn spurious_interrupt_handler(_: InterruptStackFrame) {
    write!(runtime().console.lock(), "!").unwrap();
    unsafe {
        local_apic().end_of_interrupt();
    }
}

#[naked]
pub unsafe extern fn timer_interrupt_handler() -> ! {
    asm!(r#"
        push r15
        push r14
        push r13
        push r12
        push r11
        push r10
        push r9
        push r8
        push rdi
        push rsi
        push rdx
        push rcx
        push rbx
        push rax
        push rbp
        mov rdi, rsp
        sub rsp, 0x800
        jmp {}
    "#, sym timer_interrupt, options(noreturn));
}

extern "C" fn timer_interrupt(ctx: &Context) {
    write!(runtime().console.lock(), ".").unwrap();
    unsafe {
        local_apic().end_of_interrupt();
        ctx.restore();
    }
}

pub extern "x86-interrupt" fn error_interrupt_handler(_: InterruptStackFrame) {
    panic!("LAPIC Error interrupt");
}

pub fn general_interrupt_handler(_stack_frame: InterruptStackFrame, index: u8, _error_code: Option<u64>) {
    CpuData::get().interrupts.handlers[index as usize - 64].as_ref().map(|handler| handler());
    unsafe { local_apic().end_of_interrupt() };
}
