use alloc::sync::Arc;

use core::fmt::Debug;
use core::mem;

use pci_types::capability::PciCapability;
use pci_types::{ConfigRegionAccess, EndpointHeader, PciHeader};

use crate::arch::system::System;
use crate::arch::Arch;
use crate::drivers::pci::{PciDevice, MsixCapability};

use super::virtq::VirtqHandler;

/// ISR status structure of Virtio PCI devices.
/// See Virtio specification v1.1. - 4.1.4.5
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(C)]
struct IsrStatusRaw {
	flags: u8,
}

/// An enum of the device's status field interpretations.
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum DeviceStatus {
    ACKNOWLEDGE = 1,
    DRIVER = 2,
    DRIVER_OK = 4,
    FEATURES_OK = 8,
    DEVICE_NEEDS_RESET = 64,
    FAILED = 128,
}

/// Common configuration structure of Virtio PCI devices.
/// See Virtio specification v1.1 - 4.1.43
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
#[repr(C)]
pub struct ComCfgRaw {
	device_feature_select: u32,
	device_feature: u32,
	driver_feature_select: u32,
	driver_feature: u32,
	config_msix_vector: u16,
	num_queues: u16,
	device_status: u8,
	config_generation: u8,

	pub queue_select: u16,
	pub queue_size: u16,
	pub queue_msix_vector: u16,
	pub queue_enable: u16,
	pub queue_notify_off: u16,
	pub queue_desc: u64,
	pub queue_driver: u64,
	pub queue_device: u64
}

/// Virtio's cfg_type constants; indicating type of structure in capabilities list
/// See Virtio specification v1.1 - 4.1.4
#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum CfgType {
	INVALID = 0,
	VIRTIO_PCI_CAP_COMMON_CFG = 1,
	VIRTIO_PCI_CAP_NOTIFY_CFG = 2,
	VIRTIO_PCI_CAP_ISR_CFG = 3,
	VIRTIO_PCI_CAP_DEVICE_CFG = 4,
	VIRTIO_PCI_CAP_PCI_CFG = 5,
	VIRTIO_PCI_CAP_SHARED_MEMORY_CFG = 8,
}

/// Virtio's PCI capabilities structure.
/// See Virtio specification v.1.1 - 4.1.4
#[derive(Default, Debug, Clone)]
#[repr(C)]
struct VirtioPciCapability {
	cap_vndr: u8,
	cap_next: u8,
	cap_len: u8,
	cfg_type: CfgType,
	bar_index: u8,
	id: u8,
	padding: [u8; 2],
	offset: u32,
	length: u32,

	notify_off_multiplier: u32,
}

#[derive(Default, Debug)]
struct Notify {
    memory: &'static [u8],
    notify_off_multiplier: u32
}

impl Default for CfgType {
    fn default() -> Self { CfgType::INVALID }
}

pub struct VirtioPciDevice<S: 'static> {
    pub pci: Arc<PciDevice>,
    common: &'static mut ComCfgRaw,
    isr_cfg: &'static mut IsrStatusRaw,
    notify_cfg: Notify,
    device_cfg: &'static mut S,
    pub msix: MsixCapability
}

impl<S> VirtioPciDevice<S> {
    pub fn new(device: Arc<PciDevice>) -> Result<Self, &'static str> {
        let header = PciHeader::new(device.address);
        let access = device.access();
        let endpoint_header = EndpointHeader::from_header(header, &access).unwrap();

        if endpoint_header.capability_pointer(&access) == 0 {
            return Err("Virtio device does not have a capability pointer");
        }

        let mut msix = None;
        let mut common_cfg = None;
        let mut dev_cfg = None;
        let mut isr_cfg = None;
        let mut notify_cfg = None;

        endpoint_header.capabilities(&device.access()).for_each(|c| {
            match c {
                PciCapability::MsiX(address) => {
                    let mut cap = MsixCapability::default();
                    let cap_addr = &mut cap as *mut MsixCapability;
                    for offset in (0..core::mem::size_of::<MsixCapability>()).step_by(4) {
                        let value = unsafe { device.access().read(address.address, address.offset + offset as u16) };
                        unsafe { *(cap_addr.byte_offset(offset as isize) as *mut u32) = value; }
                    }
                    cap.enable();
                    msix = Some(cap);

                    for offset in (0..core::mem::size_of::<MsixCapability>()).step_by(4) {
                        let value = unsafe { *(cap_addr.byte_offset(offset as isize) as *mut u32) };
                        unsafe { device.access().write(address.address, address.offset + offset as u16, value) };
                    }
                },
                PciCapability::Vendor(address) => {
                    let mut cap = VirtioPciCapability::default();
                    let cap_addr = &mut cap as *mut VirtioPciCapability;
                    for offset in (0..core::mem::size_of::<VirtioPciCapability>()).step_by(4) {
                        let value = unsafe { device.access().read(address.address, address.offset + offset as u16) };
                        unsafe { *(cap_addr.byte_offset(offset as isize) as *mut u32) = value; }
                    }

                    if let Some(bar) = device.bars[cap.bar_index as usize] {
                        match cap.cfg_type {
                            CfgType::VIRTIO_PCI_CAP_COMMON_CFG => {
                                common_cfg = unsafe { Some(mem::transmute(bar.as_ptr().byte_add(cap.offset as usize))) };
                            },
                            CfgType::VIRTIO_PCI_CAP_DEVICE_CFG => {
                                dev_cfg = unsafe { Some(mem::transmute(bar.as_ptr().byte_add(cap.offset as usize))) };
                            },
                            CfgType::VIRTIO_PCI_CAP_ISR_CFG => {
                                isr_cfg = unsafe { Some(mem::transmute(bar.as_ptr().byte_add(cap.offset as usize))) };
                            },
                            CfgType::VIRTIO_PCI_CAP_NOTIFY_CFG => {
                                notify_cfg = Some(Notify {
                                    memory: &bar[cap.offset as usize..(cap.offset + cap.length) as usize],
                                    notify_off_multiplier: cap.notify_off_multiplier
                                });
                            },
                            _ => {}
                        }
                    }
                },
                _ => {}
            }
        });

        Ok(Self {
            pci: device.clone(),
            common: common_cfg.ok_or("No common config found")?,
            isr_cfg: isr_cfg.ok_or("No ISR config found")?,
            notify_cfg: notify_cfg.ok_or("No notify config found")?,
            device_cfg: dev_cfg.ok_or("No device config found")?,
            msix: msix.ok_or("No MSI-X configuration")?,
        })
    }

    pub fn set_device_status(&mut self, status: DeviceStatus) {
        Arch::memory_barrier();
        self.common.device_status |= status as u8;
    }

    pub fn get_features(&mut self) -> u64 {
        Arch::memory_barrier();
        self.common.device_feature_select = 1;
        Arch::memory_barrier();

        // read high 32 bits of device features
        let mut dev_feat = u64::from(self.common.device_feature) << 32;

        // Indicate device to show low 32 bits in device_feature field.
        // See Virtio specification v1.1. - 4.1.4.3
        self.common.device_feature_select = 0;
        Arch::memory_barrier();

        // read low 32 bits of device features
        dev_feat |= u64::from(self.common.device_feature);

        dev_feat
    }

    pub fn get_virtq_handler(&mut self, index: u16) -> Option<VirtqHandler<'_>> {
        self.common.queue_select = index;
        let notify_addr = unsafe { self.notify_cfg.memory.as_ptr().byte_add(self.common.queue_notify_off as usize * self.notify_cfg.notify_off_multiplier as usize) };

        if self.common.queue_size == 0 {
            None
        } else {
            Some(VirtqHandler::new(self.common, index, notify_addr as *mut u16))
        }
	}
}
