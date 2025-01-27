//! This module contains utilities for loading images to be rendered on the map.

use std::fmt::Formatter;

use galileo_types::cartesian::Size;
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::GalileoError;

/// An image that has been loaded into memory.
#[derive(Debug, Clone)]
pub struct DecodedImage(pub(crate) DecodedImageType);

#[derive(Debug, Clone)]
pub(crate) enum DecodedImageType {
    Bitmap {
        bytes: Vec<u8>,
        dimensions: Size<u32>,
    },
    #[cfg(target_arch = "wasm32")]
    JsImageBitmap(web_sys::ImageBitmap),
}

impl DecodedImage {
    /// Decode an image from a byte slice.
    ///
    /// Attempts to guess the format of the image from the data. Non-RGBA images
    /// will be converted to RGBA.
    #[cfg(feature = "image")]
    pub fn decode(bytes: &[u8]) -> Result<Self, GalileoError> {
        use image::GenericImageView;
        let decoded = image::load_from_memory(bytes).map_err(|_| GalileoError::ImageDecode)?;
        let bytes = decoded.to_rgba8();
        let dimensions = decoded.dimensions();

        Ok(Self(DecodedImageType::Bitmap {
            bytes: bytes.into_vec(),
            dimensions: Size::new(dimensions.0, dimensions.1),
        }))
    }

    /// Create a DecodedImage from a buffer of raw RGBA pixels.
    // #[cfg(not(target_arch = "wasm32"))]
    pub fn from_raw(
        bytes: impl Into<Vec<u8>>,
        dimensions: Size<u32>,
    ) -> Result<Self, GalileoError> {
        let bytes = bytes.into();

        if bytes.len() != 4 * dimensions.width() as usize * dimensions.height() as usize {
            return Err(GalileoError::Generic(
                "invalid image dimensions for buffer size".into(),
            ));
        }

        Ok(Self(DecodedImageType::Bitmap { bytes, dimensions }))
    }

    /// Return width of the image in pixels.
    pub fn width(&self) -> u32 {
        self.0.width()
    }

    /// Return height of the image in pixels.
    pub fn height(&self) -> u32 {
        self.0.height()
    }

    /// Bitmap size in bytes
    pub fn size(&self) -> usize {
        self.width() as usize * self.height() as usize * 4
    }
}

impl DecodedImageType {
    fn width(&self) -> u32 {
        match self {
            DecodedImageType::Bitmap { dimensions, .. } => dimensions.width(),
            #[cfg(target_arch = "wasm32")]
            DecodedImageType::JsImageBitmap(image_bitmap) => image_bitmap.width(),
        }
    }

    fn height(&self) -> u32 {
        match self {
            DecodedImageType::Bitmap { dimensions, .. } => dimensions.height(),
            #[cfg(target_arch = "wasm32")]
            DecodedImageType::JsImageBitmap(image_bitmap) => image_bitmap.height(),
        }
    }
}

#[cfg(feature = "image")]
mod serialization {
    use base64::prelude::BASE64_STANDARD;
    use base64::Engine;
    use image::ImageEncoder;

    use super::*;

    impl Serialize for DecodedImage {
        fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            match &self.0 {
                DecodedImageType::Bitmap { bytes, dimensions } => {
                    use image::codecs::png::PngEncoder;
                    use image::ColorType;

                    let mut encoded = vec![];
                    let encoder = PngEncoder::new(&mut encoded);
                    if let Err(err) = encoder.write_image(
                        bytes,
                        dimensions.width(),
                        dimensions.height(),
                        ColorType::Rgba8,
                    ) {
                        return Err(serde::ser::Error::custom(format!(
                            "failed to encode image to PNG: {err}"
                        )));
                    }

                    let base64 = BASE64_STANDARD.encode(&encoded);

                    _serializer.serialize_str(&base64)
                }
                #[cfg(target_arch = "wasm32")]
                _ => Err(serde::ser::Error::custom(
                    "Serialization is only supported for raw bitmap image type",
                )),
            }
        }
    }

    impl<'de> Deserialize<'de> for DecodedImage {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let visitor = DecodedImageVisitor {};
            deserializer.deserialize_str(visitor)
        }
    }

    struct DecodedImageVisitor {}
    impl Visitor<'_> for DecodedImageVisitor {
        type Value = DecodedImage;

        fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
            formatter.write_str("base64 encoded image")
        }

        fn visit_str<E>(self, _v: &str) -> Result<Self::Value, E>
        where
            E: Error,
        {
            cfg_if::cfg_if! {
                if #[cfg(target_arch = "wasm32")] {
                    panic!("should not be used in WASM");
                } else {
                    let Ok(bytes) = BASE64_STANDARD.decode(_v) else {
                        return Err(Error::custom("not a valid base64 string"));
                    };

                    DecodedImage::decode(&bytes)
                        .map_err(|err| Error::custom(format!("failed to decode image: {err}")))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "image")]
    #[test]
    fn serialize_and_deserialize_decoded_image() {
        const IMAGE: &str = "\"iVBORw0KGgoAAAANSUhEUgAAABgAAAAYCAYAAADgdz34AAAABHNCSVQICAgIfAhkiAAAAAlwSFlzAAAApgAAAKYB3X3/OAAAABl0RVh0U29mdHdhcmUAd3d3Lmlua3NjYXBlLm9yZ5vuPBoAAANCSURBVEiJtZZPbBtFFMZ/M7ubXdtdb1xSFyeilBapySVU8h8OoFaooFSqiihIVIpQBKci6KEg9Q6H9kovIHoCIVQJJCKE1ENFjnAgcaSGC6rEnxBwA04Tx43t2FnvDAfjkNibxgHxnWb2e/u992bee7tCa00YFsffekFY+nUzFtjW0LrvjRXrCDIAaPLlW0nHL0SsZtVoaF98mLrx3pdhOqLtYPHChahZcYYO7KvPFxvRl5XPp1sN3adWiD1ZAqD6XYK1b/dvE5IWryTt2udLFedwc1+9kLp+vbbpoDh+6TklxBeAi9TL0taeWpdmZzQDry0AcO+jQ12RyohqqoYoo8RDwJrU+qXkjWtfi8Xxt58BdQuwQs9qC/afLwCw8tnQbqYAPsgxE1S6F3EAIXux2oQFKm0ihMsOF71dHYx+f3NND68ghCu1YIoePPQN1pGRABkJ6Bus96CutRZMydTl+TvuiRW1m3n0eDl0vRPcEysqdXn+jsQPsrHMquGeXEaY4Yk4wxWcY5V/9scqOMOVUFthatyTy8QyqwZ+kDURKoMWxNKr2EeqVKcTNOajqKoBgOE28U4tdQl5p5bwCw7BWquaZSzAPlwjlithJtp3pTImSqQRrb2Z8PHGigD4RZuNX6JYj6wj7O4TFLbCO/Mn/m8R+h6rYSUb3ekokRY6f/YukArN979jcW+V/S8g0eT/N3VN3kTqWbQ428m9/8k0P/1aIhF36PccEl6EhOcAUCrXKZXXWS3XKd2vc/TRBG9O5ELC17MmWubD2nKhUKZa26Ba2+D3P+4/MNCFwg59oWVeYhkzgN/JDR8deKBoD7Y+ljEjGZ0sosXVTvbc6RHirr2reNy1OXd6pJsQ+gqjk8VWFYmHrwBzW/n+uMPFiRwHB2I7ih8ciHFxIkd/3Omk5tCDV1t+2nNu5sxxpDFNx+huNhVT3/zMDz8usXC3ddaHBj1GHj/As08fwTS7Kt1HBTmyN29vdwAw+/wbwLVOJ3uAD1wi/dUH7Qei66PfyuRj4Ik9is+hglfbkbfR3cnZm7chlUWLdwmprtCohX4HUtlOcQjLYCu+fzGJH2QRKvP3UNz8bWk1qMxjGTOMThZ3kvgLI5AzFfo379UAAAAASUVORK5CYII=\"";
        let deserialized: DecodedImage =
            serde_json::from_str(IMAGE).expect("deserialization failed");
        assert_ne!(deserialized.width(), 0);
        assert_ne!(deserialized.height(), 0);

        let serialized = serde_json::to_string(&deserialized).expect("serialization failed");
        assert!(serialized.starts_with('\"'));
        assert!(serialized.ends_with('\"'));
    }
}
