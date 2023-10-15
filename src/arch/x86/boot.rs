use crate::drivers::video::fb::FrameBuffer;
use crate::main;

use super::gdt;

pub fn boot(fb: FrameBuffer) -> ! {
    gdt::init();

    main(fb);
}
