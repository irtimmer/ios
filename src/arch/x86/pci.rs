use core::fmt::Write;

use pci_types::{PciAddress, ConfigRegionAccess, PciHeader};

use spin::{Mutex, Lazy};

use x86_64::instructions::port::Port;

use crate::drivers::pci::PciDevice;
use crate::runtime::runtime;

const PCI_MAX_BUS_NUMBER: u8 = 32;
const PCI_MAX_DEVICE_NUMBER: u8 = 32;

const PCI_CONFIG_ADDRESS_PORT: u16 = 0xCF8;
const PCI_CONFIG_ADDRESS_ENABLE: u32 = 1 << 31;

const PCI_CONFIG_DATA_PORT: u16 = 0xCFC;

pub struct PciAccessPorts {
    address: Port<u32>,
    data: Port<u32>,
}

impl PciAccessPorts {
	pub const fn new() -> Self {
		Self {
            address: Port::new(PCI_CONFIG_ADDRESS_PORT),
            data: Port::new(PCI_CONFIG_DATA_PORT)
        }
	}
}

static PCI_ACCESS_PORTS: Lazy<Mutex<PciAccessPorts>> = Lazy::new(|| {
    Mutex::new(PciAccessPorts::new())
});

#[derive(Debug, Clone)]
pub struct PciConfigRegion;

impl PciConfigRegion {
    pub fn new() -> Self {
        Self {}
    }
}

impl ConfigRegionAccess for PciConfigRegion {
	#[inline]
	fn function_exists(&self, _: PciAddress) -> bool {
		true
	}

	#[inline]
	unsafe fn read(&self, pci_addr: PciAddress, register: u16) -> u32 {
		let address = PCI_CONFIG_ADDRESS_ENABLE
			| u32::from(pci_addr.bus()) << 16
			| u32::from(pci_addr.device()) << 11
			| u32::from(register);

        let mut ports = PCI_ACCESS_PORTS.lock();
		unsafe {
			ports.address.write(address);
            ports.data.read()
		}
	}

	#[inline]
	unsafe fn write(&self, pci_addr: PciAddress, register: u16, value: u32) {
		let address = PCI_CONFIG_ADDRESS_ENABLE
			| u32::from(pci_addr.bus()) << 16
			| u32::from(pci_addr.device()) << 11
			| u32::from(register);

        let ports = PCI_ACCESS_PORTS.lock();
        unsafe {
            ports.address.clone().write(address);
            ports.data.clone().write(value)
        }
	}
}

pub fn init() {
	writeln!(runtime().console.lock(), "Scanning PCI Busses 0 to {}", PCI_MAX_BUS_NUMBER - 1).unwrap();

	let pci_config = PciConfigRegion::new();
	for bus in 0..PCI_MAX_BUS_NUMBER {
		for device in 0..PCI_MAX_DEVICE_NUMBER {
			let pci_address = PciAddress::new(0, bus, device, 0);
			let header = PciHeader::new(pci_address);

			let (device_id, vendor_id) = header.id(&pci_config);
			if device_id != u16::MAX && vendor_id != u16::MAX {
                let device = PciDevice::new(pci_address, pci_config.clone());
                writeln!(runtime().console.lock(), "PCI {:#}", device).unwrap();
			}
		}
	}
}
