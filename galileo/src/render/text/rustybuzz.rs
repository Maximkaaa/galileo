use bytes::Bytes;
use lyon::lyon_tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, FillVertexConstructor, VertexBuffers,
};
use lyon::path::path::Builder;
use lyon::path::Path;
use nalgebra::Vector2;
use rustybuzz::ttf_parser::{GlyphId, OutlineBuilder};
use rustybuzz::{Face, UnicodeBuffer};

use crate::render::text::font_service::FontServiceError;
use crate::render::text::{FontServiceProvider, TessellatedGlyph, TextShaping, TextStyle};

#[derive(Default)]
pub struct RustybuzzFontServiceProvider {
    fonts_data: Vec<Bytes>,
}

impl RustybuzzFontServiceProvider {
    fn select_face(&self, _buffer: &UnicodeBuffer) -> Option<Face<'_>> {
        // todo
        let fonts_data = self.fonts_data.first()?;
        Face::from_slice(fonts_data, 0)
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

        let Some(face) = self.select_face(&buffer) else {
            return Err(FontServiceError::FontNotFound);
        };

        let units = face.units_per_em() as f32;
        let scale = style.font_size / units;

        let glyph_buffer = rustybuzz::shape(&face, &[], buffer);
        let mut tessellations = vec![];

        let mut advance_x = 0;
        let mut advance_y = 0;
        for index in 0..glyph_buffer.len() {
            let position = glyph_buffer.glyph_positions()[index];
            let glyph_info = glyph_buffer.glyph_infos()[index];

            let mut path_builder = GlyphPathBuilder::new(scale);
            face.outline_glyph(GlyphId(glyph_info.glyph_id as u16), &mut path_builder);
            tessellations.push(path_builder.tessellate(Vector2::new(
                offset.x + (position.x_offset + advance_x) as f32 * scale,
                offset.y + (position.y_offset + advance_y) as f32 * scale,
            )));

            advance_x += position.x_advance;
            advance_y += position.y_advance;
        }

        Ok(TextShaping::Tessellation {
            glyphs: tessellations,
        })
    }

    fn load_fonts(&mut self, fonts_data: Bytes) -> Result<(), FontServiceError> {
        self.fonts_data.push(fonts_data);
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
                &FillOptions::default(),
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
