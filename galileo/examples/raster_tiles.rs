use galileo::galileo_map::MapBuilder;
use galileo::tile_scheme::TileSchema;
use galileo_types::latlon;

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    run(MapBuilder::new()).await;
}

pub async fn run(builder: MapBuilder) {
    builder
        .center(latlon!(37.566, 126.9784))
        .resolution(TileSchema::web(18).lod_resolution(8).unwrap())
        .with_raster_tiles(
            |index| {
                format!(
                    "https://tile.openstreetmap.org/{}/{}/{}.png",
                    index.z, index.x, index.y
                )
            },
            TileSchema::web(18),
        )
        .build()
        .await
        .run();
}
