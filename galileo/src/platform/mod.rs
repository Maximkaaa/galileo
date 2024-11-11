//! Provides platform specific logic and [`PlatformService`] to access it.

use crate::decoded_image::DecodedImage;
use crate::error::GalileoError;
use async_trait::async_trait;

/// Service providing some platform specific functions in a generic way.
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait PlatformService {
    /// Creates a new instance of the service. This method is a part of the trait to allow other
    /// types be agnostic of the specific type of the platform service they work with.
    fn new() -> Self;
    /// Loads and decodes an image from the given url.
    async fn load_image_url(&self, url: &str) -> Result<DecodedImage, GalileoError>;
    /// Loads a byte array from the given url.
    async fn load_bytes_from_url(&self, url: &str) -> Result<bytes::Bytes, GalileoError>;
}

#[cfg(not(target_arch = "wasm32"))]
pub mod native;

/// Platform service implementation for the current platform.
#[cfg(not(target_arch = "wasm32"))]
/// Default implementation of the [`PlatformService`] for the current platform.
pub type PlatformServiceImpl = native::NativePlatformService;

#[cfg(target_arch = "wasm32")]
pub mod web;

/// Platform service implementation for the current platform.
#[cfg(target_arch = "wasm32")]
/// Default implementation of the [`PlatformService`] for the current platform.
pub type PlatformServiceImpl = web::WebPlatformService;
