use x2apic::ioapic::IoApic;

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
    runtime().keyboard.read_scancode();

    unsafe {
        local_apic().end_of_interrupt();
    }
}
