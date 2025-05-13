use std::mem::size_of;
use std::sync::Arc;

use galileo_types::cartesian::{CartesianPoint2d, CartesianPoint3d, Point2, Vector2};
use galileo_types::contour::Contour;
use galileo_types::impls::ClosedContour;
use galileo_types::Polygon;
use lyon::lyon_tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, FillVertexConstructor, LineJoin,
    Side, StrokeOptions, StrokeTessellator, StrokeVertex, StrokeVertexConstructor, VertexBuffers,
};
fn point(x: f64, y: f64) -> lyon::math::Point {
    lyon::math::Point::new(x as _, y as _)
}
use lyon::path::builder::PathBuilder;
use lyon::path::path::BuilderWithAttributes;
use lyon::path::{EndpointId, Path};
use lyon::tessellation::VertexSource;
use num_traits::AsPrimitive;
use serde::{Deserialize, Serialize};

use crate::decoded_image::DecodedImage;
use crate::render::point_paint::{CircleFill, PointPaint, PointShape, SectorParameters};
use crate::render::text::{TextService, TextShaping, TextStyle};
use crate::render::{ImagePaint, LinePaint, PolygonPaint};
use crate::{Color, MapView};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct WorldRenderSet {
    pub poly_tessellation: VertexBuffers<PolyVertex, u32>,
    pub points: Vec<PointInstance>,
    pub images: Vec<ImageInfo>,
    pub clip_area: Option<VertexBuffers<PolyVertex, u32>>,
    pub image_store: Vec<Arc<DecodedImage>>,
    pub buffer_size: usize,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Serialize, Deserialize)]
pub(crate) struct ImageInfo {
    pub(crate) store_index: usize,
    pub(crate) vertices: [ImageVertex; 4],
}

#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Serialize, Deserialize)]
#[repr(C)]
pub(crate) struct ScreenRefVertex {
    position: [f32; 3],
    normal: [f32; 2],
    color: [u8; 4],
}

impl Default for WorldRenderSet {
    fn default() -> Self {
        Self::new()
    }
}

impl WorldRenderSet {
    pub fn new() -> Self {
        Self {
            poly_tessellation: VertexBuffers::new(),
            points: Vec::new(),
            images: Vec::new(),
            clip_area: None,
            image_store: Vec::new(),
            buffer_size: 0,
        }
    }

    pub fn approx_buffer_size(&self) -> usize {
        self.buffer_size
    }

    pub fn clip_area<N, P, Poly>(&mut self, polygon: &Poly)
    where
        N: AsPrimitive<f64>,
        P: CartesianPoint3d<Num = N>,
        Poly: Polygon,
        Poly::Contour: Contour<Point = P>,
    {
        let mut tessellation = VertexBuffers::new();
        Self::tessellate_polygon(
            polygon,
            &PolygonPaint {
                color: Color::BLACK,
            },
            &mut tessellation,
        );

        self.buffer_size += tessellation.vertices.len() * std::mem::size_of::<PolyVertex>()
            + tessellation.indices.len() * std::mem::size_of::<u32>();

        self.clip_area = Some(tessellation);
    }

    pub fn add_image(
        &mut self,
        image: DecodedImage,
        vertices: [Point2; 4],
        paint: ImagePaint,
        view: &MapView,
    ) {
        let opacity = paint.opacity as f32 / 255.0;

        self.buffer_size += image.byte_size() + std::mem::size_of::<ImageVertex>() * 4;

        let index = self.add_image_to_store(Arc::new(image), view);
        let vertices = [
            ImageVertex {
                position: [vertices[0].x() as f32, vertices[0].y() as f32],
                opacity,
                tex_coords: [0.0, 1.0],
                offset: [0.0, 0.0],
            },
            ImageVertex {
                position: [vertices[1].x() as f32, vertices[1].y() as f32],
                opacity,
                tex_coords: [0.0, 0.0],
                offset: [0.0, 0.0],
            },
            ImageVertex {
                position: [vertices[3].x() as f32, vertices[3].y() as f32],
                opacity,
                tex_coords: [1.0, 1.0],
                offset: [0.0, 0.0],
            },
            ImageVertex {
                position: [vertices[2].x() as f32, vertices[2].y() as f32],
                opacity,
                tex_coords: [1.0, 0.0],
                offset: [0.0, 0.0],
            },
        ];

        self.add_image_info(index, vertices, view);
    }

    fn add_image_info(
        &mut self,
        image_store_index: usize,
        vertices: [ImageVertex; 4],
        view: &MapView,
    ) -> usize {
        let index = self.images.len();
        self.images.push(ImageInfo {
            store_index: image_store_index,
            vertices,
        });
        index
    }

    fn add_image_to_store(&mut self, image: Arc<DecodedImage>, view: &MapView) -> usize {
        for (i, stored) in self.image_store.iter().enumerate() {
            if Arc::ptr_eq(stored, &image) {
                return i;
            }
        }

        let index = self.image_store.len();
        self.image_store.push(image);
        index
    }

    pub fn add_point<N, P>(&mut self, point: &P, paint: &PointPaint, view: &MapView)
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
    {
        match &paint.shape {
            PointShape::Dot { color } => {
                self.add_dot(point, *color, paint.offset, view);
            }
            PointShape::Circle {
                fill,
                radius,
                outline,
            } => {
                self.add_circle(point, *fill, *radius, *outline, paint.offset);
            }
            PointShape::Sector(parameters) => {
                self.add_circle_sector(point, *parameters, paint.offset);
            }
            PointShape::Square {
                fill,
                size,
                outline,
            } => {
                self.add_shape(point, *fill, *size, *outline, &square_shape(), paint.offset);
            }
            PointShape::FreeShape {
                fill,
                scale,
                outline,
                shape,
            } => {
                self.add_shape(point, *fill, *scale, *outline, shape, paint.offset);
            }
            PointShape::Label { text, style } => {
                self.add_label(point, text, style, paint.offset, view)
            }
        };
    }

    pub fn add_line<N, P, C>(
        &mut self,
        line: &C,
        paint: &LinePaint,
        min_resolution: f64,
        view: &MapView,
    ) where
        N: AsPrimitive<f64>,
        P: CartesianPoint3d<Num = N>,
        C: Contour<Point = P>,
    {
        self.add_line_lod(line, *paint, min_resolution, view);
    }

    fn add_line_lod<N, P, C>(
        &mut self,
        line: &C,
        paint: LinePaint,
        min_resolution: f64,
        view: &MapView,
    ) where
        N: AsPrimitive<f64>,
        P: CartesianPoint3d<Num = N>,
        C: Contour<Point = P>,
    {
        let tessellation = &mut self.poly_tessellation;
        let mut path_builder = BuilderWithAttributes::new(1);
        let mut iterator = line.iter_points();

        let Some(first_point) = iterator.next() else {
            return;
        };

        let [cx, cy] = view
            .projected_center()
            .map(|p| [p.x(), p.y()])
            .unwrap_or([first_point.x().as_(), first_point.y().as_()]);
        let at = point(
            (first_point.x().as_() - cx) / min_resolution,
            (first_point.y().as_() - cy) / min_resolution,
        );
        let _ = path_builder.begin(at, &[first_point.z().as_() as f32]);

        for p in iterator {
            let _ = path_builder.line_to(
                point(
                    (p.x().as_() - cx) / min_resolution,
                    (p.y().as_() - cy) / min_resolution,
                ),
                &[p.z().as_() as f32],
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
            centroid: [cx, cy],
        };

        let mut tesselator = StrokeTessellator::new();
        let start_index = tessellation.vertices.len();
        let start_index_count = tessellation.indices.len();

        if let Err(err) = tesselator.tessellate_path(
            &path,
            &StrokeOptions::DEFAULT
                .with_line_cap(paint.line_cap.into())
                .with_line_width(paint.width as f32)
                .with_miter_limit(1.0)
                .with_tolerance(0.1)
                .with_line_join(LineJoin::MiterClip),
            &mut BuffersBuilder::new(tessellation, vertex_constructor),
        ) {
            log::error!("Tessellation failed: {err}");
            return;
        }

        let end_index = tessellation.vertices.len();

        self.buffer_size += (end_index - start_index) * size_of::<PolyVertex>();
        self.buffer_size += (tessellation.indices.len() - start_index_count) * size_of::<u32>();
    }

    pub fn add_polygon<N, P, Poly>(
        &mut self,
        polygon: &Poly,
        paint: &PolygonPaint,
        min_resolution: f64,
        view: &MapView,
    ) where
        N: AsPrimitive<f64>,
        P: CartesianPoint3d<Num = N>,
        Poly: Polygon,
        Poly::Contour: Contour<Point = P>,
    {
        self.add_polygon_lod(polygon, paint, min_resolution, view);
    }

    fn add_polygon_lod<N, P, Poly>(
        &mut self,
        polygon: &Poly,
        paint: &PolygonPaint,
        _min_resolution: f64,
        view: &MapView,
    ) where
        N: AsPrimitive<f64>,
        P: CartesianPoint3d<Num = N>,
        Poly: Polygon,
        Poly::Contour: Contour<Point = P>,
    {
        let lod = &mut self.poly_tessellation;
        let start_index = lod.vertices.len();
        let start_index_count = lod.indices.len();

        Self::tessellate_polygon(polygon, paint, lod);

        let end_index = self.poly_tessellation.vertices.len();

        self.buffer_size += (end_index - start_index) * size_of::<PolyVertex>();
        self.buffer_size +=
            (self.poly_tessellation.indices.len() - start_index_count) * size_of::<u32>();
    }

    fn tessellate_polygon<N, P, Poly>(
        polygon: &Poly,
        paint: &PolygonPaint,
        tessellation: &mut VertexBuffers<PolyVertex, u32>,
    ) where
        N: AsPrimitive<f64>,
        P: CartesianPoint3d<Num = N>,
        Poly: Polygon,
        Poly::Contour: Contour<Point = P>,
    {
        let ([cx, cy], ln) = polygon
            .iter_contours()
            .flat_map(|c| c.iter_points())
            .fold(([0.0, 0.0], 0), |([x, y], i), p| {
                ([x + p.x().as_(), y + p.y().as_()], i + 1)
            });
        let [cx, cy] = [cx / ln as f64, cy / ln as f64];
        let mut path_builder = BuilderWithAttributes::new(1);
        for contour in polygon.iter_contours() {
            let mut iterator = contour.iter_points();

            if let Some(first_point) = iterator.next() {
                let _ = path_builder.begin(
                    point(first_point.x().as_() - cx, first_point.y().as_() - cy),
                    &[first_point.z().as_() as _],
                );
            } else {
                return;
            }

            for p in iterator {
                let _ = path_builder.line_to(
                    point(p.x().as_() - cx, p.y().as_() - cy),
                    &[p.z().as_() as _],
                );
            }

            path_builder.end(true);
        }

        let path = path_builder.build();

        let vertex_constructor = PolygonVertexConstructor {
            color: paint.color.to_f32_array(),
            centroid: [cx, cy],
        };
        let mut tesselator = FillTessellator::new();

        if let Err(err) = tesselator.tessellate(
            &path,
            &FillOptions::DEFAULT,
            &mut BuffersBuilder::new(tessellation, vertex_constructor),
        ) {
            log::error!("Tessellation failed: {err:?}");
        }
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

        let start_vertex_count = self.poly_tessellation.vertices.len();
        let start_index_count = self.poly_tessellation.indices.len();

        let tessellation = &mut self.poly_tessellation;

        if let Some(outline) = outline {
            let vertex_constructor = ScreenRefVertexConstructor {
                color: outline.color.to_f32_array(),
                position: [
                    position.x().as_() as _,
                    position.y().as_() as _,
                    position.z().as_() as _,
                ],
                offset,
            };

            if let Err(err) = StrokeTessellator::new().tessellate(
                &path,
                &StrokeOptions::DEFAULT.with_line_width(outline.width as f32 * 2.0),
                &mut BuffersBuilder::new(tessellation, vertex_constructor),
            ) {
                log::warn!("Shape tessellation failed: {err:?}");
                return;
            }
        }

        if !fill.is_transparent() {
            let vertex_constructor = ScreenRefVertexConstructor {
                color: fill.to_f32_array(),
                position: [
                    position.x().as_() as _,
                    position.y().as_() as _,
                    position.z().as_() as _,
                ],
                offset,
            };

            if let Err(err) = FillTessellator::new().tessellate(
                &path,
                &FillOptions::DEFAULT,
                &mut BuffersBuilder::new(tessellation, vertex_constructor),
            ) {
                log::warn!("Shape tessellation failed: {err:?}");
            }
        }

        self.buffer_size += (self.poly_tessellation.vertices.len() - start_vertex_count)
            * std::mem::size_of::<ScreenRefVertex>();
        self.buffer_size +=
            (self.poly_tessellation.indices.len() - start_index_count) * std::mem::size_of::<u32>();
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

        let center = PolyVertex::new(
            [
                position.x().as_() as _,
                position.y().as_() as _,
                position.z().as_() as _,
            ],
            fill.center_color.to_f32_array(),
            [offset.dx(), offset.dy()],
            f32::MAX,
        );

        let is_full_circle = (dr - std::f32::consts::PI * 2.0).abs() < TOLERANCE;

        let mut contour = get_circle_sector(radius, start_angle, end_angle);
        let first_index = self.poly_tessellation.vertices.len() as u32;

        let start_vertex_count = self.poly_tessellation.vertices.len();
        let start_index_count = self.poly_tessellation.indices.len();

        let mut vertices = vec![center];
        let mut indices = vec![];
        for point in &contour {
            if vertices.len() > 1 {
                indices.push(first_index);
                indices.push(vertices.len() as u32 - 1 + first_index);
                indices.push(vertices.len() as u32 + first_index);
            }

            vertices.push(PolyVertex::new(
                [
                    position.x().as_() as _,
                    position.y().as_() as _,
                    position.z().as_() as _,
                ],
                fill.side_color.to_f32_array(),
                (*point + offset).coords(),
                f32::MAX,
            ));
        }

        if is_full_circle {
            indices.push(first_index);
            indices.push(vertices.len() as u32 - 1 + first_index);
            indices.push(1 + first_index);
        }

        self.poly_tessellation.vertices.append(&mut vertices);
        self.poly_tessellation.indices.append(&mut indices);

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

        self.buffer_size += (self.poly_tessellation.vertices.len() - start_vertex_count)
            * std::mem::size_of::<ScreenRefVertex>();
        self.buffer_size +=
            (self.poly_tessellation.indices.len() - start_index_count) * std::mem::size_of::<u32>();
    }

    fn add_dot<P, N>(&mut self, point: &P, color: Color, offset: Vector2<f32>, view: &MapView)
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
    {
        let position = [
            point.x().as_() + offset.dx(),
            point.y().as_() + offset.dy(),
            point.z().as_(),
        ];
        self.points.push(PointInstance {
            position,
            color: color.to_u8_array(),
        });
        self.buffer_size += size_of::<PointInstance>();
    }

    pub fn add_label<N, P>(
        &mut self,
        position: &P,
        text: &str,
        style: &TextStyle,
        offset: Vector2<f32>,
        view: &MapView,
    ) where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
    {
        match TextService::shape(text, style, offset) {
            Ok(TextShaping::Tessellation { glyphs, .. }) => {
                for glyph in glyphs {
                    let vertices_start = self.poly_tessellation.vertices.len() as u32;
                    for vertex in glyph.vertices {
                        self.poly_tessellation.vertices.push(PolyVertex::new(
                            [
                                position.x().as_() as _,
                                position.y().as_() as _,
                                position.z().as_() as _,
                            ],
                            vertex.color.to_f32_array(),
                            vertex.position,
                            f32::MAX,
                        ));
                    }
                    for index in glyph.indices {
                        self.poly_tessellation.indices.push(index + vertices_start);
                    }
                }
            }
            Err(err) => {
                log::error!("Error shaping text label: {err:?}");
            }
            _ => {
                log::error!("Not supported font type");
            }
        }
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
    // TODO: centroid this maybe
    let mut iterator = contour.iter_points();

    if let Some(first_point) = iterator.next() {
        let _ = path_builder.begin(
            point(
                (first_point.x() * scale) as f64,
                (first_point.y() * scale) as f64,
            ),
            &[],
        );
    } else {
        return None;
    }

    for p in iterator {
        let _ = path_builder.line_to(point((p.x() * scale) as f64, (p.y() * scale) as f64), &[]);
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
    centroid: [f64; 2],
}

impl StrokeVertexConstructor<PolyVertex> for LineVertexConstructor<'_> {
    fn new_vertex(&mut self, mut vertex: StrokeVertex) -> PolyVertex {
        let [cx, cy] = self.centroid;
        let position = vertex.position_on_path();
        let offset = match vertex.side() {
            Side::Negative => -self.offset,
            Side::Positive => self.offset,
        };

        let normal = [
            vertex.normal().x * (vertex.line_width() / 2.0 + offset),
            vertex.normal().y * (vertex.line_width() / 2.0 + offset),
        ];

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

        PolyVertex::new(
            [
                ((position.x * self.resolution) as f64 + cx),
                ((position.y * self.resolution) as f64 + cy),
                vertex.interpolated_attributes()[0] as _,
            ],
            self.color,
            normal,
            norm_limit,
        )
    }
}

struct PolygonVertexConstructor {
    color: [f32; 4],
    centroid: [f64; 2],
}

impl FillVertexConstructor<PolyVertex> for PolygonVertexConstructor {
    fn new_vertex(&mut self, vertex: FillVertex) -> PolyVertex {
        let [cx, cy] = self.centroid;
        PolyVertex::new(
            [
                (vertex.position().x as f64 + cx) as _,
                (vertex.position().y as f64 + cy) as _,
                0.0,
            ],
            self.color,
            Default::default(),
            1.0,
        )
    }
}

struct ScreenRefVertexConstructor {
    color: [f32; 4],
    position: [f32; 3],
    offset: Vector2<f32>,
}

impl ScreenRefVertexConstructor {
    fn create_vertex(&self, vertex: lyon::math::Point) -> PolyVertex {
        let normal = [vertex.x + self.offset.dx(), vertex.y + self.offset.dy()];
        let [x, y, z] = self.position;
        let position = [x as f64, y as f64, z as f64];
        PolyVertex::new(position, self.color, normal, f32::MAX)
    }
}

impl StrokeVertexConstructor<PolyVertex> for ScreenRefVertexConstructor {
    fn new_vertex(&mut self, vertex: StrokeVertex) -> PolyVertex {
        self.create_vertex(vertex.position())
    }
}

impl FillVertexConstructor<PolyVertex> for ScreenRefVertexConstructor {
    fn new_vertex(&mut self, vertex: FillVertex) -> PolyVertex {
        self.create_vertex(vertex.position())
    }
}

#[repr(C)]
// #[derive(Copy, Clone, Debug, bytemuck::Zeroable, Serialize, Deserialize)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Serialize, Deserialize)]
pub(crate) struct PolyVertex {
    pub position: [f64; 4],
    pub color: [f32; 4],
    pub normal: [f32; 2],
    pub norm_limit: f32,
    _padding: u32,
}

impl PolyVertex {
    pub(crate) fn new(
        [x, y, z]: [f64; 3],
        color: [f32; 4],
        normal: [f32; 2],
        norm_limit: f32,
    ) -> Self {
        Self {
            position: [x, y, z, 0.],
            color,
            normal,
            norm_limit,
            _padding: 0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Serialize, Deserialize)]
pub(crate) struct PointInstance {
    pub position: [f32; 3],
    pub color: [u8; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Serialize, Deserialize)]
pub(crate) struct ImageVertex {
    pub position: [f32; 2],
    pub opacity: f32,
    pub tex_coords: [f32; 2],
    pub offset: [f32; 2],
}
