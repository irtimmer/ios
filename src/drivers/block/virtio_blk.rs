use alloc::sync::Arc;

use crate::drivers::pci::PciDevice;
use crate::drivers::virtio::pci::{DeviceStatus, VirtioPciDevice};
use crate::runtime::Resource;

pub struct VirtioBlk {
    device: VirtioPciDevice<BlkConfig>
}

impl Resource for VirtioBlk {}

/// Virtio's block device configuration structure.
/// See specification v1.1. - 5.2.4
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct BlkConfig {
    capacity: u64,
    size_max: u32,
    seg_max: u32,
    geometry_cylinders: u16,
    geometry_heads: u8,
    geometry_sectors: u8,
    blk_size: u32,
}

impl VirtioBlk {
    pub fn new(device: Arc<PciDevice>) -> Result<Self, &'static str> {
        let mut virtio = VirtioPciDevice::new(device)?;

        virtio.set_device_status(DeviceStatus::ACKNOWLEDGE);
        virtio.set_device_status(DeviceStatus::DRIVER);

        let _device_features = virtio.get_features();
        virtio.set_device_status(DeviceStatus::FEATURES_OK);

        let device = Arc::new(VirtioBlk {
            device: virtio
        });

        Ok(device)
    }
}
