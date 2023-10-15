use core::fmt::Write;

use bitflags::bitflags;

use pc_keyboard::{layouts, DecodedKey, ScancodeSet2};

use spin::Mutex;

use x86_64::instructions::port::Port;

use crate::runtime::runtime;
struct I8042 {
    data_port: Port<u8>,
    control_port: Port<u8>,
}

bitflags! {
    pub struct ConfigFlags: u8 {
        const FIRST_PORT_INTERRUPT = 1 << 0;
        const SECOND_PORT_INTERRUPT = 1 << 1;
        const SYSTEM_FLAG = 1 << 2;
        const FIRST_PORT_CLOCK = 1 << 4;
        const SECOND_PORT_CLOCK = 1 << 5;
        const FIRST_PORT_TRANSLATION = 1 << 6;
    }
}

const MAX_SPINS: usize = 1000;

const I8042_DATA_PORT: u16 = 0x60;
const I8042_CONTROL_PORT: u16 = 0x64;

impl I8042 {
    pub fn new() -> Self {
        Self {
            data_port: Port::new(I8042_DATA_PORT),
            control_port: Port::new(I8042_CONTROL_PORT),
        }
    }

    pub fn poll_out(&mut self) -> bool {
        unsafe {
            self.control_port.read() & 2 == 0
        }
    }

    pub fn poll_in(&mut self) -> bool {
        unsafe {
            self.control_port.read() & 1 != 0
        }
    }

    pub fn wait_out(&mut self) {
        let mut spin_count = 0;
        while !self.poll_out() {
            spin_count += 1;
            if spin_count == MAX_SPINS {
                panic!("i8042 write timeout")
            }
        }
    }

    pub fn wait_in(&mut self) {
        let mut spin_count = 0;
        while !self.poll_in() {
            spin_count += 1;
            if spin_count == MAX_SPINS {
                panic!("i8042 write timeout")
            }
        }
    }

    pub fn write_cmd(&mut self, cmd: u8) {
        self.wait_out();
        unsafe {
            self.control_port.write(cmd);
        }
    }

    pub fn write_cmd_data(&mut self, cmd: u8, data: u8) {
        self.wait_out();
        unsafe {
            self.control_port.write(cmd);
        }
        self.wait_out();
        unsafe {
            self.data_port.write(data);
        }
    }

    pub fn read_data(&mut self) -> u8 {
        self.wait_in();
        unsafe {
            self.data_port.read()
        }
    }

    pub fn flush(&mut self) {
        let mut spin_count = 0;
        while self.poll_in() {
            spin_count += 1;
            if spin_count == MAX_SPINS {
                panic!("i8042 flush timeout")
            }

            unsafe {
                self.data_port.read();
            }
        }
    }

    pub fn init(&mut self) {
        // Disable first and second port
        self.write_cmd(0xAD);
        self.write_cmd(0xA7);
        self.flush();

        // Set config
        self.write_cmd(0x20);
        let mut config = self.read_data();

        config &= !(ConfigFlags::FIRST_PORT_INTERRUPT
            | ConfigFlags::SECOND_PORT_INTERRUPT
            | ConfigFlags::FIRST_PORT_TRANSLATION)
            .bits();

        let can_have_second_port = config & (1 << 5) != 0;

        self.flush();

        self.write_cmd_data(0x60, config);

        // Self test
        self.write_cmd(0xAA);

        let result = unsafe { self.data_port.read() };
        if result != 0x55 {
            panic!("i8042 self test failed");
        }

        let has_second_port = if can_have_second_port {
            // Enable and disable 2nd port, see if the config changes in response
            self.write_cmd(0xA8);
            self.write_cmd(0x20);
            let config = self.read_data();
            self.write_cmd(0xA7);
            config & (1 << 5) == 0
        } else {
            false
        };

        let port1_works = {
            self.write_cmd(0xAB);
            self.read_data() == 0x00
        };

        let port2_works = if has_second_port {
            self.write_cmd(0xA9);
            self.read_data() == 0x00
        } else {
            false
        };

        if !port1_works && !port2_works {
            panic!("No working ports");
        }

        // Enable interrupts
        self.write_cmd(0x20);
        let mut config = self.read_data();

        if port1_works {
            config |= ConfigFlags::FIRST_PORT_INTERRUPT.bits();
        }
        if port2_works {
            config |= ConfigFlags::SECOND_PORT_INTERRUPT.bits();
        }

        self.write_cmd_data(0x60, config);

        // Enable ports
        if port1_works {
            self.write_cmd_data(0xAE, 0xFF);
        }

        if port2_works {
            self.write_cmd(0xA8);
            self.write_cmd_data(0xD4, 0xFF);
        }

        //serial_println!("i8042 init done");
        self.flush()
    }
}

pub struct PcKeyboard {
    i8042: Mutex<I8042>,
    processor: Mutex<pc_keyboard::Keyboard<layouts::Us104Key, ScancodeSet2>>,
}

impl PcKeyboard {
    pub fn new() -> Self {
        Self {
            i8042: Mutex::new(I8042::new()),
            processor: Mutex::new(pc_keyboard::Keyboard::new(ScancodeSet2::new(), layouts::Us104Key, pc_keyboard::HandleControl::Ignore)),
        }
    }

    pub fn init(&self) {
        self.i8042.lock().init();
    }

    pub fn read_scancode(&self) {
        self.add_scancode(self.i8042.lock().read_data());
    }

    pub fn add_scancode(&self, scancode: u8) {
        let mut kbd = self.processor.lock();
        if let Ok(Some(key_event)) = kbd.add_byte(scancode) {
            if let Some(key) = kbd.process_keyevent(key_event) {
                match key {
                    DecodedKey::Unicode(character) => runtime().console.lock().write_char(character).unwrap(),
                    DecodedKey::RawKey(_) => {}
                }
            }
        }
    }
}
