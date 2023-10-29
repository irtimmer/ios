use spin::Once;

use x86_64::set_general_handler;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

use super::lapic::general_interrupt_handler;
use super::{lapic, ioapic};

pub const IOAPIC_INTERRUPT_OFFSET: usize = 32;
pub const KEYBOARD_INTERRUPT_INDEX: usize = IOAPIC_INTERRUPT_OFFSET + 1;

pub const SPURIOUS_INTERRUPT_INDEX: usize = 240;
pub const TIMER_INTERRUPT_INDEX: usize = 241;
pub const ERROR_INTERRUPT_INDEX: usize = 242;

static IDT: Once<InterruptDescriptorTable> = Once::new();

pub fn init() {
    IDT.call_once(|| {
        let mut idt = InterruptDescriptorTable::new();
        idt.general_protection_fault.set_handler_fn(general_protection_handler);
        idt.stack_segment_fault.set_handler_fn(stack_segment_handler);
        idt.alignment_check.set_handler_fn(aligment_check_handler);
        idt.cp_protection_exception.set_handler_fn(cp_protection_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt.double_fault.set_handler_fn(double_fault_handler);

        idt[SPURIOUS_INTERRUPT_INDEX].set_handler_fn(lapic::spurious_interrupt_handler);
        idt[TIMER_INTERRUPT_INDEX].set_handler_fn(lapic::timer_interrupt_handler);
        idt[ERROR_INTERRUPT_INDEX].set_handler_fn(lapic::error_interrupt_handler);

        idt[KEYBOARD_INTERRUPT_INDEX].set_handler_fn(ioapic::keyboard_interrupt_handler);

        set_general_handler!(&mut idt, general_interrupt_handler, 64..240);
        idt
    }).load();

    x86_64::instructions::interrupts::enable();
}

extern "x86-interrupt" fn general_protection_handler(_: InterruptStackFrame, error_code: u64)  {
    panic!("GP Fault {}", error_code);
}

extern "x86-interrupt" fn stack_segment_handler(_: InterruptStackFrame, error_code: u64)  {
    panic!("SS Fault {}", error_code);
}

extern "x86-interrupt" fn aligment_check_handler(_: InterruptStackFrame, error_code: u64)  {
    panic!("AC Fault {}", error_code);
}

extern "x86-interrupt" fn cp_protection_handler(_: InterruptStackFrame, error_code: u64)  {
    panic!("CP Fault {}", error_code);
}

extern "x86-interrupt" fn double_fault_handler(_: InterruptStackFrame, error_code: u64) -> ! {
    panic!("Double Fault {}", error_code);
}

extern "x86-interrupt" fn page_fault_handler(stack_frame: InterruptStackFrame, error_code: PageFaultErrorCode) {
    use core::fmt::Write;
    use uart_16550::SerialPort;
    use x86_64::registers::control::Cr2;
    use crate::arch::Arch;
    use crate::arch::system::System;

    let mut serial = unsafe { SerialPort::new(0x3f8) };
    serial.init();

    writeln!(serial, "Page Fault {:?}", error_code).unwrap();
    writeln!(serial, "Accessed Address: {:?}", Cr2::read()).unwrap();
    writeln!(serial, "{:#?}", stack_frame).unwrap();
    loop { Arch::sleep() }
}
