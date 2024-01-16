use crate::primitives::DecodedImage;
use crate::render::{ImagePaint, LinePaint, Paint, PointPaint, PrimitiveId};
use galileo_types::cartesian::impls::point::Point2d;
use galileo_types::cartesian::traits::cartesian_point::{CartesianPoint2d, CartesianPoint3d};
use galileo_types::contour::Contour;
use galileo_types::polygon::Polygon;
use lyon::lyon_tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, FillVertexConstructor, LineJoin,
    Side, StrokeOptions, StrokeTessellator, StrokeVertex, StrokeVertexConstructor, VertexBuffers,
};
use lyon::math::point;
use lyon::path::path::BuilderWithAttributes;
use num_traits::AsPrimitive;
use serde::{Deserialize, Serialize};
use std::ops::Range;

#[derive(Debug)]
pub struct TessellatingRenderBundle {
    pub vertex_buffers: VertexBuffers<LineVertex, u32>,
    pub points: Vec<PointInstance>,
    pub images: Vec<(DecodedImage, [ImageVertex; 4])>,
    pub primitives: Vec<PrimitiveInfo>,
}

#[derive(Debug)]
pub enum PrimitiveInfo {
    Line { vertex_range: Range<usize> },
    Point { point_index: usize },
    Image { image_index: usize },
}

impl Default for TessellatingRenderBundle {
    fn default() -> Self {
        Self::new()
    }
}

impl TessellatingRenderBundle {
    pub fn new() -> Self {
        Self {
            vertex_buffers: VertexBuffers::new(),
            points: Vec::new(),
            images: Vec::new(),
            primitives: Vec::new(),
        }
    }
}

impl TessellatingRenderBundle {
    pub fn add_image(
        &mut self,
        image: DecodedImage,
        vertices: [Point2d; 4],
        paint: ImagePaint,
    ) -> PrimitiveId {
        let opacity = paint.opacity as f32 / 255.0;
        let image_index = self.images.len();
        self.images.push((
            image,
            [
                ImageVertex {
                    position: [vertices[0].x() as f32, vertices[0].y() as f32],
                    opacity,
                    tex_coords: [0.0, 1.0],
                },
                ImageVertex {
                    position: [vertices[1].x() as f32, vertices[1].y() as f32],
                    opacity,
                    tex_coords: [0.0, 0.0],
                },
                ImageVertex {
                    position: [vertices[3].x() as f32, vertices[3].y() as f32],
                    opacity,
                    tex_coords: [1.0, 1.0],
                },
                ImageVertex {
                    position: [vertices[2].x() as f32, vertices[2].y() as f32],
                    opacity,
                    tex_coords: [1.0, 0.0],
                },
            ],
        ));

        let id = self.primitives.len();
        self.primitives.push(PrimitiveInfo::Image { image_index });

        PrimitiveId(id)
    }

    pub fn add_point<N, P>(&mut self, point: &P, paint: PointPaint) -> PrimitiveId
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
    {
        let id = self.primitives.len();
        let index = self.points.len();
        self.points.push(PointInstance {
            position: [point.x().as_(), point.y().as_(), point.z().as_()],
            size: paint.size as f32,
            color: paint.color.to_f32_array(),
        });

        self.primitives
            .push(PrimitiveInfo::Point { point_index: index });
        PrimitiveId(id)
    }

    pub fn add_line<N, P, C>(&mut self, line: &C, paint: LinePaint, resolution: f64) -> PrimitiveId
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
        C: Contour<Point = P>,
    {
        let resolution = resolution as f32;
        let vertex_constructor = LineVertexConstructor {
            width: paint.width as f32,
            offset: paint.offset as f32,
            color: paint.color.to_f32_array(),
            resolution,
        };

        let mut path_builder = BuilderWithAttributes::new(1);
        let mut iterator = line.iter_points();

        if let Some(first_point) = iterator.next() {
            let _ = path_builder.begin(
                point(
                    first_point.x().as_() / resolution,
                    first_point.y().as_() / resolution,
                ),
                &[first_point.z().as_()],
            );
        } else {
            return PrimitiveId::INVALID;
        }

        for p in iterator {
            let _ = path_builder.line_to(
                point(p.x().as_() / resolution, p.y().as_() / resolution),
                &[p.z().as_()],
            );
        }
        path_builder.end(line.is_closed());
        let path = path_builder.build();

        let mut tesselator = StrokeTessellator::new();
        let start_index = self.vertex_buffers.vertices.len();
        tesselator
            .tessellate_path(
                &path,
                &StrokeOptions::DEFAULT
                    .with_line_cap(paint.line_cap.into())
                    .with_line_width(paint.width as f32)
                    .with_miter_limit(2.0)
                    .with_tolerance(0.1)
                    .with_line_join(LineJoin::MiterClip),
                &mut BuffersBuilder::new(&mut self.vertex_buffers, vertex_constructor),
            )
            .unwrap();

        let end_index = self.vertex_buffers.vertices.len();
        let id = self.primitives.len();

        self.primitives.push(PrimitiveInfo::Line {
            vertex_range: start_index..end_index,
        });

        PrimitiveId(id)
    }

    pub fn add_polygon<N, P, Poly>(
        &mut self,
        polygon: &Poly,
        paint: Paint,
        _resolution: f64,
    ) -> PrimitiveId
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
        Poly: Polygon,
        Poly::Contour: Contour<Point = P>,
    {
        let mut path_builder = BuilderWithAttributes::new(1);
        for contour in polygon.iter_contours() {
            let mut iterator = contour.iter_points();

            if let Some(first_point) = iterator.next() {
                let _ = path_builder.begin(
                    point(first_point.x().as_(), first_point.y().as_()),
                    &[first_point.z().as_()],
                );
            } else {
                return PrimitiveId::INVALID;
            }

            for p in iterator {
                let _ = path_builder.line_to(point(p.x().as_(), p.y().as_()), &[p.z().as_()]);
            }

            path_builder.end(true);
        }

        let path = path_builder.build();

        let vertex_constructor = PolygonVertexConstructor {
            color: paint.color.to_f32_array(),
        };
        let mut tesselator = FillTessellator::new();
        let start_index = self.vertex_buffers.vertices.len();
        tesselator
            .tessellate(
                &path,
                &FillOptions::DEFAULT,
                &mut BuffersBuilder::new(&mut self.vertex_buffers, vertex_constructor),
            )
            .unwrap();

        let end_index = self.vertex_buffers.vertices.len();
        let id = self.primitives.len();

        self.primitives.push(PrimitiveInfo::Line {
            vertex_range: start_index..end_index,
        });

        PrimitiveId(id)
    }

    pub fn is_empty(&self) -> bool {
        self.primitives.is_empty()
    }
}

#[allow(dead_code)]
struct LineVertexConstructor {
    width: f32,
    offset: f32,
    color: [f32; 4],
    resolution: f32,
}

impl StrokeVertexConstructor<LineVertex> for LineVertexConstructor {
    fn new_vertex(&mut self, mut vertex: StrokeVertex) -> LineVertex {
        let position = vertex.position_on_path();
        let normal = match vertex.side() {
            Side::Negative => [
                vertex.normal().x * (vertex.line_width() - self.offset * 2.0),
                vertex.normal().y * (vertex.line_width() - self.offset * 2.0),
            ],
            Side::Positive => [
                vertex.normal().x * (vertex.line_width() + self.offset * 2.0),
                vertex.normal().y * (vertex.line_width() + self.offset * 2.0),
            ],
        };
        LineVertex {
            position: [
                position.x * self.resolution,
                position.y * self.resolution,
                vertex.interpolated_attributes()[0],
            ],
            color: self.color,
            normal,
        }
    }
}

struct PolygonVertexConstructor {
    color: [f32; 4],
}

impl FillVertexConstructor<LineVertex> for PolygonVertexConstructor {
    fn new_vertex(&mut self, vertex: FillVertex) -> LineVertex {
        LineVertex {
            position: [vertex.position().x, vertex.position().y, 0.0],
            color: self.color,
            normal: Default::default(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Serialize, Deserialize)]
pub struct LineVertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
    pub normal: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PointInstance {
    pub position: [f32; 3],
    pub size: f32,
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ImageVertex {
    pub position: [f32; 2],
    pub opacity: f32,
    pub tex_coords: [f32; 2],
}
