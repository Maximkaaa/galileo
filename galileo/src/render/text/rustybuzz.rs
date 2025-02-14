use ahash::HashMap;
use bytes::Bytes;
use font_query::{Database, Query, Stretch, Style, Weight, ID};
use lyon::lyon_tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, FillVertexConstructor, VertexBuffers,
};
use lyon::path::path::Builder;
use lyon::path::Path;
use nalgebra::Vector2;
use owned_ttf_parser::{AsFaceRef, OwnedFace};
use rustybuzz::ttf_parser::{GlyphId, OutlineBuilder, Tag};
use rustybuzz::UnicodeBuffer;

use crate::render::text::font_service::FontServiceError;
use crate::render::text::{FontServiceProvider, TessellatedGlyph, TextShaping, TextStyle};

/// Font service provider that uses `rustybuzz` crate to shape and vectorize text
#[derive(Default)]
pub struct RustybuzzFontServiceProvider {
    font_db: Database,
    loaded_faces: HashMap<ID, OwnedFace>,
}

impl RustybuzzFontServiceProvider {
    fn select_face(&self, text: &str, style: &TextStyle) -> Option<&OwnedFace> {
        let query = Query {
            families: &style
                .font_family
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            weight: Weight::NORMAL,
            stretch: Stretch::Normal,
            style: Style::Normal,
        };

        let matches = self.font_db.query(&query);

        let mut last_face = None;
        for face_id in matches {
            let Some(face) = self.loaded_faces.get(&face_id) else {
                continue;
            };

            let Some(first_char) = text.chars().next() else {
                return Some(face);
            };

            if face.as_face_ref().glyph_index(first_char).is_some() {
                return Some(face);
            } else {
                last_face = Some(face);
            }
        }

        last_face
    }
}

impl FontServiceProvider for RustybuzzFontServiceProvider {
    fn shape(
        &self,
        text: &str,
        style: &TextStyle,
        offset: Vector2<f32>,
    ) -> Result<TextShaping, FontServiceError> {
        let mut buffer = UnicodeBuffer::new();
        buffer.push_str(text);
        buffer.guess_segment_properties();

        let Some(face) = self.select_face(text, style) else {
            return Err(FontServiceError::FontNotFound);
        };

        let mut face = rustybuzz::Face::from_face(face.as_face_ref().clone());

        face.set_variation(Tag::from_bytes(b"wght"), 400.0);
        face.set_variation(Tag::from_bytes(b"wdth"), 1.0);

        let units = face.units_per_em() as f32;
        let scale = style.font_size / units;

        let glyph_buffer = rustybuzz::shape(&face, &[], buffer);
        let mut tessellations = vec![];

        let mut advance_x = 0.0;
        let mut advance_y = 0.0;

        for index in 0..glyph_buffer.len() {
            let position = glyph_buffer.glyph_positions()[index];
            let glyph_info = glyph_buffer.glyph_infos()[index];

            let mut path_builder = GlyphPathBuilder::new(scale);
            face.outline_glyph(GlyphId(glyph_info.glyph_id as u16), &mut path_builder);

            let snapped_x = (position.x_offset as f32 * scale + advance_x).round();
            let snapped_y = (position.y_offset as f32 * scale + advance_y).round();
            tessellations.push(
                path_builder.tessellate(Vector2::new(offset.x + snapped_x, offset.y + snapped_y)),
            );

            advance_x += position.x_advance as f32 * scale;
            advance_y += position.y_advance as f32 * scale;
        }

        Ok(TextShaping::Tessellation {
            glyphs: tessellations,
        })
    }

    fn load_fonts(&mut self, fonts_data: Bytes) -> Result<(), FontServiceError> {
        let face_ids = self.font_db.load_font_data(fonts_data.to_vec());

        for face_index in 0..face_ids.len() {
            let face = OwnedFace::from_vec(fonts_data.to_vec(), face_index as u32)?;
            self.loaded_faces.insert(face_ids[face_index], face);
        }

        Ok(())
    }
}

struct GlyphPathBuilder {
    builder: Builder,
    scale: f32,
}

impl GlyphPathBuilder {
    fn tessellate(self, offset: Vector2<f32>) -> TessellatedGlyph {
        let vertex_constructor = GlyphVertexConstructor { offset };
        let mut tessellator = FillTessellator::new();
        let mut buffers: VertexBuffers<[f32; 2], u32> = VertexBuffers::new();
        if tessellator
            .tessellate(
                &self.builder.build(),
                &FillOptions::default().with_fill_rule(lyon::path::FillRule::NonZero),
                &mut BuffersBuilder::new(&mut buffers, vertex_constructor),
            )
            .is_ok()
        {
            TessellatedGlyph {
                vertices: buffers.vertices,
                indices: buffers.indices,
            }
        } else {
            invalid_glyph_substitution()
        }
    }
}

fn invalid_glyph_substitution() -> TessellatedGlyph {
    todo!()
}

impl GlyphPathBuilder {
    fn new(scale: f32) -> Self {
        Self {
            scale,
            builder: Path::builder(),
        }
    }
}

impl OutlineBuilder for GlyphPathBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.builder
            .begin(lyon::geom::point(x * self.scale, y * self.scale));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.builder
            .line_to(lyon::geom::point(x * self.scale, y * self.scale));
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.builder.quadratic_bezier_to(
            lyon::geom::point(x1 * self.scale, y1 * self.scale),
            lyon::geom::point(x * self.scale, y * self.scale),
        );
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.builder.cubic_bezier_to(
            lyon::geom::point(x1 * self.scale, y1 * self.scale),
            lyon::geom::point(x2 * self.scale, y2 * self.scale),
            lyon::geom::point(x * self.scale, y * self.scale),
        );
    }

    fn close(&mut self) {
        self.builder.end(true);
    }
}

struct GlyphVertexConstructor {
    offset: Vector2<f32>,
}

impl FillVertexConstructor<[f32; 2]> for GlyphVertexConstructor {
    fn new_vertex(&mut self, vertex: FillVertex) -> [f32; 2] {
        [
            vertex.position().x + self.offset.x,
            vertex.position().y + self.offset.y,
        ]
    }
}
