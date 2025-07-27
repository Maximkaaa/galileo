use std::sync::Arc;

use galileo_types::cartesian::Vector2;
use lyon::lyon_tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, FillVertexConstructor, VertexBuffers,
};
use lyon::path::path::Builder;
use lyon::path::Path;
use lyon::tessellation::{StrokeOptions, StrokeTessellator, StrokeVertexConstructor};
use rustybuzz::ttf_parser::{self, GlyphId, OutlineBuilder, Tag};
use rustybuzz::{Direction, UnicodeBuffer};

use super::font_provider::FontProvider;
use super::text_service::FontServiceError;
use super::{FontProperties, GlyphVertex};
use crate::render::text::{TessellatedGlyph, TextRasterizer, TextShaping, TextStyle};
use crate::Color;

/// Font service provider that uses `rustybuzz` crate to shape and vectorize text
#[derive(Default)]
pub struct RustybuzzRasterizer {}

impl RustybuzzRasterizer {
    fn select_face(
        &self,
        text: &str,
        style: &TextStyle,
        font_provider: &dyn FontProvider,
    ) -> Option<(Arc<Vec<u8>>, u32)> {
        let properties = FontProperties {
            weight: style.weight,
            style: style.style,
        };

        font_provider.best_match(text, &style.font_family, properties)
    }
}

impl TextRasterizer for RustybuzzRasterizer {
    fn shape(
        &self,
        text: &str,
        style: &TextStyle,
        offset: Vector2<f32>,
        font_provider: &dyn FontProvider,
        dpi_scale_factor: f32,
    ) -> Result<TextShaping, FontServiceError> {
        if text.is_empty() {
            return Ok(TextShaping::Tessellation { glyphs: vec![] });
        }

        let mut buffer = UnicodeBuffer::new();
        buffer.push_str(text);
        buffer.guess_segment_properties();

        let Some((font_data, index)) = self.select_face(text, style, font_provider) else {
            return Err(FontServiceError::FontNotFound);
        };

        let face = ttf_parser::Face::parse(&font_data, index)?;
        let mut face = rustybuzz::Face::from_face(face);

        face.set_variation(Tag::from_bytes(b"wght"), style.weight.0 as f32);
        face.set_variation(Tag::from_bytes(b"wdth"), 1.0);

        let units = face.units_per_em() as f32;
        let scale = style.font_size / units * dpi_scale_factor;

        let is_vertical = matches!(
            buffer.direction(),
            Direction::TopToBottom | Direction::BottomToTop
        );
        let glyph_buffer = rustybuzz::shape(&face, &[], buffer);
        let mut fill = vec![];
        let mut outline = vec![];

        let (width, height) = if is_vertical {
            let width = face.units_per_em();
            let height = glyph_buffer
                .glyph_positions()
                .iter()
                .fold(0, |aggr, glyph| aggr + glyph.y_advance);
            (width as f32, height as f32)
        } else {
            let width = glyph_buffer
                .glyph_positions()
                .iter()
                .fold(0, |aggr, glyph| aggr + glyph.x_advance);
            let height = face.ascender() + face.descender();
            (width as f32, height as f32)
        };

        let width = width * scale;
        let height = height * scale;

        let offset_x = offset.dx()
            + match style.horizontal_alignment {
                super::HorizontalAlignment::Left => 0.0,
                super::HorizontalAlignment::Center => -width / 2.0,
                super::HorizontalAlignment::Right => -width,
            };

        let offset_y = offset.dy()
            + match style.vertical_alignment {
                super::VerticalAlignment::Top => -height,
                super::VerticalAlignment::Middle => -height / 2.0,
                super::VerticalAlignment::Bottom => 0.0,
            };

        let mut advance_x = 0.0;
        let mut advance_y = 0.0;

        for index in 0..glyph_buffer.len() {
            let position = glyph_buffer.glyph_positions()[index];
            let glyph_info = glyph_buffer.glyph_infos()[index];

            let mut path_builder = GlyphPathBuilder::new(scale);
            face.outline_glyph(GlyphId(glyph_info.glyph_id as u16), &mut path_builder);

            let snapped_x = (position.x_offset as f32 * scale + advance_x + offset_x).round();
            let snapped_y = (position.y_offset as f32 * scale + advance_y + offset_y).round();

            let glyph_position = Vector2::new(snapped_x, snapped_y);

            if style.outline_width > 0.0 && !style.outline_color.is_transparent() {
                outline.push(path_builder.clone().tessellate_outline(
                    glyph_position,
                    style.outline_width,
                    style.outline_color,
                ));
            }

            fill.push(path_builder.tessellate_fill(glyph_position, style.font_color));

            advance_x += position.x_advance as f32 * scale;
            advance_y += position.y_advance as f32 * scale;
        }

        outline.append(&mut fill);

        Ok(TextShaping::Tessellation { glyphs: outline })
    }
}

#[derive(Clone)]
struct GlyphPathBuilder {
    builder: Builder,
    scale: f32,
}

impl GlyphPathBuilder {
    fn tessellate_fill(self, offset: Vector2<f32>, color: Color) -> TessellatedGlyph {
        let vertex_constructor = GlyphVertexConstructor { offset, color };
        let mut tessellator = FillTessellator::new();
        let mut buffers: VertexBuffers<GlyphVertex, u32> = VertexBuffers::new();
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

    fn tessellate_outline(
        self,
        offset: Vector2<f32>,
        width: f32,
        color: Color,
    ) -> TessellatedGlyph {
        let vertex_constructor = GlyphVertexConstructor { offset, color };
        let mut tessellator = StrokeTessellator::new();
        let mut buffers: VertexBuffers<GlyphVertex, u32> = VertexBuffers::new();
        if tessellator
            .tessellate(
                &self.builder.build(),
                &StrokeOptions::default().with_line_width(width),
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
    color: Color,
}

impl FillVertexConstructor<GlyphVertex> for GlyphVertexConstructor {
    fn new_vertex(&mut self, vertex: FillVertex) -> GlyphVertex {
        GlyphVertex {
            position: [
                vertex.position().x + self.offset.dx(),
                vertex.position().y + self.offset.dy(),
            ],
            color: self.color,
        }
    }
}

impl StrokeVertexConstructor<GlyphVertex> for GlyphVertexConstructor {
    fn new_vertex(&mut self, vertex: lyon::tessellation::StrokeVertex) -> GlyphVertex {
        GlyphVertex {
            position: [
                vertex.position().x + self.offset.dx(),
                vertex.position().y + self.offset.dy(),
            ],
            color: self.color,
        }
    }
}
