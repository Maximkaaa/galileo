use std::path::PathBuf;
use std::sync::Arc;

use ahash::{HashMap, HashMapExt};
use font_kit::family_name::FamilyName;
use font_kit::handle::Handle;
use font_kit::properties::{Properties, Style, Weight};
use font_kit::source::Source;
use font_kit::sources::fs::FsSource;
use font_kit::sources::mem::MemSource;
use font_kit::sources::multi::MultiSource;
use parking_lot::{Mutex, RwLock};
use rustybuzz::ttf_parser::fonts_in_collection;

use super::FontProvider;
use crate::render::text::{FontProperties, FontStyle, FontWeight};

pub(crate) struct SystemFontProvider {
    source: Mutex<MultiSource>,
    loaded_fonts: RwLock<HashMap<PathBuf, Arc<Vec<u8>>>>,
}

unsafe impl Send for SystemFontProvider {}
unsafe impl Sync for SystemFontProvider {}

impl SystemFontProvider {
    pub(crate) fn new() -> Self {
        let sources: Vec<Box<dyn Source>> = vec![Box::new(MemSource::empty())];

        Self {
            source: Mutex::new(MultiSource::from_sources(sources)),
            loaded_fonts: RwLock::new(HashMap::new()),
        }
    }

    fn load_font(&self, handle: Handle) -> Option<(Arc<Vec<u8>>, u32)> {
        match handle {
            Handle::Path { path, font_index } => {
                if let Some(loaded) = self.loaded_fonts.read().get(&path) {
                    return Some((loaded.clone(), font_index));
                }

                let data = Arc::new(std::fs::read(&path).ok()?);
                self.loaded_fonts.write().insert(path, data.clone());

                Some((data, font_index))
            }
            Handle::Memory { bytes, font_index } => Some((bytes, font_index)),
        }
    }
}

impl FontProvider for SystemFontProvider {
    fn find_best_match(
        &self,
        font_families: &[String],
        font_properties: &FontProperties,
    ) -> Option<(Arc<Vec<u8>>, u32)> {
        let families: Vec<_> = font_families
            .iter()
            .map(|f| FamilyName::Title(f.to_string()))
            .collect();

        let properties = Properties {
            style: font_properties.style.into(),
            weight: font_properties.weight.into(),
            stretch: Default::default(),
        };
        let selected = self
            .source
            .lock()
            .select_best_match(&families, &properties)
            .ok()?;

        self.load_font(selected)
    }

    fn load_fonts_folder(&self, path: PathBuf) {
        let fs_source = FsSource::in_path(path);
        let mut source = self.source.lock();
        let prev_source = std::mem::replace(&mut *source, MultiSource::from_sources(vec![]));
        *source = MultiSource::from_sources(vec![
            Box::new(prev_source),
            Box::new(fs_source),
            Box::new(MemSource::empty()),
        ]);
    }

    fn load_font_data(&self, font_data: Arc<Vec<u8>>) {
        let mut source = self.source.lock();
        let Some(mem_source) = source.find_source_mut::<MemSource>() else {
            log::error!("Memory font source is not in the MemSource. Font is not loaded");
            return;
        };

        let face_count = fonts_in_collection(&font_data).unwrap_or(1);
        for font_index in 0..face_count {
            match mem_source.add_font(Handle::Memory {
                bytes: font_data.clone(),
                font_index,
            }) {
                Ok(face) => log::debug!("Loaded font: {}", face.full_name()),
                Err(err) => log::warn!("Failed to load face with index {font_index}: {err}"),
            }
        }
    }
}

impl From<FontStyle> for Style {
    fn from(value: FontStyle) -> Self {
        match value {
            FontStyle::Normal => Self::Normal,
            FontStyle::Italic => Self::Italic,
            FontStyle::Oblique => Self::Oblique,
        }
    }
}

impl From<FontWeight> for Weight {
    fn from(value: FontWeight) -> Self {
        Self(value.0 as f32)
    }
}
