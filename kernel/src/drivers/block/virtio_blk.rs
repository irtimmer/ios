use alloc::boxed::Box;
use alloc::sync::Arc;

use async_trait::async_trait;

use spin::Mutex;

use crate::arch::system::System;
use crate::block::Block;
use crate::drivers::pci::PciDevice;
use crate::drivers::virtio::pci::{DeviceStatus, VirtioPciDevice};
use crate::drivers::virtio::virtq::{Virtq, Descriptor, VIRTQ_DESC_F_WRITE};
use crate::runtime::{Resource, runtime};

pub struct VirtioBlk {
    device: VirtioPciDevice<BlkConfig>,
    queue: Arc<Mutex<Virtq>>
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

#[allow(non_camel_case_types, dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum BlkRequestType {
    VIRTIO_BLK_T_IN = 0,
    VIRTIO_BLK_T_OUT = 1,
    VIRTIO_BLK_T_FLUSH = 4,
    VIRTIO_BLK_T_DISCARD = 11,
    VIRTIO_BLK_T_WRITE_ZEROES = 13,
}

#[allow(non_camel_case_types, dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum BlkRequestStatus {
    VIRTIO_BLK_S_OK = 0,
    VIRTIO_BLK_S_IOERR = 1,
    VIRTIO_BLK_S_UNSUPP = 2,
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct BlkRequest {
    request_type: BlkRequestType,
    reserved: u32,
    sector: u64,
}

impl VirtioBlk {
    pub fn new(device: Arc<PciDevice>) -> Result<Self, &'static str> {
        let mut virtio = VirtioPciDevice::new(device)?;

        virtio.set_device_status(DeviceStatus::ACKNOWLEDGE);
        virtio.set_device_status(DeviceStatus::DRIVER);

        let _device_features = virtio.get_features();
        virtio.set_device_status(DeviceStatus::FEATURES_OK);

        let mut queue_handler = virtio.get_virtq_handler(0).ok_or("Virtqueue not found!")?;
        queue_handler.set_msix_vector(0);

        let queue = Virtq::new(&mut queue_handler);

        let mut device = VirtioBlk {
            device: virtio,
            queue: Arc::new(Mutex::new(queue))
        };

        let dev_queue = device.queue.clone();
        let vector = runtime().system.request_irq_handler(Box::new(move || {
            unsafe { dev_queue.force_unlock(); }
            dev_queue.lock().process();
        })).unwrap();

        let entries = device.device.msix.entries(device.device.pci.bars[1].unwrap().as_ptr() as usize);

        let processor = 0;
        let edgetrigger = false;
        let deassert = false;
        entries[0].message_data = (vector as u32 & 0xFF)
            | (if edgetrigger { 0 } else { 1 << 15 })
            | (if deassert { 0 } else { 1 << 14 });
        entries[0].message_address = 0xFEE00000 | (processor << 12);
        entries[0].vector_control = 0;

        device.device.set_device_status(DeviceStatus::DRIVER_OK);

        Ok(device)
    }
}

#[async_trait]
impl Block for VirtioBlk {
    async fn read(&self, buf: &mut [u8], sector: u64) -> Result<(), &'static str> {
        let mut status = BlkRequestStatus::VIRTIO_BLK_S_IOERR;

        let mut blk_request = Box::pin(BlkRequest {
            request_type: BlkRequestType::VIRTIO_BLK_T_IN,
            reserved: 0,
            sector
        });

        let descs = [
            Descriptor::new(blk_request.as_mut().get_mut(), 0),
            Descriptor::new_raw(buf.as_mut_ptr(), buf.len(), VIRTQ_DESC_F_WRITE),
            Descriptor::new(&mut status, VIRTQ_DESC_F_WRITE),
        ];

        self.queue.lock().request(&descs).await.unwrap();

        Ok(())
    }
}
