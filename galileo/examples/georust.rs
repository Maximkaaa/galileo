//! This exmample shows how to use geometries from `geo` crate as inputs for feature layers.

use galileo::layer::feature_layer::{FeatureLayer, FeatureLayerOptions};
use galileo::symbol::ImagePointSymbol;
use galileo::tile_scheme::TileSchema;
use galileo::{Map, MapBuilder, MapView};
use galileo_types::geo::Crs;
use galileo_types::geometry_type::GeoSpace2d;
use galileo_types::{latlon, Disambig, Disambiguate};
use geozero::geojson::GeoJson;
use geozero::ToGeo;
use nalgebra::Vector2;

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

    let raster_layer = MapBuilder::create_raster_tile_layer(
        |index| {
            format!(
                "https://tile.openstreetmap.org/{}/{}/{}.png",
                index.z, index.x, index.y
            )
        },
        TileSchema::web(18),
    );

    Map::new(
        MapView::new(&latlon!(53.732562, -1.863383), 50.0),
        vec![Box::new(raster_layer), Box::new(point_layer)],
        None,
    )
}
