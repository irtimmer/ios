use spin::{Once, Mutex};

use crate::drivers::video::console::Console;
use crate::drivers::video::fb::FrameBuffer;

pub static RUNTIME: Once<Runtime> = Once::new();

pub struct Runtime {
    pub console: Mutex<Console>
}

impl Runtime {
    pub fn init(fb: FrameBuffer) -> &'static Self {
        RUNTIME.call_once(|| {
            Runtime {
                console: Mutex::new(Console::new(fb))
            }
        })
    }
}

pub fn runtime() -> &'static Runtime {
    RUNTIME.wait()
}
