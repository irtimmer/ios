use core::arch::asm;

use x86_64::VirtAddr;
use x86_64::instructions::segmentation::Segment;
use x86_64::structures::idt::InterruptStackFrameValue;
use x86_64::registers::segmentation;

use crate::arch::system::ThreadContext;

use super::CpuData;

// Enable interrupts
const STACK_FRAME_INTERRUPT_FLAG: u64 = 0x200;

#[derive(Debug, Clone)]
#[repr(C)]
pub struct Context {
    rbp: u64,
    rax: u64,
    rbx: u64,
    rcx: u64,
    rdx: u64,
    rsi: u64,
    rdi: u64,
    r8: u64,
    r9: u64,
    r10: u64,
    r11: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,
    stack_frame: InterruptStackFrameValue
}

impl Context {
    #[inline(always)]
    pub unsafe fn restore(&self) -> ! {
        asm!(r#"
            mov rsp, {}
            pop rbp
            pop rax
            pop rbx
            pop rcx
            pop rdx
            pop rsi
            pop rdi
            pop r8
            pop r9
            pop r10
            pop r11
            pop r12
            pop r13
            pop r14
            pop r15
            iretq
        "#, in(reg) self, options(noreturn))
    }
}

#[derive(Clone)]
pub enum ThreadState {
    Paused(Context),
    Starting(u64, u64)
}

impl ThreadContext for ThreadState {
    fn new(ip: u64, sp: u64) -> Self {
        Self::Starting(ip, sp)
    }

    unsafe fn activate(&self) -> ! {
        match self {
            ThreadState::Paused(ctx) => ctx.restore(),
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
