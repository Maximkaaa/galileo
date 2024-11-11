// Left this file here temporarily until figure out finally how to do font discovery and
// substitution.

use cosmic_text::rustybuzz::ttf_parser::{GlyphId, OutlineBuilder};
use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping};
use lyon::lyon_tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, FillVertexConstructor, VertexBuffers,
};
use lyon::path::path::Builder;
use lyon::path::Path;
use nalgebra::Vector2;

use crate::render::text::font_service::FontServiceError;
use crate::render::text::{FontServiceProvider, TessellatedGlyph, TextShaping, TextStyle};

pub struct CosmicTextProvider {
    font_system: FontSystem,
}

impl CosmicTextProvider {
    pub fn new() -> Self {
        Self {
            font_system: FontSystem::new(),
        }
    }
}

impl FontServiceProvider for CosmicTextProvider {
    fn shape(
        &mut self,
        text: &str,
        style: &TextStyle,
        offset: Vector2<f32>,
    ) -> Result<TextShaping, FontServiceError> {
        let metrics = Metrics::new(style.font_size, style.font_size);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);

        let attrs = Attrs::new();
        let attrs = attrs.family(Family::Name(&style.font_name));

        // This will hang debug build for 40 seconds: "包头市 ᠪᠤᠭᠤᠲᠤ"

        buffer
            .borrow_with(&mut self.font_system)
            .set_text(text, attrs, Shaping::Advanced);
        buffer
            .borrow_with(&mut self.font_system)
            .set_size(Some(2048.0), Some(2048.0));

        let mut tessellations = vec![];
        for run in buffer.layout_runs() {
            for glyph in run.glyphs.iter() {
                let Some(font) = self.font_system.get_font(glyph.font_id) else {
                    return Err(FontServiceError::FontNotFound);
                };
                let face = font.rustybuzz();
                let units = face.units_per_em() as f32;
                let scale = style.font_size / units;

                let mut path_builder = GlyphPathBuilder::new(scale);
                face.outline_glyph(GlyphId(glyph.glyph_id), &mut path_builder);
                tessellations.push(
                    path_builder.tessellate(Vector2::new(glyph.x + offset.x, glyph.y + offset.y)),
                );
            }
        }

        Ok(TextShaping::Tessellation {
            glyphs: tessellations,
        })
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
        if let Ok(_) = tessellator.tessellate(
            &self.builder.build(),
            &FillOptions::default(),
            &mut BuffersBuilder::new(&mut buffers, vertex_constructor),
        ) {
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
