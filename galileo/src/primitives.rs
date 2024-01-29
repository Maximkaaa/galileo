use image::GenericImageView;
use std::ops::Deref;

use crate::error::GalileoError;

#[derive(Debug, Clone)]
pub struct DecodedImage {
    pub bytes: Vec<u8>,
    pub dimensions: (u32, u32),
}

impl DecodedImage {
    pub fn new(bytes: &[u8]) -> Result<Self, GalileoError> {
        let decoded = image::load_from_memory(bytes)?;
        let bytes = decoded.to_rgba8();
        let dimensions = decoded.dimensions();

        Ok(Self {
            bytes: Vec::from(bytes.deref()),
            dimensions,
        })
    }
}
