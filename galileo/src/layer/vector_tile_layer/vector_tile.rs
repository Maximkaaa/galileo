use crate::error::GalileoError;
use crate::layer::vector_tile_layer::style::VectorTileStyle;
use crate::render::render_bundle::RenderBundle;
use crate::render::{LineCap, LinePaint, PackedBundle, PolygonPaint};
use crate::tile_scheme::{TileIndex, TileScheme};
use galileo_mvt::{MvtFeature, MvtGeometry, MvtTile};
use galileo_types::cartesian::impls::contour::{ClosedContour, Contour};
use galileo_types::cartesian::impls::point::Point3d;
use galileo_types::cartesian::impls::polygon::Polygon;
use galileo_types::cartesian::rect::Rect;
use galileo_types::cartesian::traits::cartesian_point::CartesianPoint2d;
use num_traits::ToPrimitive;

pub struct VectorTile {
    pub mvt_tile: MvtTile,
    pub bundle: Box<dyn PackedBundle>,
}

impl VectorTile {
    pub fn prepare(
        mvt_tile: &MvtTile,
        bundle: &mut RenderBundle,
        index: TileIndex,
        style: &VectorTileStyle,
        tile_scheme: &TileScheme,
    ) -> Result<(), GalileoError> {
        let bbox = tile_scheme.tile_bbox(index).unwrap();
        let lod_resolution = tile_scheme.lod_resolution(index.z).unwrap();
        let tile_resolution = lod_resolution * tile_scheme.tile_width() as f64;

        let bounds = Polygon::new(
            ClosedContour::new(vec![
                Point3d::new(bbox.x_min, bbox.y_min, 0.0),
                Point3d::new(bbox.x_min, bbox.y_max, 0.0),
                Point3d::new(bbox.x_max, bbox.y_max, 0.0),
                Point3d::new(bbox.x_max, bbox.y_min, 0.0),
            ]),
            vec![],
        );
        bundle.clip_area(&bounds);

        bundle.add_polygon(
            &bounds,
            PolygonPaint {
                color: style.background,
            },
            tile_resolution,
        );

        for layer in &mvt_tile.layers {
            for feature in &layer.features {
                match &feature.geometry {
                    MvtGeometry::Point(_points) => {
                        // todo
                        continue;
                    }
                    MvtGeometry::LineString(contours) => {
                        if let Some(paint) = Self::get_line_symbol(style, &layer.name, feature) {
                            for contour in contours {
                                bundle.add_line(
                                    &Contour {
                                        is_closed: false,
                                        points: contour
                                            .points
                                            .iter()
                                            .map(|p| {
                                                Self::transform_point(p, bbox, tile_resolution)
                                            })
                                            .collect(),
                                    },
                                    paint,
                                    tile_resolution,
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
                                    paint,
                                    tile_resolution,
                                );
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn get_line_symbol(
        style: &VectorTileStyle,
        layer_name: &str,
        feature: &MvtFeature,
    ) -> Option<LinePaint> {
        let Some(rule) = style.get_style_rule(layer_name, feature) else {
            let symbol = style.default_symbol.line.as_ref()?;
            return Some(LinePaint {
                width: symbol.width,
                color: symbol.stroke_color,
                offset: 0.0,
                line_cap: LineCap::Butt,
            });
        };

        let symbol = rule.symbol.line.as_ref()?;

        Some(LinePaint {
            width: symbol.width,
            color: symbol.stroke_color,
            offset: 0.0,
            line_cap: LineCap::Butt,
        })
    }

    fn get_polygon_symbol(
        style: &VectorTileStyle,
        layer_name: &str,
        feature: &MvtFeature,
    ) -> Option<PolygonPaint> {
        let Some(rule) = style.get_style_rule(layer_name, feature) else {
            return Some(PolygonPaint {
                color: style.default_symbol.polygon.as_ref()?.fill_color,
            });
        };

        Some(PolygonPaint {
            color: rule.symbol.polygon.as_ref()?.fill_color,
        })
    }

    fn transform_point<Num: num_traits::Float + ToPrimitive>(
        p_in: &impl CartesianPoint2d<Num = Num>,
        tile_bbox: Rect,
        tile_resolution: f64,
    ) -> Point3d {
        let x = tile_bbox.x_min() + p_in.x().to_f64().unwrap() * tile_resolution;
        let y = tile_bbox.y_max() - p_in.y().to_f64().unwrap() * tile_resolution;
        Point3d::new(x, y, 0.0)
    }
}
