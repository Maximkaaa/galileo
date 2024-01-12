use galileo::galileo_map::MapBuilder;
use galileo::layer::feature_layer::symbol::point::CirclePointSymbol;
use galileo::layer::feature_layer::FeatureLayer;
use galileo::primitives::Color;
use galileo::tile_scheme::TileScheme;
use galileo_types::disambig::{Disambig, Disambiguate};
use galileo_types::geo::crs::Crs;
use galileo_types::geometry_type::GeoSpace2d;
use galileo_types::latlon;
use geozero::geojson::GeoJson;
use geozero::ToGeo;

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
        CirclePointSymbol::new(Color::RED, 10.0),
        Crs::WGS84,
    );

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
            TileScheme::web(18),
        )
        .with_layer(point_layer)
        .build()
        .await
        .run();
}
