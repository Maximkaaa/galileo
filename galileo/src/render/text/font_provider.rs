use std::path::PathBuf;
use std::sync::Arc;

use rustybuzz::ttf_parser::{self};

use super::FontProperties;

#[cfg(not(target_arch = "wasm32"))]
mod system;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) type DefaultFontProvider = system::SystemFontProvider;

#[cfg(target_arch = "wasm32")]
mod font_db;
#[cfg(target_arch = "wasm32")]
pub(crate) type DefaultFontProvider = font_db::FontdbFontProvider;

pub trait FontProvider {
    fn best_match(
        &self,
        text: &str,
        font_families: &[String],
        font_properties: FontProperties,
    ) -> Option<(Arc<Vec<u8>>, u32)> {
        log::trace!("Selecting font for text '{text}'");

        let mut candidate: Option<(Arc<Vec<u8>>, u32, usize)> = None;
        let text_len = text.chars().count();

        for first in 0..font_families.len() {
            let Some(loaded) = self.find_best_match(&font_families[first..], &font_properties)
            else {
                continue;
            };

            let Ok(font) = ttf_parser::Face::parse(&loaded.0, loaded.1) else {
                continue;
            };

            let contains_chars = text
                .chars()
                .filter(|c| font.glyph_index(*c).is_some())
                .count();
            if contains_chars > 0 && candidate.is_none()
                || matches!(candidate, Some((_, _, chars_count)) if chars_count < contains_chars)
            {
                log::trace!(
                    "Using latest best match as it has {contains_chars} chars from the text"
                );

                candidate = Some((loaded.0, loaded.1, contains_chars));
                if contains_chars == text_len {
                    break;
                }
            }
        }

        candidate.map(|(data, index, _)| (data, index))
    }

    fn find_best_match(
        &self,
        font_families: &[String],
        font_properties: &FontProperties,
    ) -> Option<(Arc<Vec<u8>>, u32)>;

    fn load_fonts_folder(&self, path: PathBuf);

    fn load_font_data(&self, font_data: Arc<Vec<u8>>);
}
