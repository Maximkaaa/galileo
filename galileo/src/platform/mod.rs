use crate::error::GalileoError;
use crate::primitives::DecodedImage;
use async_trait::async_trait;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait PlatformService {
    fn new() -> Self;
    async fn load_image_url(&self, url: &str) -> Result<DecodedImage, GalileoError>;
    async fn load_bytes_from_url(&self, url: &str) -> Result<bytes::Bytes, GalileoError>;
}

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub type PlatformServiceImpl = native::NativePlatformService;

#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
pub type PlatformServiceImpl = web::WebPlatformService;
