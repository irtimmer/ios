use alloc::boxed::Box;
use alloc::sync::Arc;

use async_trait::async_trait;

use crate::runtime::Resource;

use super::Block;

#[repr(C, packed)]
pub struct MbrHeader {
    bootstrap_code: [u8; 446],
    pub partitions: [PartitionEntry; 4],
    signature: u16
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct PartitionEntry {
    boot_indicator: u8,
    starting_chs: [u8; 3],
    partition_type: u8,
    ending_chs: [u8; 3],
    starting_lba: u32,
    size_in_sectors: u32
}

pub struct Mbr {
    header: MbrHeader,
    block: Arc<dyn Block>,
}

impl Mbr {
    pub async fn new(block: Arc<dyn Block>) -> Result<Self, &'static str> {
        let mut buf = [0u8; 512];
        block.read(buf.as_mut_slice(), 0).await?;
        let header: MbrHeader = unsafe { core::mem::transmute(buf) };

        Ok(Self {
            header,
            block
        })
    }

    pub fn partitions(&self) -> [PartitionEntry; 4] {
        self.header.partitions
    }

    pub async fn get_partition(&self, index: usize) -> Result<MbrPartition, &'static str> {
        if index > self.header.partitions.len() {
            return Err("Partition index out of bounds");
        }

        Ok(MbrPartition::new(self.header.partitions[index], self.block.clone()).await?)
    }
}

pub struct MbrPartition {
    entry: PartitionEntry,
    block: Arc<dyn Block>,
}

impl MbrPartition {
    pub async fn new(entry: PartitionEntry, block: Arc<dyn Block>) -> Result<Self, &'static str> {
        Ok(Self {
            entry,
            block
        })
    }
}

impl Resource for MbrPartition {}

#[async_trait]
impl Block for MbrPartition {
    async fn read(&self, buf: &mut [u8], sector: u64) -> Result<(), &'static str> {
        self.block.read(buf, sector + self.entry.starting_lba as u64).await
    }
}
