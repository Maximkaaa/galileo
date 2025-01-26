use crate::error::GalileoError;
use crate::layer::data_provider::DataProcessor;
use crate::layer::vector_tile_layer::style::{VectorTileLabelSymbol, VectorTileStyle};
use crate::render::point_paint::PointPaint;
use crate::render::render_bundle::{RenderBundle, RenderPrimitive};
use crate::render::{LinePaint, PolygonPaint};
use crate::tile_scheme::TileIndex;
use crate::TileSchema;
use bytes::Bytes;
use galileo_mvt::{MvtFeature, MvtGeometry, MvtTile};
use galileo_types::cartesian::{CartesianPoint2d, Point3d, Rect};
use galileo_types::impls::{ClosedContour, Polygon};
use galileo_types::Contour;
use num_traits::ToPrimitive;
use strfmt::strfmt;

/// Data processor that decodes vector tiles.
pub struct VtProcessor {}

/// Vector tiles decoding context.
pub struct VectorTileDecodeContext {
    /// Index of the tile.
    pub index: TileIndex,
    /// Vector tile layer style.
    pub style: VectorTileStyle,
    /// Vector tile layer tile schema.
    pub tile_schema: TileSchema,
    /// Render bundle to add render primitives to.
    pub bundle: RenderBundle,
}

impl DataProcessor for VtProcessor {
    type Input = Bytes;
    type Output = (RenderBundle, MvtTile);
    type Context = VectorTileDecodeContext;

    fn process(
        &self,
        input: Self::Input,
        context: Self::Context,
    ) -> Result<Self::Output, GalileoError> {
        let start = std::time::Instant::now();
        let mvt_tile = MvtTile::decode(input, false)?;
        let mvt_decoded_in = start.elapsed();
        let VectorTileDecodeContext {
            mut bundle,
            index,
            style,
            tile_schema: tile_scheme,
        } = context;
        Self::prepare(&mvt_tile, &mut bundle, index, &style, &tile_scheme)?;
        let prerendered_in = start.elapsed() - mvt_decoded_in;

        log::info!(
            "Decoded tile in {} ms, prerendered in {} ms",
            mvt_decoded_in.as_millis(),
            prerendered_in.as_millis()
        );

        Ok((bundle, mvt_tile))
    }
}

impl VtProcessor {
    /// Pre-render the given tile into the given `bundle`.
    pub fn prepare(
        mvt_tile: &MvtTile,
        bundle: &mut RenderBundle,
        index: TileIndex,
        style: &VectorTileStyle,
        tile_scheme: &TileSchema,
    ) -> Result<(), GalileoError> {
        let bbox = tile_scheme
            .tile_bbox(index)
            .ok_or_else(|| GalileoError::Generic("cannot get tile bbox".into()))?;
        let lod_resolution = tile_scheme.lod_resolution(index.z).ok_or_else(|| {
            GalileoError::Generic(format!("cannot get lod resolution for lod {}", index.z))
        })?;
        let tile_resolution = lod_resolution * tile_scheme.tile_width() as f64;

        let bounds = Polygon::new(
            ClosedContour::new(vec![
                Point3d::new(bbox.x_min(), bbox.y_min(), 0.0),
                Point3d::new(bbox.x_min(), bbox.y_max(), 0.0),
                Point3d::new(bbox.x_max(), bbox.y_max(), 0.0),
                Point3d::new(bbox.x_max(), bbox.y_min(), 0.0),
            ]),
            vec![],
        );
        bundle.clip_area(&bounds);

        for layer in mvt_tile.layers.iter().rev() {
            for feature in &layer.features {
                match &feature.geometry {
                    MvtGeometry::Point(points) => {
                        let Some(paint) = Self::get_point_symbol(style, &layer.name, feature)
                        else {
                            continue;
                        };

                        for point in points {
                            bundle.add(RenderPrimitive::<_, _, galileo_types::impls::Contour<_>, Polygon<_>>::new_point_ref(&Self::transform_point(point, bbox, tile_resolution), &paint), lod_resolution);
                        }
                    }
                    MvtGeometry::LineString(contours) => {
                        if let Some(paint) = Self::get_line_symbol(style, &layer.name, feature) {
                            for contour in contours {
                                bundle.add(
                                    RenderPrimitive::<_, _, _, Polygon<_>>::new_contour_ref(
                                        &galileo_types::impls::Contour::new(
                                            contour
                                                .iter_points()
                                                .map(|p| {
                                                    Self::transform_point(p, bbox, tile_resolution)
                                                })
                                                .collect(),
                                            false,
                                        ),
                                        paint,
                                    ),
                                    lod_resolution,
                                );
                            }
                        }
                    }
                    MvtGeometry::Polygon(polygons) => {
                        if let Some(paint) = Self::get_polygon_symbol(style, &layer.name, feature) {
                            for polygon in polygons {
                                bundle.add(
                                    RenderPrimitive::<_, _, galileo_types::impls::Contour<_>, _>::new_polygon_ref(
                                        &polygon.cast_points(|p| {
                                            Self::transform_point(p, bbox, tile_resolution)
                                        }),
                                        paint,
                                    ),
                                    lod_resolution,
                                );
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn get_point_symbol<'a>(
        style: &'a VectorTileStyle,
        layer_name: &str,
        feature: &MvtFeature,
    ) -> Option<PointPaint<'a>> {
        style
            .get_style_rule(layer_name, feature)
            .and_then(|rule| {
                rule.symbol
                    .point()
                    .copied()
                    .map(|symbol| symbol.into())
                    .or_else(|| {
                        rule.symbol
                            .label()
                            .and_then(|symbol| Self::format_label(symbol, feature))
                    })
            })
            .or_else(|| {
                style
                    .default_symbol
                    .point
                    .map(|symbol| symbol.into())
                    .or_else(|| {
                        style
                            .default_symbol
                            .label
                            .as_ref()
                            .and_then(|symbol| Self::format_label(symbol, feature))
                    })
            })
    }

    fn format_label<'a>(
        label_symbol: &VectorTileLabelSymbol,
        feature: &MvtFeature,
    ) -> Option<PointPaint<'a>> {
        let text = strfmt(&label_symbol.pattern, &feature.properties).ok()?;
        Some(PointPaint::label_owned(
            text,
            label_symbol.text_style.clone(),
        ))
    }

    fn get_line_symbol(
        style: &VectorTileStyle,
        layer_name: &str,
        feature: &MvtFeature,
    ) -> Option<LinePaint> {
        style
            .get_style_rule(layer_name, feature)
            .and_then(|rule| rule.symbol.line().copied())
            .or(style.default_symbol.line)
            .map(|symbol| symbol.into())
    }

    fn get_polygon_symbol(
        style: &VectorTileStyle,
        layer_name: &str,
        feature: &MvtFeature,
    ) -> Option<PolygonPaint> {
        style
            .get_style_rule(layer_name, feature)
            .and_then(|rule| rule.symbol.polygon().copied())
            .or(style.default_symbol.polygon)
            .map(|symbol| symbol.into())
    }

    fn transform_point<Num: num_traits::Float + ToPrimitive>(
        p_in: &impl CartesianPoint2d<Num = Num>,
        tile_bbox: Rect,
        tile_resolution: f64,
    ) -> Point3d {
        let x = tile_bbox.x_min() + p_in.x().to_f64().expect("double overflow") * tile_resolution;
        let y = tile_bbox.y_max() - p_in.y().to_f64().expect("double overflow") * tile_resolution;
        Point3d::new(x, y, 0.0)
    }
}
