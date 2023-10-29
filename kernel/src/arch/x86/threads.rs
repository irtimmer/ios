use x86_64::VirtAddr;
use x86_64::instructions::segmentation::Segment;
use x86_64::structures::idt::InterruptStackFrameValue;
use x86_64::registers::segmentation;

use crate::arch::system::ThreadContext;

use super::CpuData;

// Enable interrupts
const STACK_FRAME_INTERRUPT_FLAG: u64 = 0x200;

#[derive(Clone)]
pub enum ThreadState {
    Starting(u64, u64)
}

impl ThreadContext for ThreadState {
    fn new(ip: u64, sp: u64) -> Self {
        Self::Starting(ip, sp)
    }

    unsafe fn activate(&self) -> ! {
        match self {
            ThreadState::Starting(ip, sp) => {
                let selectors = &CpuData::get().selectors;
                let stack_frame = InterruptStackFrameValue {
                    instruction_pointer: VirtAddr::new(*ip),
                    stack_pointer: VirtAddr::new(*sp),
                    code_segment: selectors.user_code.0 as u64,
                    stack_segment: selectors.user_data.0 as u64,
                    cpu_flags: STACK_FRAME_INTERRUPT_FLAG
                };

                segmentation::DS::set_reg(selectors.user_data);

                stack_frame.iretq()
            }
        }
    }
}
