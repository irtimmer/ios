use alloc::boxed::Box;
use alloc::borrow::ToOwned;
use alloc::string::ToString;
use alloc::sync::Arc;

use async_trait::async_trait;

use core::future;
use core::str;

use futures_util::stream::{BoxStream, self};
use futures_util::StreamExt;

use crate::block::Block;

use super::{FileSystem, File, FileEntry};

#[repr(C, packed)]
pub struct DriverParameterBlock {
    rest2: [u8; 11],
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    pub reserved_sectors: u16,
    pub num_fats: u8,
    root_entries: u16,
    total_sectors_16: u16,
    media_descriptor: u8,
    pub sectors_per_fat_16: u16,
    sectors_per_track: u16,
    num_heads: u16,
    hidden_sectors: u32,
    total_sectors_32: u32,
    sectors_per_fat_32: u32,
    ext_flags: u16,
    fs_version: u16,
    root_cluster: u32,
    fs_info_sector: u16,
    backup_boot_sector: u16,
    // Padding to align to 512 bytes
    reserved: [u8; 12],
    rest: [u8; 448]
}

//FAT file entry struct
#[derive(Copy, Clone)]
#[repr(C, packed)]
pub struct Entry {
    pub name: [u8; 11],
    attributes: u8,
    reserved: u8,
    created_time_tenths: u8,
    created_time: u16,
    created_date: u16,
    accessed_date: u16,
    first_cluster_high: u16,
    modified_time: u16,
    modified_date: u16,
    first_cluster_low: u16,
    size: u32,
}

pub struct VFat16 {
    header: DriverParameterBlock,
    block: Arc<dyn Block>,
}

impl VFat16 {
    pub async fn new(block: Arc<dyn Block>) -> Result<Self, &'static str> {
        let mut buf = [0u8; 512];
        block.read(buf.as_mut_slice(), 0).await?;
        let header: DriverParameterBlock = unsafe { core::mem::transmute(buf) };

        Ok(Self {
            header,
            block
        })
    }
}

struct ListDirState {
    sector: u32,
    offset: u16,
    buf: [u8; 512]
}

#[async_trait]
impl FileSystem for VFat16 {
    async fn open(self: Arc<Self>, path: &str) -> Result<Box<dyn File>, &'static str> {
        let listdir = self.listdir(path).await?;
        let path = path.to_owned();
        let mut files = Box::pin(listdir.filter(|entry| future::ready(entry.name == path)));
        let file = files.next().await.ok_or("File not found")?;

        Ok(Box::new(VFat16File::new(self.clone(), file.length, file.inode as usize)))
    }

    async fn open_node(self: Arc<Self>, inode: u64, len: u64) -> Result<Box<dyn File>, &'static str> {
        let cluster = inode as usize;
        Ok(Box::new(VFat16File::new(self, len as usize, cluster)))
    }

    async fn listdir(&self, _path: &str) -> Result<BoxStream<FileEntry>, &'static str> {
        let root = self.header.reserved_sectors + self.header.sectors_per_fat_16 * self.header.num_fats as u16;

        let _state = ListDirState {
            // Use sector minus one to load the next sector on the first iteration
            sector: (root - 1) as u32,
            offset: 16,
            buf: [0u8; 512]
        };
        Ok(stream::unfold(_state, move |mut state| async move {
            state.offset += 1;
            if state.offset >= 16 {
                state.sector += 1;
                self.block.read(state.buf.as_mut_slice(), state.sector as u64).await.unwrap();
                state.offset = 0;
            }

            let entries = unsafe { core::slice::from_raw_parts(state.buf.as_ptr() as *mut Entry, 512 / core::mem::size_of::<Entry>()) };
            let entry = entries[state.offset as usize];

            if entry.name[0] == 0 {
                None
            } else {
                Some((FileEntry {
                    inode: entry.first_cluster_low as u64,
                    length: entry.size as usize,
                    name: str::from_utf8(&entry.name).unwrap_or("Unknown").trim_end().to_string()
                }, state))
            }
        }).boxed())
    }
}

struct VFat16File {
    offset: usize,
    length: usize,
    cluster: usize,
    fs: Arc<VFat16>,
    block: Arc<dyn Block>
}

impl VFat16File {
    pub fn new(fs: Arc<VFat16>, length: usize, cluster: usize) -> Self {
        let block = fs.block.clone();
        Self {
            offset: 0,
            length,
            fs,
            block,
            cluster
        }
    }
}

#[async_trait]
impl File for VFat16File {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        let mut data_offset = (self.fs.header.reserved_sectors + self.fs.header.sectors_per_fat_16 * self.fs.header.num_fats as u16) as usize;
        let cluster_length = self.fs.header.sectors_per_cluster as usize * self.fs.header.bytes_per_sector as usize;
        data_offset += self.fs.header.sectors_per_cluster as usize * self.cluster;

        let data_left = self.length - self.offset;
        if data_left == 0 {
            return Ok(0);
        }

        if self.offset < cluster_length {
            let current_sector = self.offset / self.fs.header.bytes_per_sector as usize;
            let sector_offset = self.offset % self.fs.header.bytes_per_sector as usize;
            let sector_left = self.fs.header.bytes_per_sector as usize - sector_offset;
            let max_read = buf.len().min(sector_left).min(data_left);
            let mut sector_buf = [0u8; 512];
            self.block.read(sector_buf.as_mut(), (data_offset + current_sector) as u64).await?;

            buf[0..max_read].copy_from_slice(&sector_buf[sector_offset..sector_offset + max_read]);
            self.offset += max_read;
            Ok(max_read)
        } else {
            Err("Not implemented")
        }
    }
}
