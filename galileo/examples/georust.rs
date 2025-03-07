//! This exmample shows how to use geometries from `geo` crate as inputs for feature layers.

use galileo::layer::feature_layer::{FeatureLayer, FeatureLayerOptions};
use galileo::layer::raster_tile_layer::RasterTileLayerBuilder;
use galileo::symbol::ImagePointSymbol;
use galileo::{Map, MapBuilder};
use galileo_types::cartesian::Vector2;
use galileo_types::geo::Crs;
use galileo_types::geometry_type::GeoSpace2d;
use galileo_types::{Disambig, Disambiguate};
use geozero::geojson::GeoJson;
use geozero::ToGeo;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    run()
}

pub(crate) fn run() {
    galileo_egui::init(create_map(), []).expect("failed to initialize");
}

fn load_points() -> Vec<Disambig<geo_types::Point, GeoSpace2d>> {
    let json = include_str!("./data/Museums 2021.geojson");
    let geojson = GeoJson(json);
    match geojson.to_geo().expect("invalid geojson") {
        geo_types::Geometry::GeometryCollection(points) => points
            .iter()
            .map(|p| match p {
                geo_types::Geometry::Point(p) => p.to_geo2d(),
                _ => panic!("not points"),
            })
            .collect(),
        _ => panic!("not geometry collection"),
    }
}

fn create_map() -> Map {
    let symbol_image = include_bytes!("data/pin-yellow.png");
    let point_layer = FeatureLayer::new(
        load_points(),
        ImagePointSymbol::from_bytes(symbol_image, Vector2::new(0.5, 1.0), 0.5)
            .expect("invalid image file"),
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
        .with_latlon(53.732562, -1.863383)
        .with_resolution(50.0)
        .with_layer(raster_layer)
        .with_layer(point_layer)
        .build()
}
