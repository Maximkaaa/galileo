use crate::decoded_image::DecodedImage;
use crate::error::GalileoError;
use crate::render::point_paint::{CircleFill, PointPaint, PointShape, SectorParameters};
use crate::render::render_bundle::RenderPrimitive;
use crate::render::text::{FontService, TextShaping, TextStyle};
use crate::render::{ImagePaint, LinePaint, PolygonPaint, PrimitiveId};
use crate::view::MapView;
use crate::Color;
use galileo_types::cartesian::{CartesianPoint2d, CartesianPoint3d, Point2d, Point3d};
use galileo_types::contour::Contour;
use galileo_types::impls::ClosedContour;
use galileo_types::Polygon;
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
use std::borrow::Borrow;
use std::mem::size_of;
use std::ops::Range;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub(crate) struct TessellatingRenderBundle {
    pub poly_tessellation: VertexBuffers<PolyVertex, u32>,
    pub points: Vec<PointInstance>,
    pub screen_ref: ScreenRefTessellation,
    pub images: Vec<ImageInfo>,
    pub clip_area: Option<VertexBuffers<PolyVertex, u32>>,
    pub image_store: Vec<ImageStoreInfo>,
    pub primitives: Vec<PrimitiveInfo>,
    vacant_ids: Vec<usize>,
    vacant_image_ids: Vec<usize>,
    vacant_image_store_ids: Vec<usize>,
    buffer_size: usize,
}

#[derive(Debug, Clone)]
pub(crate) enum ImageStoreInfo {
    Vacant,
    Image(Arc<DecodedImage>),
}

#[derive(Debug, Clone)]
pub(crate) enum ImageInfo {
    Vacant,
    Image((usize, [ImageVertex; 4])),
}

pub(crate) type ScreenRefTessellation = VertexBuffers<ScreenRefVertex, u32>;

#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub(crate) struct ScreenRefVertex {
    position: [f32; 3],
    normal: [f32; 2],
    color: [u8; 4],
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) enum PrimitiveInfo {
    None,
    Vacant,
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
            image_store: Vec::new(),
            vacant_ids: vec![],
            vacant_image_ids: vec![],
            vacant_image_store_ids: vec![],
            buffer_size: 0,
        }
    }

    pub fn approx_buffer_size(&self) -> usize {
        self.buffer_size
    }

    pub fn set_approx_buffer_size(&mut self, size: usize) {
        self.buffer_size = size;
    }

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

        self.buffer_size += tessellation.vertices.len() * std::mem::size_of::<PolyVertex>()
            + tessellation.indices.len() * std::mem::size_of::<u32>();

        self.clip_area = Some(tessellation);
    }

    pub fn add_image(
        &mut self,
        image: DecodedImage,
        vertices: [Point2d; 4],
        paint: ImagePaint,
    ) -> PrimitiveId {
        let opacity = paint.opacity as f32 / 255.0;

        self.buffer_size += image.bytes().len() + std::mem::size_of::<ImageVertex>() * 4;

        let index = self.add_image_to_store(Arc::new(image));
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

        let image_index = self.add_image_info(index, vertices);

        let id = self.primitives.len();

        self.primitives.push(PrimitiveInfo::Image { image_index });
        PrimitiveId(id)
    }

    fn add_image_point<N, P>(
        &mut self,
        position: &P,
        image: Arc<DecodedImage>,
        opacity: u8,
        width: f32,
        height: f32,
        offset: Vector2<f32>,
    ) -> PrimitiveInfo
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
    {
        let opacity = opacity as f32 / 255.0;

        self.buffer_size += image.bytes().len() + size_of::<ImageVertex>() * 4;

        let position = [position.x().as_(), position.y().as_()];
        let offset_x = -offset[0] * width;
        let offset_y = offset[1] * height;

        let index = self.add_image_to_store(image);
        let vertices = [
            ImageVertex {
                position,
                opacity,
                tex_coords: [0.0, 1.0],
                offset: [offset_x, offset_y - height],
            },
            ImageVertex {
                position,
                opacity,
                tex_coords: [0.0, 0.0],
                offset: [offset_x, offset_y],
            },
            ImageVertex {
                position,
                opacity,
                tex_coords: [1.0, 1.0],
                offset: [offset_x + width, offset_y - height],
            },
            ImageVertex {
                position,
                opacity,
                tex_coords: [1.0, 0.0],
                offset: [offset_x + width, offset_y],
            },
        ];

        let image_index = self.add_image_info(index, vertices);

        PrimitiveInfo::Image { image_index }
    }

    fn add_image_info(&mut self, image_store_index: usize, vertices: [ImageVertex; 4]) -> usize {
        if let Some(id) = self.vacant_image_ids.pop() {
            self.images[id] = ImageInfo::Image((image_store_index, vertices));
            id
        } else {
            let index = self.images.len();
            self.images
                .push(ImageInfo::Image((image_store_index, vertices)));
            index
        }
    }

    fn add_primitive_info(&mut self, info: PrimitiveInfo) -> PrimitiveId {
        if let Some(id) = self.vacant_ids.pop() {
            self.primitives[id] = info;
            PrimitiveId(id)
        } else {
            let id = self.primitives.len();
            self.primitives.push(info);
            PrimitiveId(id)
        }
    }

    fn add_image_to_store(&mut self, image: Arc<DecodedImage>) -> usize {
        for (i, stored) in self.image_store.iter().enumerate() {
            match stored {
                ImageStoreInfo::Vacant => {}
                ImageStoreInfo::Image(stored) => {
                    if Arc::ptr_eq(stored, &image) {
                        return i;
                    }
                }
            }
        }

        if let Some(id) = self.vacant_image_store_ids.pop() {
            self.image_store[id] = ImageStoreInfo::Image(image);
            id
        } else {
            let index = self.image_store.len();
            self.image_store.push(ImageStoreInfo::Image(image));
            index
        }
    }

    pub fn add<N, P, C, Poly>(
        &mut self,
        primitive: RenderPrimitive<N, P, C, Poly>,
        min_resolution: f64,
    ) -> PrimitiveId
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N> + Clone,
        C: Contour<Point = P> + Clone,
        Poly: Polygon + Clone,
        Poly::Contour: Contour<Point = P>,
    {
        match primitive {
            RenderPrimitive::Point(point, paint) => self.add_point::<N, P>(point.borrow(), &paint),
            RenderPrimitive::Contour(contour, paint) => {
                self.add_line::<N, P, C>(contour.borrow(), paint, min_resolution)
            }
            RenderPrimitive::Polygon(polygon, paint) => {
                self.add_polygon::<N, P, Poly>(polygon.borrow(), paint, min_resolution)
            }
        }
    }

    pub fn update<N, P, C, Poly>(
        &mut self,
        primitive_id: PrimitiveId,
        primitive: RenderPrimitive<N, P, C, Poly>,
    ) -> Result<(), GalileoError>
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N> + Clone,
        C: Contour<Point = P> + Clone,
        Poly: Polygon + Clone,
        Poly::Contour: Contour<Point = P>,
    {
        if primitive_id.0 >= self.primitives.len() {
            return Err(GalileoError::Generic(
                "no primitive with the given id".into(),
            ));
        }

        let info = &self.primitives[primitive_id.0];

        match info {
            PrimitiveInfo::MapRef { vertex_range } => {
                self.update_map_ref(vertex_range.clone(), primitive)
            }
            PrimitiveInfo::Vacant => Ok(()),
            _ => todo!(),
        }
    }

    pub fn remove(&mut self, primitive_id: PrimitiveId) -> Result<(), GalileoError> {
        if primitive_id.0 >= self.primitives.len() {
            return Err(GalileoError::Generic(
                "no primitive with the given id".into(),
            ));
        }

        let info = std::mem::replace(&mut self.primitives[primitive_id.0], PrimitiveInfo::Vacant);

        match info {
            PrimitiveInfo::MapRef { vertex_range } => self.remove_map_ref(vertex_range),
            PrimitiveInfo::ScreenRef { vertex_range } => self.remove_screen_ref(vertex_range),
            PrimitiveInfo::Dot { point_index } => self.remove_dot(point_index),
            PrimitiveInfo::Image { image_index } => self.remove_image(image_index),
            PrimitiveInfo::Vacant => Ok(()),
            PrimitiveInfo::None => Ok(()),
        }
    }

    fn remove_image(&mut self, index: usize) -> Result<(), GalileoError> {
        if index >= self.images.len() {
            Err(GalileoError::Generic("index out of bounds".into()))
        } else {
            let image_id = match std::mem::replace(&mut self.images[index], ImageInfo::Vacant) {
                ImageInfo::Vacant => {
                    // this should not happen
                    return Err(GalileoError::Generic(
                        "tried to replace vacant image with vacant slot".into(),
                    ));
                }
                ImageInfo::Image((image_id, _)) => {
                    self.vacant_image_ids.push(index);
                    image_id
                }
            };

            let stored_image_unused = self.images.iter().all(|info| match info {
                ImageInfo::Vacant => false,
                ImageInfo::Image((i, _)) => *i != image_id,
            });

            if stored_image_unused {
                match std::mem::replace(&mut self.image_store[image_id], ImageStoreInfo::Vacant) {
                    ImageStoreInfo::Vacant => {
                        // this should not happen
                    }
                    ImageStoreInfo::Image(image) => {
                        self.vacant_image_store_ids.push(image_id);

                        self.buffer_size -= image.bytes.len() + size_of::<ImageVertex>() * 4;
                    }
                }
            }

            for info in &mut self.primitives {
                match info {
                    PrimitiveInfo::Dot {
                        point_index: ref mut image_index,
                    } if *image_index > index => {
                        *image_index -= 1;
                    }
                    _ => {}
                }
            }

            Ok(())
        }
    }

    fn remove_dot(&mut self, index: usize) -> Result<(), GalileoError> {
        if index >= self.points.len() {
            Err(GalileoError::Generic("index out of bounds".into()))
        } else {
            self.points.remove(index);

            self.buffer_size -= size_of::<PointInstance>();

            for info in &mut self.primitives {
                match info {
                    PrimitiveInfo::Dot {
                        ref mut point_index,
                    } if *point_index > index => {
                        *point_index -= 1;
                    }
                    _ => {}
                }
            }

            Ok(())
        }
    }

    fn remove_screen_ref(&mut self, range: Range<usize>) -> Result<(), GalileoError> {
        let removed_index_count =
            Self::remove_from_tessellation(&mut self.screen_ref, range.clone())?;
        let len = range.len();
        self.buffer_size -=
            size_of::<ScreenRefVertex>() * len + size_of::<u32>() * removed_index_count;

        for info in &mut self.primitives {
            match info {
                PrimitiveInfo::ScreenRef {
                    ref mut vertex_range,
                } if vertex_range.start >= range.end => {
                    vertex_range.start -= len;
                    vertex_range.end -= len;
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn remove_map_ref(&mut self, range: Range<usize>) -> Result<(), GalileoError> {
        let removed_index_count =
            Self::remove_from_tessellation(&mut self.poly_tessellation, range.clone())?;
        let len = range.len();
        self.buffer_size -= size_of::<PolyVertex>() * len + size_of::<u32>() * removed_index_count;

        for info in &mut self.primitives {
            match info {
                PrimitiveInfo::MapRef {
                    ref mut vertex_range,
                } if vertex_range.start >= range.end => {
                    vertex_range.start -= len;
                    vertex_range.end -= len;
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn remove_from_tessellation<T>(
        tessellation: &mut VertexBuffers<T, u32>,
        range: Range<usize>,
    ) -> Result<usize, GalileoError> {
        if range.is_empty() {
            return Ok(0);
        }

        let len = range.len() as u32;
        let start = range.start as u32;
        let end = range.end as u32;

        if range.end > tessellation.vertices.len() {
            return Err(GalileoError::Generic("range out of bounds".into()));
        }

        tessellation.vertices.drain(range);
        let length_before = tessellation.indices.len();
        tessellation.indices = tessellation
            .indices
            .iter()
            .filter_map(|index| match *index {
                i if i < start => Some(i),
                i if i >= end => Some(i - len),
                _ => None,
            })
            .collect();
        let length_after = tessellation.indices.len();

        Ok(length_before - length_after)
    }

    pub fn add_point<N, P>(&mut self, point: &P, paint: &PointPaint) -> PrimitiveId
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
    {
        let start_index = self.screen_ref.vertices.len();
        let info = match &paint.shape {
            PointShape::Dot { color } => {
                self.add_dot(point, *color, paint.offset);
                PrimitiveInfo::Dot {
                    point_index: self.points.len() - 1,
                }
            }
            PointShape::Image {
                image,
                opacity,
                width,
                height,
            } => self.add_image_point(
                point,
                image.clone(),
                *opacity,
                *width,
                *height,
                paint.offset,
            ),
            PointShape::Circle {
                fill,
                radius,
                outline,
            } => {
                self.add_circle(point, *fill, *radius, *outline, paint.offset);
                PrimitiveInfo::ScreenRef {
                    vertex_range: start_index..self.screen_ref.vertices.len(),
                }
            }
            PointShape::Sector(parameters) => {
                self.add_circle_sector(point, *parameters, paint.offset);
                PrimitiveInfo::ScreenRef {
                    vertex_range: start_index..self.screen_ref.vertices.len(),
                }
            }
            PointShape::Square {
                fill,
                size,
                outline,
            } => {
                self.add_shape(point, *fill, *size, *outline, &square_shape(), paint.offset);
                PrimitiveInfo::ScreenRef {
                    vertex_range: start_index..self.screen_ref.vertices.len(),
                }
            }
            PointShape::FreeShape {
                fill,
                scale,
                outline,
                shape,
            } => {
                self.add_shape(point, *fill, *scale, *outline, shape, paint.offset);
                PrimitiveInfo::ScreenRef {
                    vertex_range: start_index..self.screen_ref.vertices.len(),
                }
            }
            PointShape::Label { text, style } => self.add_label(point, text, style, paint.offset),
        };

        self.add_primitive_info(info)
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

        self.add_primitive_info(PrimitiveInfo::MapRef {
            vertex_range: range,
        })
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
        let start_index_count = tessellation.indices.len();

        if let Err(err) = tesselator.tessellate_path(
            &path,
            &StrokeOptions::DEFAULT
                .with_line_cap(paint.line_cap.into())
                .with_line_width(paint.width as f32)
                .with_miter_limit(1.0)
                .with_tolerance(0.1)
                .with_line_join(LineJoin::Round),
            &mut BuffersBuilder::new(tessellation, vertex_constructor),
        ) {
            log::error!("Tessellation failed: {err}");
            return 0..0;
        }

        let end_index = tessellation.vertices.len();

        self.buffer_size += (end_index - start_index) * size_of::<PolyVertex>();
        self.buffer_size += (tessellation.indices.len() - start_index_count) * size_of::<u32>();

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
        self.add_primitive_info(PrimitiveInfo::MapRef { vertex_range })
    }

    pub fn modify_image(&mut self, id: PrimitiveId, paint: ImagePaint) -> Result<(), GalileoError> {
        let info = self
            .primitives
            .get(id.0)
            .ok_or(GalileoError::Generic("primitive does not exist".into()))?;
        match info {
            PrimitiveInfo::Image { image_index } => {
                match self
                    .images
                    .get_mut(*image_index)
                    .ok_or(GalileoError::Generic("invalid image id".into()))?
                {
                    ImageInfo::Vacant => {
                        return Err(GalileoError::Generic("tried to modify vacant image".into()))
                    }
                    ImageInfo::Image((_, vertices)) => {
                        for vertex in vertices {
                            vertex.opacity = paint.opacity as f32 / 255.0;
                        }
                    }
                }
            }
            _ => return Err(GalileoError::Generic("invalid primitive type".into())),
        }

        Ok(())
    }

    fn update_map_ref<N, P, C, Poly>(
        &mut self,
        range: Range<usize>,
        primitive: RenderPrimitive<N, P, C, Poly>,
    ) -> Result<(), GalileoError>
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N> + Clone,
        C: Contour<Point = P> + Clone,
        Poly: Polygon + Clone,
        Poly::Contour: Contour<Point = P>,
    {
        let color = match primitive {
            RenderPrimitive::Contour(_, LinePaint { color, .. })
            | RenderPrimitive::Polygon(_, PolygonPaint { color }) => color,
            _ => {
                return Err(GalileoError::Generic(
                    "expected line or polygon primitive, but got a point".into(),
                ));
            }
        };

        for vertex in &mut self.poly_tessellation.vertices[range] {
            vertex.color = color.to_f32_array();
        }

        Ok(())
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
        let start_index_count = lod.indices.len();

        Self::tessellate_polygon(polygon, paint, lod);

        let end_index = lod.vertices.len();

        self.buffer_size += (end_index - start_index) * size_of::<PolyVertex>();
        self.buffer_size += (lod.indices.len() - start_index_count) * size_of::<u32>();

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

        let start_vertex_count = self.screen_ref.vertices.len();
        let start_index_count = self.screen_ref.indices.len();

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

        self.buffer_size += (self.screen_ref.vertices.len() - start_vertex_count)
            * std::mem::size_of::<ScreenRefVertex>();
        self.buffer_size +=
            (self.screen_ref.indices.len() - start_index_count) * std::mem::size_of::<u32>();
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

        let start_vertex_count = self.screen_ref.vertices.len();
        let start_index_count = self.screen_ref.indices.len();

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

        self.buffer_size += (self.screen_ref.vertices.len() - start_vertex_count)
            * std::mem::size_of::<ScreenRefVertex>();
        self.buffer_size +=
            (self.screen_ref.indices.len() - start_index_count) * std::mem::size_of::<u32>();
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
        });
        self.buffer_size += size_of::<PointInstance>();
    }

    pub fn sort_by_depth(&mut self, view: &MapView) {
        self.sort_images_by_depth(view);
    }

    pub fn sort_images_by_depth(&mut self, view: &MapView) {
        let Some(transform) = view.map_to_scene_transform() else {
            return;
        };
        self.images.sort_by(|info_a, info_b| {
            let point_a = match info_a {
                ImageInfo::Vacant => Point3d::new(0.0, 0.0, 0.0).to_homogeneous(),
                ImageInfo::Image((_, vertex_set_a)) => Point3d::new(
                    vertex_set_a[0].position[0] as f64,
                    vertex_set_a[0].position[1] as f64,
                    0.0,
                )
                .to_homogeneous(),
            };

            let point_b = match info_b {
                ImageInfo::Vacant => Point3d::new(0.0, 0.0, 0.0).to_homogeneous(),
                ImageInfo::Image((_, vertex_set_b)) => Point3d::new(
                    vertex_set_b[0].position[0] as f64,
                    vertex_set_b[0].position[1] as f64,
                    0.0,
                )
                .to_homogeneous(),
            };

            let projected_a = transform * point_a;
            let projected_b = transform * point_b;

            projected_b.z.total_cmp(&projected_a.z)
        });
    }

    fn add_label<N, P>(
        &mut self,
        position: &P,
        text: &str,
        style: &TextStyle,
        offset: Vector2<f32>,
    ) -> PrimitiveInfo
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
    {
        FontService::with(
            |font_service| match font_service.shape(text, style, offset) {
                Ok(TextShaping::Tessellation { glyphs, .. }) => {
                    let indices_start = self.screen_ref.indices.len();

                    for glyph in glyphs {
                        let vertices_start = self.screen_ref.vertices.len() as u32;
                        for vertex in glyph.vertices {
                            self.screen_ref.vertices.push(ScreenRefVertex {
                                position: [
                                    position.x().as_(),
                                    position.y().as_(),
                                    position.z().as_(),
                                ],
                                normal: vertex,
                                color: style.font_color.to_u8_array(),
                            });
                        }
                        for index in glyph.indices {
                            self.screen_ref.indices.push(index + vertices_start);
                        }
                    }

                    PrimitiveInfo::ScreenRef {
                        vertex_range: indices_start..self.screen_ref.indices.len(),
                    }
                }
                Err(err) => {
                    log::error!("Error shaping text label: {err:?}");
                    PrimitiveInfo::None
                }
                _ => {
                    log::error!("Not supported font type");
                    PrimitiveInfo::None
                }
            },
        )
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
pub(crate) struct PolyVertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
    pub normal: [f32; 2],
    pub norm_limit: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct PointInstance {
    pub position: [f32; 3],
    pub color: [u8; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct ImageVertex {
    pub position: [f32; 2],
    pub opacity: f32,
    pub tex_coords: [f32; 2],
    pub offset: [f32; 2],
}

#[cfg(target_arch = "wasm32")]
pub(crate) mod serialization;

#[cfg(test)]
mod tests {
    use super::*;

    type C = galileo_types::impls::Contour<Point3d>;

    #[test]
    fn remove_map_ref() {
        let mut bundle = TessellatingRenderBundle::new();
        let polygon = galileo_types::impls::Polygon::from(vec![
            Point3d::new(0.0, 0.0, 0.0),
            Point3d::new(1.0, 0.0, 0.0),
            Point3d::new(1.0, 1.0, 0.0),
            Point3d::new(0.0, 1.0, 0.0),
        ]);
        let paint1 = PolygonPaint {
            color: Color::BLACK,
        };
        let paint2 = PolygonPaint { color: Color::RED };

        let _id0 = bundle.add(
            RenderPrimitive::<_, _, C, _>::new_polygon_ref(&polygon, paint1),
            1.0,
        );
        let id1 = bundle.add(
            RenderPrimitive::<_, _, C, _>::new_polygon_ref(&polygon, paint2),
            1.0,
        );
        let id2 = bundle.add(
            RenderPrimitive::<_, _, C, _>::new_polygon_ref(&polygon, paint1),
            1.0,
        );

        let vertex_range = 0..bundle.poly_tessellation.vertices.len();

        bundle.remove(id1).unwrap();

        assert!(bundle
            .poly_tessellation
            .vertices
            .iter()
            .all(|v| v.color == Color::BLACK.to_f32_array()));
        assert!(bundle
            .poly_tessellation
            .indices
            .iter()
            .all(|v| vertex_range.contains(&(*v as usize))));

        let vertex_count = bundle.poly_tessellation.vertices.len();
        let PrimitiveInfo::MapRef { vertex_range } = bundle.primitives[id2.0].clone() else {
            panic!("invalid primitive type");
        };

        assert_eq!(vertex_range.end, vertex_count);
    }
}
