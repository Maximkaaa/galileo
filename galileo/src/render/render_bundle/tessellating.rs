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
use lyon::path::{EndpointId, Path};
use lyon::tessellation::VertexSource;
use num_traits::AsPrimitive;
use serde::{Deserialize, Serialize};
use std::ops::Range;

#[derive(Debug)]
pub struct TessellatingRenderBundle {
    pub poly_tessellation: Vec<LodTessellation>,
    pub points: Vec<PointInstance>,
    pub images: Vec<(DecodedImage, [ImageVertex; 4])>,
    pub primitives: Vec<PrimitiveInfo>,
}

#[derive(Debug)]
pub struct LodTessellation {
    pub min_resolution: f32,
    pub tessellation: VertexBuffers<PolyVertex, u32>,
}

#[derive(Debug)]
pub enum PrimitiveInfo {
    Poly { vertex_ranges: Vec<Range<usize>> },
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
            poly_tessellation: vec![LodTessellation {
                min_resolution: 1.0,
                tessellation: VertexBuffers::new(),
            }],
            points: Vec::new(),
            images: Vec::new(),
            primitives: Vec::new(),
        }
    }

    pub fn with_lods(lods: &[f32]) -> Self {
        Self {
            poly_tessellation: lods
                .iter()
                .map(|&min_resolution| LodTessellation {
                    min_resolution,
                    tessellation: VertexBuffers::new(),
                })
                .collect(),
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

    pub fn add_line<N, P, C>(&mut self, line: &C, paint: LinePaint) -> PrimitiveId
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
        C: Contour<Point = P>,
    {
        let mut ranges = vec![];
        for index in 0..self.poly_tessellation.len() {
            ranges.push(self.add_line_lod(line, paint, index));
        }

        let id = self.primitives.len();
        self.primitives.push(PrimitiveInfo::Poly {
            vertex_ranges: ranges,
        });

        PrimitiveId(id)
    }

    pub fn add_line_lod<N, P, C>(
        &mut self,
        line: &C,
        paint: LinePaint,
        lod_index: usize,
    ) -> Range<usize>
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
        C: Contour<Point = P>,
    {
        let lod = &mut self.poly_tessellation[lod_index];
        let mut path_builder = BuilderWithAttributes::new(1);
        let mut iterator = line.iter_points();

        let Some(first_point) = iterator.next() else {
            return 0..0;
        };

        let _ = path_builder.begin(
            point(
                first_point.x().as_() / lod.min_resolution,
                first_point.y().as_() / lod.min_resolution,
            ),
            &[first_point.z().as_()],
        );

        for p in iterator {
            let _ = path_builder.line_to(
                point(
                    p.x().as_() / lod.min_resolution,
                    p.y().as_() / lod.min_resolution,
                ),
                &[p.z().as_()],
            );
        }

        path_builder.end(line.is_closed());
        let path = path_builder.build();

        let vertex_constructor = LineVertexConstructor {
            width: paint.width as f32,
            offset: paint.offset as f32,
            color: paint.color.to_f32_array(),
            resolution: lod.min_resolution,
            path: &path,
        };

        let mut tesselator = StrokeTessellator::new();
        let start_index = lod.tessellation.vertices.len();

        tesselator
            .tessellate_path(
                &path,
                &StrokeOptions::DEFAULT
                    .with_line_cap(paint.line_cap.into())
                    .with_line_width(paint.width as f32)
                    .with_miter_limit(1.0)
                    .with_tolerance(0.1)
                    .with_line_join(LineJoin::Round),
                &mut BuffersBuilder::new(&mut lod.tessellation, vertex_constructor),
            )
            .unwrap();

        let end_index = lod.tessellation.vertices.len();
        start_index..end_index
    }

    pub fn add_polygon<N, P, Poly>(&mut self, polygon: &Poly, paint: Paint) -> PrimitiveId
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
        Poly: Polygon,
        Poly::Contour: Contour<Point = P>,
    {
        let mut ranges = vec![];
        for lod_index in 0..self.poly_tessellation.len() {
            ranges.push(self.add_polygon_lod(polygon, paint, lod_index));
        }

        let id = self.primitives.len();
        self.primitives.push(PrimitiveInfo::Poly {
            vertex_ranges: ranges,
        });

        PrimitiveId(id)
    }

    fn add_polygon_lod<N, P, Poly>(
        &mut self,
        polygon: &Poly,
        paint: Paint,
        lod_index: usize,
    ) -> Range<usize>
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
        Poly: Polygon,
        Poly::Contour: Contour<Point = P>,
    {
        let lod = &mut self.poly_tessellation[lod_index];
        let mut path_builder = BuilderWithAttributes::new(1);
        for contour in polygon.iter_contours() {
            let mut iterator = contour.iter_points();

            if let Some(first_point) = iterator.next() {
                let _ = path_builder.begin(
                    point(first_point.x().as_(), first_point.y().as_()),
                    &[first_point.z().as_()],
                );
            } else {
                return 0..0;
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
        let start_index = lod.tessellation.vertices.len();
        let tolerance = match lod.min_resolution / 10.0 {
            v if v.is_finite() => v,
            _ => f32::MIN,
        };

        tesselator
            .tessellate(
                &path,
                &FillOptions::DEFAULT.with_tolerance(tolerance),
                &mut BuffersBuilder::new(&mut lod.tessellation, vertex_constructor),
            )
            .unwrap();

        let end_index = lod.tessellation.vertices.len();
        start_index..end_index
    }

    pub fn is_empty(&self) -> bool {
        self.primitives.is_empty()
    }
}

#[allow(dead_code)]
struct LineVertexConstructor<'a> {
    width: f32,
    offset: f32,
    color: [f32; 4],
    resolution: f32,
    path: &'a Path,
}

impl<'a> StrokeVertexConstructor<PolyVertex> for LineVertexConstructor<'a> {
    fn new_vertex(&mut self, mut vertex: StrokeVertex) -> PolyVertex {
        let position = vertex.position_on_path();
        let normal = match vertex.side() {
            Side::Negative => [
                vertex.normal().x * (vertex.line_width() / 2.0 - self.offset),
                vertex.normal().y * (vertex.line_width() / 2.0 - self.offset),
            ],
            Side::Positive => [
                vertex.normal().x * (vertex.line_width() / 2.0 + self.offset),
                vertex.normal().y * (vertex.line_width() / 2.0 + self.offset),
            ],
        };

        let norm_limit_sq = if let VertexSource::Endpoint { id } = vertex.source() {
            let mut prev_id = id.0.saturating_sub(1);
            while self.path[EndpointId(prev_id)] == Default::default() && prev_id > 0 {
                prev_id -= 1;
            }

            if prev_id != 0 {
                let prev_id = EndpointId(prev_id);
                let from = self.path[prev_id];
                let to = self.path[id];
                let dx = from.x - to.x;
                let dy = from.y - to.y;
                (dx * dx + dy * dy).sqrt() * 2.0 * self.resolution
            } else {
                self.width
            }
        } else {
            self.width
        };

        PolyVertex {
            position: [
                position.x * self.resolution,
                position.y * self.resolution,
                vertex.interpolated_attributes()[0],
            ],
            color: self.color,
            normal,
            norm_limit_sq,
        }
    }
}

struct PolygonVertexConstructor {
    color: [f32; 4],
}

impl FillVertexConstructor<PolyVertex> for PolygonVertexConstructor {
    fn new_vertex(&mut self, vertex: FillVertex) -> PolyVertex {
        PolyVertex {
            position: [vertex.position().x, vertex.position().y, 0.0],
            color: self.color,
            normal: Default::default(),
            norm_limit_sq: 1.0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Serialize, Deserialize)]
pub struct PolyVertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
    pub normal: [f32; 2],
    pub norm_limit_sq: f32,
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
