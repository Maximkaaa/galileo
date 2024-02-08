extern crate galileo;
extern crate tokio;

use android_activity::AndroidApp;
use galileo::{MapBuilder, TileSchema};
use galileo_types::latlon;
use tokio::runtime::Runtime;

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;

    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );

    log::info!("Starting up Galileo");

    Runtime::new()
        .expect("failed to create tokio runtime")
        .block_on(async move {
            let event_loop = winit::event_loop::EventLoopBuilder::new()
                .with_android_app(app)
                .build()
                .expect("failed to create event loop");

            log::info!("Building the map");

            MapBuilder::new()
                .with_event_loop(event_loop)
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
        });
}
