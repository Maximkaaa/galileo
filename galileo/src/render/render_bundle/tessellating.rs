use crate::primitives::DecodedImage;
use crate::render::point_paint::{CircleFill, PointPaint, PointShape, SectorParameters};
use crate::render::{ImagePaint, LinePaint, PolygonPaint, PrimitiveId};
use crate::Color;
use galileo_types::cartesian::impls::contour::ClosedContour;
use galileo_types::cartesian::impls::point::Point2d;
use galileo_types::cartesian::traits::cartesian_point::{CartesianPoint2d, CartesianPoint3d};
use galileo_types::contour::Contour;
use galileo_types::polygon::Polygon;
use lyon::lyon_tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, FillVertexConstructor, LineJoin,
    Side, StrokeOptions, StrokeTessellator, StrokeVertex, StrokeVertexConstructor, VertexBuffers,
};
use lyon::math::point;
use lyon::path::builder::PathBuilder;
use lyon::path::path::BuilderWithAttributes;
use lyon::path::{EndpointId, Path};
use lyon::tessellation::VertexSource;
use nalgebra::{Point2, Vector2};
use num_traits::AsPrimitive;
use serde::{Deserialize, Serialize};
use std::ops::Range;

#[derive(Debug)]
pub struct TessellatingRenderBundle {
    pub poly_tessellation: VertexBuffers<PolyVertex, u32>,
    pub points: Vec<PointInstance>,
    pub screen_ref: ScreenRefTessellation,
    pub images: Vec<(DecodedImage, [ImageVertex; 4])>,
    pub clip_area: Option<VertexBuffers<PolyVertex, u32>>,
    pub primitives: Vec<PrimitiveInfo>,
}

pub(crate) type ScreenRefTessellation = VertexBuffers<ScreenRefVertex, u32>;

#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct ScreenRefVertex {
    position: [f32; 3],
    normal: [f32; 2],
    color: [u8; 4],
}

#[derive(Debug, Serialize, Deserialize)]
pub enum PrimitiveInfo {
    MapRef { vertex_range: Range<usize> },
    ScreenRef { vertex_range: Range<usize> },
    Dot { point_index: usize },
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
            poly_tessellation: VertexBuffers::new(),
            points: Vec::new(),
            screen_ref: VertexBuffers::new(),
            images: Vec::new(),
            primitives: Vec::new(),
            clip_area: None,
        }
    }
}

impl TessellatingRenderBundle {
    pub fn clip_area<N, P, Poly>(&mut self, polygon: &Poly)
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
        Poly: Polygon,
        Poly::Contour: Contour<Point = P>,
    {
        let mut tessellation = VertexBuffers::new();
        Self::tessellate_polygon(
            polygon,
            PolygonPaint {
                color: Color::BLACK,
            },
            &mut tessellation,
        );
        self.clip_area = Some(tessellation);
    }

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
        let start_index = self.screen_ref.vertices.len();
        let id = PrimitiveId(self.primitives.len());
        match &paint.shape {
            PointShape::Dot { color } => {
                self.add_dot(point, *color, paint.offset);
                self.primitives.push(PrimitiveInfo::Dot {
                    point_index: self.points.len() - 1,
                });
            }
            PointShape::_Image { .. } => todo!(),
            PointShape::Circle {
                fill,
                radius,
                outline,
            } => {
                self.add_circle(point, *fill, *radius, *outline, paint.offset);
                self.primitives.push(PrimitiveInfo::ScreenRef {
                    vertex_range: start_index..self.screen_ref.vertices.len(),
                });
            }
            PointShape::Sector(parameters) => {
                self.add_circle_sector(point, *parameters, paint.offset);
                self.primitives.push(PrimitiveInfo::ScreenRef {
                    vertex_range: start_index..self.screen_ref.vertices.len(),
                });
            }
            PointShape::Square {
                fill,
                size,
                outline,
            } => {
                self.add_shape(point, *fill, *size, *outline, &square_shape(), paint.offset);
                self.primitives.push(PrimitiveInfo::ScreenRef {
                    vertex_range: start_index..self.screen_ref.vertices.len(),
                });
            }
            PointShape::FreeShape {
                fill,
                scale,
                outline,
                shape,
            } => {
                self.add_shape(point, *fill, *scale, *outline, shape, paint.offset);
                self.primitives.push(PrimitiveInfo::ScreenRef {
                    vertex_range: start_index..self.screen_ref.vertices.len(),
                });
            }
        }

        id
    }

    pub fn add_line<N, P, C>(
        &mut self,
        line: &C,
        paint: LinePaint,
        min_resolution: f64,
    ) -> PrimitiveId
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
        C: Contour<Point = P>,
    {
        let range = self.add_line_lod(line, paint, min_resolution);

        let id = self.primitives.len();
        self.primitives.push(PrimitiveInfo::MapRef {
            vertex_range: range,
        });

        PrimitiveId(id)
    }

    fn add_line_lod<N, P, C>(
        &mut self,
        line: &C,
        paint: LinePaint,
        min_resolution: f64,
    ) -> Range<usize>
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
        C: Contour<Point = P>,
    {
        let tessellation = &mut self.poly_tessellation;
        let mut path_builder = BuilderWithAttributes::new(1);
        let mut iterator = line.iter_points();

        let Some(first_point) = iterator.next() else {
            return 0..0;
        };

        let _ = path_builder.begin(
            point(
                first_point.x().as_() / min_resolution as f32,
                first_point.y().as_() / min_resolution as f32,
            ),
            &[first_point.z().as_()],
        );

        for p in iterator {
            let _ = path_builder.line_to(
                point(
                    p.x().as_() / min_resolution as f32,
                    p.y().as_() / min_resolution as f32,
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
            resolution: min_resolution as f32,
            path: &path,
        };

        let mut tesselator = StrokeTessellator::new();
        let start_index = tessellation.vertices.len();

        tesselator
            .tessellate_path(
                &path,
                &StrokeOptions::DEFAULT
                    .with_line_cap(paint.line_cap.into())
                    .with_line_width(paint.width as f32)
                    .with_miter_limit(1.0)
                    .with_tolerance(0.1)
                    .with_line_join(LineJoin::Round),
                &mut BuffersBuilder::new(tessellation, vertex_constructor),
            )
            .unwrap();

        let end_index = tessellation.vertices.len();
        start_index..end_index
    }

    pub fn add_polygon<N, P, Poly>(
        &mut self,
        polygon: &Poly,
        paint: PolygonPaint,
        min_resolution: f64,
    ) -> PrimitiveId
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
        Poly: Polygon,
        Poly::Contour: Contour<Point = P>,
    {
        let vertex_range = self.add_polygon_lod(polygon, paint, min_resolution as f32);
        let id = self.primitives.len();
        self.primitives.push(PrimitiveInfo::MapRef { vertex_range });

        PrimitiveId(id)
    }

    fn add_polygon_lod<N, P, Poly>(
        &mut self,
        polygon: &Poly,
        paint: PolygonPaint,
        _min_resolution: f32,
    ) -> Range<usize>
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
        Poly: Polygon,
        Poly::Contour: Contour<Point = P>,
    {
        let lod = &mut self.poly_tessellation;
        let start_index = lod.vertices.len();

        Self::tessellate_polygon(polygon, paint, lod);

        let end_index = lod.vertices.len();
        start_index..end_index
    }

    pub fn is_empty(&self) -> bool {
        self.primitives.is_empty()
    }

    fn tessellate_polygon<N, P, Poly>(
        polygon: &Poly,
        paint: PolygonPaint,
        tessellation: &mut VertexBuffers<PolyVertex, u32>,
    ) where
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
                return;
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

        tesselator
            .tessellate(
                &path,
                &FillOptions::DEFAULT,
                &mut BuffersBuilder::new(tessellation, vertex_constructor),
            )
            .unwrap();
    }

    pub fn add_shape<N, P>(
        &mut self,
        position: &P,
        fill: Color,
        scale: f32,
        outline: Option<LinePaint>,
        shape: &ClosedContour<Point2<f32>>,
        offset: Vector2<f32>,
    ) where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
    {
        let mut path_builder = BuilderWithAttributes::new(0);
        build_contour_path(&mut path_builder, shape, scale);
        let path = path_builder.build();

        if let Some(outline) = outline {
            let vertex_constructor = ScreenRefVertexConstructor {
                color: outline.color.to_u8_array(),
                position: [position.x().as_(), position.y().as_(), position.z().as_()],
                offset,
            };

            if let Err(err) = StrokeTessellator::new().tessellate(
                &path,
                &StrokeOptions::DEFAULT.with_line_width(outline.width as f32 * 2.0),
                &mut BuffersBuilder::new(&mut self.screen_ref, vertex_constructor),
            ) {
                log::warn!("Shape tessellation failed: {err:?}");
                return;
            }
        }

        if !fill.is_transparent() {
            let vertex_constructor = ScreenRefVertexConstructor {
                color: fill.to_u8_array(),
                position: [position.x().as_(), position.y().as_(), position.z().as_()],
                offset,
            };

            if let Err(err) = FillTessellator::new().tessellate(
                &path,
                &FillOptions::DEFAULT,
                &mut BuffersBuilder::new(&mut self.screen_ref, vertex_constructor),
            ) {
                log::warn!("Shape tessellation failed: {err:?}");
            }
        }
    }

    fn add_circle<N, P>(
        &mut self,
        position: &P,
        fill: CircleFill,
        radius: f32,
        outline: Option<LinePaint>,
        offset: Vector2<f32>,
    ) where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
    {
        self.add_circle_sector(
            position,
            SectorParameters {
                fill,
                radius,
                start_angle: 0.0,
                end_angle: std::f32::consts::PI * 2.0,
                outline,
            },
            offset,
        )
    }

    fn add_circle_sector<N, P>(
        &mut self,
        position: &P,
        parameters: SectorParameters,
        offset: Vector2<f32>,
    ) where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
    {
        let SectorParameters {
            fill,
            radius,
            start_angle,
            end_angle,
            outline,
        } = parameters;
        const TOLERANCE: f32 = 0.1;
        let dr = (end_angle - start_angle)
            .abs()
            .min(std::f32::consts::PI * 2.0);

        let center = ScreenRefVertex {
            position: [position.x().as_(), position.y().as_(), position.z().as_()],
            normal: [offset.x, offset.y],
            color: fill.center_color.to_u8_array(),
        };

        let is_full_circle = (dr - std::f32::consts::PI * 2.0).abs() < TOLERANCE;

        let mut contour = get_circle_sector(radius, start_angle, end_angle);
        let first_index = self.screen_ref.vertices.len() as u32;

        let mut vertices = vec![center];
        let mut indices = vec![];
        for point in &contour {
            if vertices.len() > 1 {
                indices.push(first_index);
                indices.push(vertices.len() as u32 - 1 + first_index);
                indices.push(vertices.len() as u32 + first_index);
            }

            vertices.push(ScreenRefVertex {
                position: [position.x().as_(), position.y().as_(), position.z().as_()],
                normal: (point + offset).coords.into(),
                color: fill.side_color.to_u8_array(),
            });
        }

        if is_full_circle {
            indices.push(first_index);
            indices.push(vertices.len() as u32 - 1 + first_index);
            indices.push(1 + first_index);
        }

        self.screen_ref.vertices.append(&mut vertices);
        self.screen_ref.indices.append(&mut indices);

        if outline.is_some() {
            if !is_full_circle {
                contour.push(Point2::new(0.0, 0.0));
            }
            self.add_shape(
                position,
                Color::TRANSPARENT,
                radius,
                outline,
                &ClosedContour::new(contour),
                offset,
            );
        }
    }

    fn add_dot<P, N>(&mut self, point: &P, color: Color, offset: Vector2<f32>)
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
    {
        let position = [
            point.x().as_() + offset.x,
            point.y().as_() + offset.y,
            point.z().as_(),
        ];
        self.points.push(PointInstance {
            position,
            color: color.to_u8_array(),
        })
    }
}

fn get_circle_sector(radius: f32, start_angle: f32, end_angle: f32) -> Vec<Point2<f32>> {
    const TOLERANCE: f32 = 0.1;

    let mut contour = vec![];

    if radius <= TOLERANCE {
        return contour;
    }

    let dr = (end_angle - start_angle)
        .abs()
        .min(std::f32::consts::PI * 2.0);

    let circle_steps_count =
        std::f32::consts::PI / ((radius - TOLERANCE) / (radius + TOLERANCE)).acos();

    let segment_steps_count =
        ((dr / std::f32::consts::PI * 2.0) * circle_steps_count).ceil() as usize;
    let angle_step = (end_angle - start_angle) / segment_steps_count as f32;

    for step in 0..segment_steps_count {
        let angle = start_angle + angle_step * step as f32;
        let x = angle.cos() * radius;
        let y = angle.sin() * radius;
        contour.push(Point2::new(x, y));
    }

    contour
}

fn square_shape() -> ClosedContour<Point2<f32>> {
    ClosedContour::new(vec![
        Point2::new(-0.5, -0.5),
        Point2::new(-0.5, 0.5),
        Point2::new(0.5, 0.5),
        Point2::new(0.5, -0.5),
    ])
}

fn build_contour_path(
    path_builder: &mut impl PathBuilder,
    contour: &impl Contour<Point = Point2<f32>>,
    scale: f32,
) -> Option<()> {
    let mut iterator = contour.iter_points();

    if let Some(first_point) = iterator.next() {
        let _ = path_builder.begin(point(first_point.x() * scale, first_point.y() * scale), &[]);
    } else {
        return None;
    }

    for p in iterator {
        let _ = path_builder.line_to(point(p.x() * scale, p.y() * scale), &[]);
    }

    path_builder.end(contour.is_closed());

    Some(())
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

        let norm_limit = if let VertexSource::Endpoint { id } = vertex.source() {
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
                f32::MAX
            }
        } else {
            f32::MAX
        };

        PolyVertex {
            position: [
                position.x * self.resolution,
                position.y * self.resolution,
                vertex.interpolated_attributes()[0],
            ],
            color: self.color,
            normal,
            norm_limit,
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
            norm_limit: 1.0,
        }
    }
}

struct ScreenRefVertexConstructor {
    color: [u8; 4],
    position: [f32; 3],
    offset: Vector2<f32>,
}

impl ScreenRefVertexConstructor {
    fn create_vertex(&self, position: lyon::math::Point) -> ScreenRefVertex {
        ScreenRefVertex {
            position: self.position,
            normal: [position.x + self.offset.x, position.y + self.offset.y],
            color: self.color,
        }
    }
}

impl StrokeVertexConstructor<ScreenRefVertex> for ScreenRefVertexConstructor {
    fn new_vertex(&mut self, vertex: StrokeVertex) -> ScreenRefVertex {
        self.create_vertex(vertex.position())
    }
}

impl FillVertexConstructor<ScreenRefVertex> for ScreenRefVertexConstructor {
    fn new_vertex(&mut self, vertex: FillVertex) -> ScreenRefVertex {
        self.create_vertex(vertex.position())
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Serialize, Deserialize)]
pub struct PolyVertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
    pub normal: [f32; 2],
    pub norm_limit: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PointInstance {
    pub position: [f32; 3],
    pub color: [u8; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ImageVertex {
    pub position: [f32; 2],
    pub opacity: f32,
    pub tex_coords: [f32; 2],
}

#[cfg(feature = "byte-conversion")]
pub(crate) mod serialization;
