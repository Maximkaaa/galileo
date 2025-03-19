use std::sync::Arc;

use fontdb::{Database, Family, Query, Source};
use parking_lot::Mutex;

use super::FontProvider;
use crate::render::text::{FontStyle, FontWeight};

pub(crate) struct FontdbFontProvider {
    db: Mutex<Database>,
}

impl FontdbFontProvider {
    pub fn new() -> Self {
        Self {
            db: Mutex::new(Database::new()),
        }
    }
}

impl FontProvider for FontdbFontProvider {
    fn find_best_match(
        &self,
        font_families: &[String],
        font_properties: &crate::render::text::FontProperties,
    ) -> Option<(std::sync::Arc<Vec<u8>>, u32)> {
        let families: Vec<_> = font_families.iter().map(|f| Family::Name(f)).collect();
        let query = Query {
            families: &families,
            weight: font_properties.weight.into(),
            stretch: Default::default(),
            style: font_properties.style.into(),
        };

        let db = self.db.lock();
        let id = db.query(&query)?;
        let (source, index) = db.face_source(id)?;

        match source {
            Source::Binary(data) => Some((Arc::new((*data).as_ref().to_vec()), index)),
        }
    }

    fn load_fonts_folder(&self, _path: std::path::PathBuf) {
        log::error!("Fontdb provider doesn't support FS operations");
    }

    fn load_font_data(&self, font_data: std::sync::Arc<Vec<u8>>) {
        self.db.lock().load_font_data(font_data.to_vec());
    }
}

impl From<FontStyle> for fontdb::Style {
    fn from(value: FontStyle) -> Self {
        match value {
            FontStyle::Normal => Self::Normal,
            FontStyle::Italic => Self::Italic,
            FontStyle::Oblique => Self::Oblique,
        }
    }
}

impl From<FontWeight> for fontdb::Weight {
    fn from(value: FontWeight) -> Self {
        Self(value.0)
    }
}
