//! This module contains utilities for loading images to be rendered on the map.

#[cfg(not(target_arch = "wasm32"))]
use crate::error::GalileoError;

/// An image that has been loaded into memory.
#[derive(Debug, Clone)]
pub struct DecodedImage {
    /// Raw bytes of the image, in RGBA order.
    pub(crate) bytes: Vec<u8>,
    /// Width and height of the image.
    pub(crate) dimensions: (u32, u32),
}

impl DecodedImage {
    /// Decode an image from a byte slice.
    ///
    /// Attempts to guess the format of the image from the data. Non-RGBA images
    /// will be converted to RGBA.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(bytes: &[u8]) -> Result<Self, GalileoError> {
        use image::GenericImageView;
        let decoded = image::load_from_memory(bytes)?;
        let bytes = decoded.to_rgba8();
        let dimensions = decoded.dimensions();

        Ok(Self {
            bytes: bytes.into_vec(),
            dimensions,
        })
    }
}
