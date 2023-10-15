use core::fmt::Write;

use x2apic::ioapic::IoApic;

use x86_64::instructions::port::Port;
use x86_64::structures::idt::InterruptStackFrame;

use crate::runtime::runtime;

use super::interrupts::{IOAPIC_INTERRUPT_OFFSET, KEYBOARD_INTERRUPT_INDEX};
use super::lapic::local_apic;

pub fn init(base_addr: u64, id: u8) {
    unsafe {
        let mut ioapic = IoApic::new(base_addr);
        ioapic.set_id(id);
        ioapic.init(IOAPIC_INTERRUPT_OFFSET as u8);
        ioapic.enable_irq((KEYBOARD_INTERRUPT_INDEX - IOAPIC_INTERRUPT_OFFSET) as u8);
    };
}

pub extern "x86-interrupt" fn keyboard_interrupt_handler(_: InterruptStackFrame) {
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    writeln!(runtime().console.lock(), "KBD: {}", scancode).unwrap();

    unsafe {
        local_apic().end_of_interrupt();
    }
}
