use galileo::layer::feature_layer::{FeatureLayer, FeatureLayerOptions};
use galileo::symbol::ImagePointSymbol;
use galileo::tile_scheme::TileSchema;
use galileo::MapBuilder;
use galileo_types::geo::Crs;
use galileo_types::geometry_type::GeoSpace2d;
use galileo_types::{latlon, Disambig, Disambiguate};
use geozero::geojson::GeoJson;
use geozero::ToGeo;
use nalgebra::Vector2;

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    run(MapBuilder::new()).await;
}

fn load_points() -> Vec<Disambig<geo_types::Point, GeoSpace2d>> {
    let json = include_str!("./data/Museums 2021.geojson");
    let geojson = GeoJson(json);
    match geojson.to_geo().unwrap() {
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

pub async fn run(builder: MapBuilder) {
    let point_layer = FeatureLayer::new(
        load_points(),
        ImagePointSymbol::from_path(
            "galileo/examples/data/pin-yellow.png",
            Vector2::new(0.5, 1.0),
            0.5,
        )
        .unwrap(),
        Crs::WGS84,
    )
    .with_options(FeatureLayerOptions {
        sort_by_depth: true,
        ..Default::default()
    });

    builder
        .center(latlon!(53.732562, -1.863383))
        .resolution(50.0)
        .with_raster_tiles(
            |index| {
                format!(
                    "https://tile.openstreetmap.org/{}/{}/{}.png",
                    index.z, index.x, index.y
                )
            },
            TileSchema::web(18),
        )
        .with_layer(point_layer)
        .build()
        .await
        .run();
}
