//! Module for types that behave differently on different platforms.

use crate::decoded_image::DecodedImage;
use crate::error::GalileoError;
use async_trait::async_trait;

/// Platform service.
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait PlatformService {
    /// Create a new instance of the service.
    fn new() -> Self;
    /// Load and decode an image from the given url.
    async fn load_image_url(&self, url: &str) -> Result<DecodedImage, GalileoError>;
    /// Load binary data from the given url.
    async fn load_bytes_from_url(&self, url: &str) -> Result<bytes::Bytes, GalileoError>;
}

#[cfg(not(target_arch = "wasm32"))]
pub mod native;

/// Platform service implementation for the current platform.
#[cfg(not(target_arch = "wasm32"))]
pub type PlatformServiceImpl = native::NativePlatformService;

#[cfg(target_arch = "wasm32")]
pub mod web;

/// Platform service implementation for the current platform.
#[cfg(target_arch = "wasm32")]
pub type PlatformServiceImpl = web::WebPlatformService;
