use crate::drivers::video::fb::FrameBuffer;
use crate::runtime::Runtime;
use crate::main;

use super::{gdt, interrupts};

pub fn boot(fb: FrameBuffer) -> ! {
    Runtime::init(fb);

    gdt::init();
    interrupts::init();

    main();
}
