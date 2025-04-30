use galileo_mvt::{MvtFeature, MvtGeometry, MvtTile};
use galileo_types::cartesian::{CartesianPoint2d, CartesianPoint3d, Point2, Point3, Rect, Vector2};
use galileo_types::impls::{ClosedContour, Polygon};
use galileo_types::Contour;
use num_traits::ToPrimitive;
use regex::Regex;

use crate::error::GalileoError;
use crate::layer::vector_tile_layer::style::{VectorTileLabelSymbol, VectorTileStyle};
use crate::render::point_paint::{PointPaint, PointShape};
use crate::render::render_bundle::RenderBundle;
use crate::render::{LinePaint, PolygonPaint};
use crate::tile_schema::TileIndex;
use crate::TileSchema;

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

impl VtProcessor {
    /// Pre-render the given tile into the given `bundle`.
    pub fn prepare(
        mvt_tile: &MvtTile,
        bundle: &mut RenderBundle,
        index: TileIndex,
        style: &VectorTileStyle,
        tile_schema: &TileSchema,
    ) -> Result<(), GalileoError> {
        let bbox = tile_schema
            .tile_bbox(index)
            .ok_or_else(|| GalileoError::Generic("cannot get tile bbox".into()))?;
        let lod_resolution = tile_schema.lod_resolution(index.z).ok_or_else(|| {
            GalileoError::Generic(format!("cannot get lod resolution for lod {}", index.z))
        })?;
        let tile_resolution = lod_resolution * tile_schema.tile_width() as f64;

        let bounds = Polygon::new(
            ClosedContour::new(vec![
                Point3::new(bbox.x_min(), bbox.y_min(), 0.0),
                Point3::new(bbox.x_min(), bbox.y_max(), 0.0),
                Point3::new(bbox.x_max(), bbox.y_max(), 0.0),
                Point3::new(bbox.x_max(), bbox.y_min(), 0.0),
            ]),
            vec![],
        );
        bundle.world_set.clip_area(&bounds);

        for layer in mvt_tile.layers.iter().rev() {
            for feature in &layer.features {
                match &feature.geometry {
                    MvtGeometry::Point(points) => {
                        let Some(paint) = Self::get_point_symbol(style, &layer.name, feature)
                        else {
                            continue;
                        };

                        for point in points {
                            let position = Self::transform_point(point, bbox, tile_resolution);
                            if !bbox.contains(&Point2::new(position.x(), position.y())) {
                                // Some vector tiles add out-of-bounds point to start displaying labels that
                                // are not fully on the screen yet. We need to deal with that case
                                // in some clever way, but for now let's ignore those points.
                                continue;
                            }

                            match &paint.shape {
                                PointShape::Label { text, style } => {
                                    bundle.add_label(
                                        &position,
                                        text,
                                        style,
                                        Vector2::default(),
                                        false,
                                    );
                                }
                                _ => {
                                    bundle.add_point(&position, &paint, lod_resolution);
                                }
                            }
                        }
                    }
                    MvtGeometry::LineString(contours) => {
                        if let Some(paint) = Self::get_line_symbol(style, &layer.name, feature) {
                            for contour in contours {
                                bundle.add_line(
                                    &galileo_types::impls::Contour::new(
                                        contour
                                            .iter_points()
                                            .map(|p| {
                                                Self::transform_point(p, bbox, tile_resolution)
                                            })
                                            .collect(),
                                        false,
                                    ),
                                    &paint,
                                    lod_resolution,
                                );
                            }
                        }
                    }
                    MvtGeometry::Polygon(polygons) => {
                        if let Some(paint) = Self::get_polygon_symbol(style, &layer.name, feature) {
                            for polygon in polygons {
                                bundle.add_polygon(
                                    &polygon.cast_points(|p| {
                                        Self::transform_point(p, bbox, tile_resolution)
                                    }),
                                    &paint,
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
        let re = Regex::new("\\{(?<name>.+)}").ok()?;
        let mut text = label_symbol.pattern.to_string();
        for m in re.captures_iter(&label_symbol.pattern) {
            let prop_name = &m["name"];
            let prop = feature
                .properties
                .get(prop_name)
                .map(|v| v.to_string())
                .unwrap_or_default();

            text = text.replace(&format!("{{{prop_name}}}"), &prop);
        }
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
    ) -> Point3 {
        let x = tile_bbox.x_min() + p_in.x().to_f64().expect("double overflow") * tile_resolution;
        let y = tile_bbox.y_max() - p_in.y().to_f64().expect("double overflow") * tile_resolution;
        Point3::new(x, y, 0.0)
    }
}
