use core::fmt;
use core::slice;

use pci_types::{PciAddress, ConfigRegionAccess, PciHeader, EndpointHeader, MAX_BARS, Bar};

use crate::arch::PciConfigRegion;
use crate::arch::system::System;
use crate::runtime::runtime;

pub struct PciDevice {
    pub address: PciAddress,
    pub bars: [Option<&'static [u8]>; MAX_BARS],
    access: PciConfigRegion
}

impl PciDevice {
    pub fn new(address: PciAddress, access: PciConfigRegion) -> Self{
        Self {
            bars: [None; MAX_BARS],
            address,
            access
        }
    }

    pub fn init(&mut self) -> Result<(), &'static str> {
        let header = PciHeader::new(self.address);
        let endpoint = EndpointHeader::from_header(header, &self.access).ok_or("Endpoint header not found!")?;

        let mut skip_next = false;
        for slot in 0..MAX_BARS as u8 {
            if skip_next {
                skip_next = false;
                continue;
            }
            match endpoint.bar(slot, &self.access) {
                Some(Bar::Memory64 { address, size, prefetchable }) => {
                    unsafe { runtime().system.map(address as usize, address as usize, size as usize); }
                    self.bars[slot as usize] = unsafe { Some(slice::from_raw_parts_mut(address as *mut u8, size as usize)) };
                    skip_next = true;
                },
                Some(Bar::Memory32 { address, size, prefetchable }) => {
                    unsafe { runtime().system.map(address as usize, address as usize, size as usize); }
                    self.bars[slot as usize] = unsafe { Some(slice::from_raw_parts_mut(address as *mut u8, size as usize)) };
                },
                _ => {}
            }
        }
        Ok(())
    }

    pub fn access(&self) -> PciConfigRegion {
        return self.access.clone();
    }

    pub fn read_register(&self, register: u16) -> u32 {
		unsafe { self.access.read(self.address, register) }
	}

	pub fn write_register(&self, register: u16, value: u32) {
		unsafe { self.access.write(self.address, register, value) }
	}
}

impl fmt::Display for PciDevice {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let header = PciHeader::new(self.address);
		let (vendor_id, device_id) = header.id(&self.access);
		let (_dev_rev, class_id, subclass_id, _interface) = header.revision_and_class(&self.access);

        if let Some(endpoint) = EndpointHeader::from_header(header, &self.access) {
            // Output detailed readable information about this device.
            write!(f, "{:02X}:{:02X} [{:02X}{:02X}]: [{:04X}:{:04X}]", self.address.bus(), self.address.device(), class_id, subclass_id, vendor_id, device_id)?;

            let (subsystem_id, vendor_id) = endpoint.subsystem(&self.access);
            write!(f, " [{:04X}:{:04X}]", vendor_id, subsystem_id)?;

            // If the devices uses an IRQ, output this one as well.
            let (_, irq) = endpoint.interrupt(&self.access);
            if irq != 0 && irq != u8::MAX {
                write!(f, ", IRQ {irq}")?;
            }

            let mut slot: u8 = 0;
			while usize::from(slot) < MAX_BARS {
				if let Some(pci_bar) = endpoint.bar(slot, &self.access) {
					match pci_bar {
						Bar::Memory64 { address, size, prefetchable } => {
							write!(f, "\n BAR{slot} Memory64 {{ address: {address:#X}, size: {size:#X}, prefetchable: {prefetchable} }}")?;
							slot += 1;
						}
						Bar::Memory32 { address, size, prefetchable } => {
							write!(f, "\n BAR{slot} Memory32 {{ address: {address:#X}, size: {size:#X}, prefetchable: {prefetchable} }}")?;
						}
						Bar::Io { port } => {
							write!(f, "\n BAR{slot} IO {{ port: {port:#X} }}")?;
						}
					}
				}
				slot += 1;
			}
        }

        Ok(())
    }
}
