use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;

use async_trait::async_trait;

use futures_util::stream::BoxStream;

pub mod vfat;

#[async_trait]
pub trait FileSystem: Send + Sync {
    async fn open(self: Arc<Self>, path: &str) -> Result<Box<dyn File>, &'static str>;
    async fn open_node(self: Arc<Self>, inode: u64, len: u64) -> Result<Box<dyn File>, &'static str>;
    async fn listdir(&self, path: &str) -> Result<BoxStream<FileEntry>, &'static str>;
}

#[derive(Debug)]
pub struct FileEntry {
    pub inode: u64,
    pub name: String,
    pub length: usize,
}

#[async_trait]
pub trait File: Send + Sync {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str>;
}
