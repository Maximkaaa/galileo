//! This example shows how to render a map to an image file without creating a window.
//!
//! Run this example with one argument - path to a `.geojson` file to plot. Running it will create
//! a file `output_map.png` with the plotted GEOJSON with OSM background.
//!
//! ```shell
//! cargo run --example render_to_file --features geojson -- "./galileo/examples/data/Museums 2021.geojson"
//! ```

use anyhow::{anyhow, Result};
use galileo::layer::data_provider::file_cache::FileCacheController;
use galileo::layer::data_provider::url_image_provider::UrlImageProvider;
use galileo::layer::{FeatureLayer, RasterTileLayer};
use galileo::map::Map;
use galileo::messenger::{DummyMessenger, Messenger};
use galileo::render::wgpu::WgpuRenderer;
use galileo::symbol::arbitrary::ArbitraryGeometrySymbol;
use galileo::tile_scheme::TileIndex;
use galileo::view::MapView;
use galileo::TileSchema;
use galileo_types::cartesian::size::Size;
use galileo_types::geo::crs::Crs;
use geojson::{FeatureCollection, GeoJson};
use image::{ImageBuffer, Rgba};
use std::sync::Arc;
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    if std::env::args().count() != 2 {
        return Err(anyhow!(
            "This example must be run with one argument - name of the .geojson file to load"
        ));
    }

    let file_name = std::env::args().skip(1).next().unwrap();
    let json = &std::fs::read_to_string(file_name)?;
    let geojson = json.parse::<GeoJson>()?;
    let collection = FeatureCollection::try_from(geojson)?;

    // We can give GEOJSON features directly to a feature layer, as `geo-json` feature provides
    // implementation of `Feature` trait for GEOJSON features and of `Geometry` trait for
    // GEOJSON geometries.
    //
    // All GEOJSON files contain data in Wgs84, so we specify this CRS for the layer.
    let layer = FeatureLayer::new(
        collection.features,
        ArbitraryGeometrySymbol::default(),
        Crs::WGS84,
    );

    // To calculate the area of the map which we want to draw, we use map's CRS instead of
    // layer CRS.
    let extent = layer.extent_projected(&Crs::EPSG3857).unwrap();
    let center = extent.center();

    let image_size = Size::new(512, 512);

    let width_resolution = extent.width() / image_size.width() as f64;
    let height_resolution = extent.height() / image_size.height() as f64;
    let resolution = (width_resolution.max(height_resolution) * 1.1)
        .max(TileSchema::web(18).lod_resolution(17).unwrap());

    // Create OSM layer for background
    let cache_controller = Some(FileCacheController::new(".tile_cache"));
    let tile_provider = UrlImageProvider::new(
        |index: &TileIndex| {
            format!(
                "https://tile.openstreetmap.org/{}/{}/{}.png",
                index.z, index.x, index.y
            )
        },
        cache_controller,
    );
    let mut osm = RasterTileLayer::new(
        TileSchema::web(18),
        tile_provider,
        None::<Arc<dyn Messenger>>,
    );

    // If we don't set fade in duration to 0, when the image is first drawn, all tiles will
    // be transparent.
    osm.set_fade_in_duration(Duration::default());

    let map_view = MapView::new_projected(&center, resolution).with_size(image_size.cast());

    // Load all tiles required for the given view before we request rendering.
    osm.load_tiles(&map_view).await;

    let map = Map::new(
        map_view,
        vec![Box::new(osm), Box::new(layer)],
        None::<DummyMessenger>,
    );

    // We create a renderer without window, so it will use internal texture to render to.
    // Every time the `render` method is callled, the image is updated and can be retrieved
    // by the `get_image` method.
    let renderer = WgpuRenderer::new_with_texture_rt(image_size).await;
    renderer.render(&map).unwrap();

    let bitmap = renderer.get_image().await.unwrap();
    let buffer =
        ImageBuffer::<Rgba<u8>, _>::from_raw(image_size.width(), image_size.height(), bitmap)
            .unwrap();
    buffer.save("output_map.png").unwrap();

    Ok(())
}
