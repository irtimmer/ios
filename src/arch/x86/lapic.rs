use core::fmt::Write;
use core::mem;

use spin::Once;

use x2apic::lapic::{LocalApic, LocalApicBuilder, xapic_base};

use x86_64::structures::idt::InterruptStackFrame;

use crate::runtime::runtime;

use super::interrupts;

static LOCAL_APIC: Once<LocalApic> = Once::new();

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

pub extern "x86-interrupt" fn timer_interrupt_handler(_: InterruptStackFrame) {
    write!(runtime().console.lock(), ".").unwrap();
    unsafe {
        local_apic().end_of_interrupt();
    }
}

pub extern "x86-interrupt" fn error_interrupt_handler(_: InterruptStackFrame) {
    panic!("LAPIC Error interrupt");
}
