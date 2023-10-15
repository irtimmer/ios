use crate::drivers::video::fb::FrameBuffer;
use crate::main;

use super::{gdt, interrupts};

pub fn boot(fb: FrameBuffer) -> ! {
    gdt::init();
    interrupts::init();

    main(fb);
}
