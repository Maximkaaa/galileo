//! This exmample shows how to render `LineString` defined in a geojson `FeatureCollection` as a `Contour` in a `FeatureLayer`.

use galileo::layer::feature_layer::{FeatureLayer, FeatureLayerOptions};
use galileo::layer::raster_tile_layer::RasterTileLayerBuilder;
use galileo::symbol::SimpleContourSymbol;
use galileo::{Color, Map, MapBuilder};
use galileo_types::geo::impls::GeoPoint2d;
use galileo_types::geo::{Crs, NewGeoPoint};
use galileo_types::geometry_type::GeoSpace2d;
use galileo_types::impls::Contour;
use galileo_types::Disambig;
use geojson::FeatureCollection;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    run()
}

pub(crate) fn run() {
    galileo_egui::InitBuilder::new(create_map())
        .init()
        .expect("failed to initialize");
}

fn load_lines() -> Vec<Disambig<Contour<GeoPoint2d>, GeoSpace2d>> {
    let json = include_str!("./data/linestring.geojson");
    let collection: FeatureCollection = serde_json::from_str(json).expect("invalid geojson");

    collection
        .features
        .iter()
        .filter_map(|feature| {
            if let Some(ref geometry) = feature.geometry {
                match &geometry.value {
                    geojson::Value::LineString(coords) => {
                        let points: Vec<GeoPoint2d> = coords
                            .iter()
                            .map(|pos| NewGeoPoint::latlon(pos[1], pos[0]))
                            .collect();
                        Some(Disambig::new(Contour::open(points)))
                    }
                    _ => None,
                }
            } else {
                None
            }
        })
        .collect()
}

fn create_map() -> Map {
    let line_layer = FeatureLayer::new(
        load_lines(),
        SimpleContourSymbol {
            color: Color::BLACK,
            width: 8.0,
        },
        Crs::WGS84,
    )
    .with_options(FeatureLayerOptions {
        sort_by_depth: true,
        ..Default::default()
    });

    let raster_layer = RasterTileLayerBuilder::new_osm()
        .with_file_cache_checked(".tile_cache")
        .build()
        .expect("failed to create layer");

    MapBuilder::default()
        .with_latlon(37.830348, -122.486052)
        .with_resolution(1.5)
        .with_layer(raster_layer)
        .with_layer(line_layer)
        .build()
}
