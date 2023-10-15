use spin::{Once, Mutex};

use crate::arch::Arch;
use crate::drivers::i8042::PcKeyboard;
use crate::drivers::video::console::Console;
use crate::drivers::video::fb::FrameBuffer;

pub static RUNTIME: Once<Runtime> = Once::new();

pub struct Runtime {
    pub system: Arch,
    pub console: Mutex<Console>,
    pub keyboard: PcKeyboard
}

impl Runtime {
    pub fn init(system: Arch, fb: FrameBuffer, kbd: PcKeyboard) -> &'static Self {
        RUNTIME.call_once(|| {
            Runtime {
                system,
                console: Mutex::new(Console::new(fb)),
                keyboard: kbd
            }
        })
    }
}

pub fn runtime() -> &'static Runtime {
    RUNTIME.wait()
}
