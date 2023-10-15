use crate::drivers::video::fb::FrameBuffer;
use crate::runtime::Runtime;
use crate::main;

use super::{gdt, interrupts, lapic};

pub fn boot(fb: FrameBuffer) -> ! {
    Runtime::init(fb);

    gdt::init();
    interrupts::init();
    lapic::init();

    main();
}
