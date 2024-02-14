#[cfg(not(target_arch = "wasm32"))]
use crate::error::GalileoError;
#[cfg(not(target_arch = "wasm32"))]
use std::ops::Deref;

#[derive(Debug, Clone)]
pub struct DecodedImage {
    pub bytes: Vec<u8>,
    pub dimensions: (u32, u32),
}

impl DecodedImage {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(bytes: &[u8]) -> Result<Self, GalileoError> {
        use image::GenericImageView;
        let decoded = image::load_from_memory(bytes)?;
        let bytes = decoded.to_rgba8();
        let dimensions = decoded.dimensions();

        Ok(Self {
            bytes: Vec::from(bytes.deref()),
            dimensions,
        })
    }
}
