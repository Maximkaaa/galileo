use std::mem::size_of;
use std::sync::Arc;

use galileo_types::cartesian::{CartesianPoint2d, CartesianPoint3d, Point2, Point3, Vector2};
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

pub struct ShapeArguments<'a> {
    fill: Color,
    scale: f32,
    outline: Option<LinePaint>,
    shape: &'a ClosedContour<Point2<f32>>,
    offset: Vector2<f32>,
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

    pub fn clip_area<N, P, Poly>(&mut self, polygon: &Poly, view: &MapView)
    // Added view
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
            view, // Pass view
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

        // let [cx, cy, _] = view.projected_center().expect("Invalid MapView").array();
        let [cx, cy] = [0.0, 0.0];

        let relative_vertices = [
            ImageVertex {
                position: [(vertices[0].x() - cx) as f32, (vertices[0].y() - cy) as f32],
                opacity,
                tex_coords: [0.0, 1.0],
                offset: [0.0, 0.0],
            },
            ImageVertex {
                position: [(vertices[1].x() - cx) as f32, (vertices[1].y() - cy) as f32],
                opacity,
                tex_coords: [0.0, 0.0],
                offset: [0.0, 0.0],
            },
            ImageVertex {
                position: [(vertices[3].x() - cx) as f32, (vertices[3].y() - cy) as f32],
                opacity,
                tex_coords: [1.0, 1.0],
                offset: [0.0, 0.0],
            },
            ImageVertex {
                position: [(vertices[2].x() - cx) as f32, (vertices[2].y() - cy) as f32],
                opacity,
                tex_coords: [1.0, 0.0],
                offset: [0.0, 0.0],
            },
        ];

        self.add_image_info(index, relative_vertices, view);
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
        N: AsPrimitive<f64>,
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
                self.add_circle(point, *fill, *radius, *outline, paint.offset, view);
            }
            PointShape::Sector(parameters) => {
                self.add_circle_sector(point, *parameters, paint.offset, view);
            }
            PointShape::Square {
                fill,
                size,
                outline,
            } => {
                let shape = ShapeArguments {
                    fill: *fill,
                    scale: *size,
                    outline: *outline,
                    shape: &square_shape(),
                    offset: paint.offset,
                };
                self.add_shape(point, shape, view);
            }
            PointShape::FreeShape {
                fill,
                scale,
                outline,
                shape,
            } => {
                let shape = ShapeArguments {
                    fill: *fill,
                    scale: *scale,
                    outline: *outline,
                    shape,
                    offset: paint.offset,
                };
                self.add_shape(point, shape, view);
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

        let view_center = view.projected_center().unwrap();
        let cx = view_center.x();
        let cy = view_center.y();
        let cz = view_center.z(); // Assuming Z=0 if not otherwise set

        let at = point(
            (first_point.x().as_() - cx) / min_resolution,
            (first_point.y().as_() - cy) / min_resolution,
        );
        // Make Z coordinate relative to view center's Z
        let _ = path_builder.begin(at, &[(first_point.z().as_() - cz) as f32]);

        for p in iterator {
            let _ = path_builder.line_to(
                point(
                    (p.x().as_() - cx) / min_resolution,
                    (p.y().as_() - cy) / min_resolution,
                ),
                // Make Z coordinate relative to view center's Z
                &[(p.z().as_() - cz) as f32],
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

        Self::tessellate_polygon(polygon, paint, lod, view); // Pass view

        let end_index = self.poly_tessellation.vertices.len();

        self.buffer_size += (end_index - start_index) * size_of::<PolyVertex>();
        self.buffer_size +=
            (self.poly_tessellation.indices.len() - start_index_count) * size_of::<u32>();
    }

    fn tessellate_polygon<N, P, Poly>(
        polygon: &Poly,
        paint: &PolygonPaint,
        tessellation: &mut VertexBuffers<PolyVertex, u32>,
        view: &MapView, // Added view parameter
    ) where
        N: AsPrimitive<f64>,
        P: CartesianPoint3d<Num = N>,
        Poly: Polygon,
        Poly::Contour: Contour<Point = P>,
    {
        // Use the MapView's projected center as the reference for relative coordinates
        // Get the first point to use as a fallback if view center is not available (though it should be)
        let view_center = view.projected_center().unwrap();
        let v_cx = view_center.x();
        let v_cy = view_center.y();
        let v_cz = view_center.z();

        let mut path_builder = BuilderWithAttributes::new(1); // 1 attribute for Z
        for contour in polygon.iter_contours() {
            let mut iterator = contour.iter_points();

            if let Some(first_point) = iterator.next() {
                let _ = path_builder.begin(
                    point(first_point.x().as_() - v_cx, first_point.y().as_() - v_cy),
                    &[(first_point.z().as_() - v_cz) as f32], // Z made relative
                );
            } else {
                // If a contour is empty, skip it. If all contours are empty, path_builder will be empty.
                continue;
            }

            for p in iterator {
                let _ = path_builder.line_to(
                    point(p.x().as_() - v_cx, p.y().as_() - v_cy),
                    &[(p.z().as_() - v_cz) as f32], // Z made relative
                );
            }

            path_builder.end(true); // Polygons are closed contours
        }

        let path = path_builder.build();

        let vertex_constructor = PolygonVertexConstructor {
            color: paint.color.to_f32_array(),
            // centroid field is removed
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

    pub fn add_shape<N, P>(&mut self, position: &P, shape: ShapeArguments, view: &MapView)
    where
        N: AsPrimitive<f64>,
        P: CartesianPoint3d<Num = N>,
    {
        let ShapeArguments {
            fill,
            scale,
            outline,
            shape,
            offset,
        } = shape;
        let view_center = view.projected_center().unwrap();
        let rel_anchor_x = position.x().as_() - view_center.x();
        let rel_anchor_y = position.y().as_() - view_center.y();
        let rel_anchor_z = position.z().as_() - view_center.z();
        let relative_anchor_pos_f32 = [
            rel_anchor_x as f32,
            rel_anchor_y as f32,
            rel_anchor_z as f32,
        ];

        let mut path_builder = BuilderWithAttributes::new(0);
        build_contour_path(&mut path_builder, shape, scale);
        let path = path_builder.build();

        let start_vertex_count = self.poly_tessellation.vertices.len();
        let start_index_count = self.poly_tessellation.indices.len();

        let tessellation = &mut self.poly_tessellation;

        if let Some(outline) = outline {
            let vertex_constructor = ScreenRefVertexConstructor {
                color: outline.color.to_f32_array(),
                position: relative_anchor_pos_f32,
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
                position: relative_anchor_pos_f32,
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
        view: &MapView,
    ) where
        N: AsPrimitive<f64>,
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
            view,
        )
    }

    fn add_circle_sector<N, P>(
        &mut self,
        position: &P,
        parameters: SectorParameters,
        offset: Vector2<f32>,
        view: &MapView,
    ) where
        N: AsPrimitive<f64>,
        P: CartesianPoint3d<Num = N>,
    {
        let view_center = view.projected_center().unwrap();
        let rel_anchor_x = position.x().as_() - view_center.x();
        let rel_anchor_y = position.y().as_() - view_center.y();
        let rel_anchor_z = position.z().as_() - view_center.z();

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
            [rel_anchor_x, rel_anchor_y, rel_anchor_z],
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
                [rel_anchor_x, rel_anchor_y, rel_anchor_z],
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
            let shape = ShapeArguments {
                fill: Color::TRANSPARENT,
                scale: radius,
                outline,
                shape: &ClosedContour::new(contour),
                offset,
            };
            self.add_shape(position, shape, view);
        }

        self.buffer_size += (self.poly_tessellation.vertices.len() - start_vertex_count)
            * std::mem::size_of::<ScreenRefVertex>();
        self.buffer_size +=
            (self.poly_tessellation.indices.len() - start_index_count) * std::mem::size_of::<u32>();
    }

    fn add_dot<P, N>(&mut self, point: &P, color: Color, offset: Vector2<f32>, view: &MapView)
    where
        N: AsPrimitive<f64>,
        P: CartesianPoint3d<Num = N>,
    {
        let view_center = view.projected_center().unwrap();
        let vc_x = view_center.x();
        let vc_y = view_center.y();
        let vc_z = view_center.z();

        // Assuming offset is in world units for Dot, applied before making relative to view center
        let world_x_with_offset = point.x().as_() + offset.dx() as f64;
        let world_y_with_offset = point.y().as_() + offset.dy() as f64;
        let world_z = point.z().as_();

        self.points.push(PointInstance {
            position: [
                (world_x_with_offset - vc_x) as f32,
                (world_y_with_offset - vc_y) as f32,
                (world_z - vc_z) as f32,
            ],
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
        N: AsPrimitive<f64>,
        P: CartesianPoint3d<Num = N>,
    {
        let view_center = view.projected_center().unwrap();
        let rel_anchor_x = position.x().as_() - view_center.x();
        let rel_anchor_y = position.y().as_() - view_center.y();
        let rel_anchor_z = position.z().as_() - view_center.z();

        match TextService::shape(text, style, offset) {
            Ok(TextShaping::Tessellation { glyphs, .. }) => {
                for glyph in glyphs {
                    let vertices_start = self.poly_tessellation.vertices.len() as u32;
                    for vertex in glyph.vertices {
                        self.poly_tessellation.vertices.push(PolyVertex::new(
                            [rel_anchor_x, rel_anchor_y, rel_anchor_z],
                            vertex.color.to_f32_array(),
                            vertex.position, // vertex.position is glyph's local offset + paint.offset
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
}

impl StrokeVertexConstructor<PolyVertex> for LineVertexConstructor<'_> {
    fn new_vertex(&mut self, mut vertex: StrokeVertex) -> PolyVertex {
        // position_on_path() gives coordinates relative to the (0,0) of the path builder,
        // which were (world_coord - view_center_coord) / resolution.
        // So, multiplying by self.resolution gives (world_coord - view_center_coord).
        let pos_x_relative_to_view_center = (vertex.position_on_path().x * self.resolution) as f64;
        let pos_y_relative_to_view_center = (vertex.position_on_path().y * self.resolution) as f64;

        // Z coordinate was also made relative: (world_z - view_center_z)
        // It was passed as an attribute to Lyon, so interpolated_attributes()[0] contains this relative Z.
        let pos_z_relative_to_view_center = vertex.interpolated_attributes()[0] as f64;

        let screen_offset_val = match vertex.side() {
            Side::Negative => -self.offset,
            Side::Positive => self.offset,
        };

        let normal_for_shader = [
            // This is the screen-space style offset vector
            vertex.normal().x * (vertex.line_width() / 2.0 + screen_offset_val),
            vertex.normal().y * (vertex.line_width() / 2.0 + screen_offset_val),
        ];

        let norm_limit = if let VertexSource::Endpoint { id } = vertex.source() {
            let mut prev_id = id.0.saturating_sub(1);
            while self.path[EndpointId(prev_id)] == Default::default() && prev_id > 0 {
                prev_id -= 1;
            }

            if prev_id != 0 {
                let prev_id = EndpointId(prev_id);
                let from = self.path[prev_id]; // Lyon path point: (world-center)/res
                let to = self.path[id]; // Lyon path point: (world-center)/res
                let dx = from.x - to.x; // Diff in (world-center)/res units
                let dy = from.y - to.y; // Diff in (world-center)/res units
                                        // norm_limit should be in world units.
                                        // (dx*dx + dy*dy).sqrt() is length in (world-center)/res units.
                                        // Multiply by self.resolution to get world length.
                (dx * dx + dy * dy).sqrt() * 2.0 * self.resolution
            } else {
                f32::MAX
            }
        } else {
            f32::MAX
        };

        PolyVertex::new(
            [
                pos_x_relative_to_view_center,
                pos_y_relative_to_view_center,
                pos_z_relative_to_view_center,
            ],
            self.color,
            normal_for_shader,
            norm_limit,
        )
    }
}

struct PolygonVertexConstructor {
    color: [f32; 4],
}

impl FillVertexConstructor<PolyVertex> for PolygonVertexConstructor {
    fn new_vertex(&mut self, mut vertex: FillVertex) -> PolyVertex {
        // vertex.position() is from Lyon, relative to (0,0) in the space Lyon processed.
        // Path points given to Lyon were (world_coord - view_center_coord).
        // So, vertex.position() is already (world_coord - view_center_coord).
        // Z was also made relative.
        let pos_x_relative_to_view_center = vertex.position().x as f64;
        let pos_y_relative_to_view_center = vertex.position().y as f64;
        // Assuming interpolated_attributes()[0] for Z if used, or 0.0 if Z is constant for polygons.
        // The path builder for polygons was using first_point.z().as_() - cz.
        let pos_z_relative_to_view_center = vertex.interpolated_attributes()[0] as f64;

        PolyVertex::new(
            [
                pos_x_relative_to_view_center,
                pos_y_relative_to_view_center,
                pos_z_relative_to_view_center,
            ],
            self.color,
            Default::default(), // Polygons usually don't have screen-space normals like lines
            1.0,                // norm_limit, typically not used for filled polygons this way
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
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Serialize, Deserialize)]
pub(crate) struct PolyVertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
    pub normal: [f32; 2],
    pub norm_limit: f32,
}

impl PolyVertex {
    pub(crate) fn new(
        [x, y, z]: [f64; 3],
        color: [f32; 4],
        normal: [f32; 2],
        norm_limit: f32,
    ) -> Self {
        Self {
            position: [x as f32, y as f32, z as f32],
            color,
            normal,
            norm_limit,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Serialize, Deserialize)]
pub(crate) struct PointInstance {
    // TODO: could be f64
    pub position: [f32; 3],
    pub color: [u8; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Serialize, Deserialize)]
pub(crate) struct ImageVertex {
    // TODO: make it f64
    pub position: [f32; 2],
    pub opacity: f32,
    pub tex_coords: [f32; 2],
    pub offset: [f32; 2],
}
