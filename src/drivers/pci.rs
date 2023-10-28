use core::fmt;
use core::slice;

use pci_types::{PciAddress, ConfigRegionAccess, PciHeader, EndpointHeader, MAX_BARS, Bar};

use crate::arch::PciConfigRegion;
use crate::arch::system::System;
use crate::runtime::runtime;

#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone)]
pub struct MsixCapability {
	cap_id: u8,
	cap_next: u8,
    message_control: u16,
    table_offset: u32,
    pba_offset: u32,
}

// Define structure for MSI-X Table Entry
#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone)]
pub struct MsixTableEntry {
    pub message_address: u64,
    pub message_data: u32,
    pub vector_control: u32
}

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

impl MsixCapability {
    pub fn entries(&mut self, bar_address: usize) -> &mut [MsixTableEntry] {
        let length = self.message_control & 0x3FF;
        let address = (self.table_offset & 0xFFFC) as usize + bar_address;
        unsafe { core::slice::from_raw_parts_mut(address as *mut MsixTableEntry, length as usize) }
    }

    pub fn enable(&mut self) {
        self.message_control |= 0x8000;
    }
}

impl fmt::Display for MsixCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "MSI-X Capability:")?;
        writeln!(f, " Cap ID: {}", self.cap_id)?;
        writeln!(f, " Cap Next: {}", self.cap_next)?;
        let enabled = self.message_control & 0x8000;
        let function_mask = self.message_control & 0x4000;
        let table_size = self.message_control & 0x3FF;
        let mc = self.message_control;
        writeln!(f, " Message Control: {:b} {}/{} {}", mc, enabled, function_mask, table_size)?;
        let table = self.table_offset & 0xFFFC;
        let bir = self.table_offset & 0x3;
        writeln!(f, " Table Offset: {}/{}", table, bir)?;
        let pba = self.pba_offset & 0xFFFC;
        let pbir = self.pba_offset & 0x3;
        write!(f, " PBA Offset: {}/{}", pba, pbir)?;

        Ok(())
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
