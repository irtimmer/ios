use alloc::boxed::Box;

use async_trait::async_trait;

use crate::runtime::Resource;

#[async_trait]
pub trait Block: Resource {
    async fn read(&self, buf: &mut [u8], sector: u64) -> Result<(), &'static str>;
}
