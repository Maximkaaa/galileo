//! This exmpale shows how to create and work with vector tile layers.

use std::sync::Arc;

use galileo::control::{EventPropagation, MouseButton, UserEvent};
use galileo::layer::vector_tile_layer::style::VectorTileStyle;
use galileo::tile_scheme::{TileIndex, TileSchema, VerticalDirection};
use galileo::{Lod, MapBuilder};
use galileo_types::cartesian::{Point2d, Rect};
use galileo_types::geo::Crs;
use parking_lot::RwLock;

#[cfg(not(target_arch = "wasm32"))]
fn get_layer_style() -> Option<VectorTileStyle> {
    const STYLE: &str = "galileo/examples/data/vt_style.json";
    Some(
        serde_json::from_reader(std::fs::File::open(STYLE).expect("invalid style json"))
            .expect("invalid style json"),
    )
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    let Some(api_key) = std::env::var_os("VT_API_KEY") else {
        eprintln!("You must set VT_API_KEY environment variable with a valid MapTiler API key to run this example");
        eprintln!("You can obtain your free API key at https://maptiler.com");

        return;
    };

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("galileo=trace"))
        .init();
    run(
        MapBuilder::new(),
        get_layer_style().expect("failed to load style"),
        api_key.into_string().expect("invalid VT API key"),
    )
    .await;
}

pub(crate) async fn run(builder: MapBuilder, style: VectorTileStyle, api_key: String) {
    let layer = Arc::new(RwLock::new(MapBuilder::create_vector_tile_layer(
        move |&index: &TileIndex| {
            format!(
                "https://api.maptiler.com/tiles/v3-openmaptiles/{z}/{x}/{y}.pbf?key={api_key}",
                z = index.z,
                x = index.x,
                y = index.y
            )
        },
        tile_schema(),
        style,
    )));

    builder
        .with_layer(layer.clone())
        .with_event_handler(move |ev, map| match ev {
            UserEvent::Click(MouseButton::Left, mouse_event) => {
                let view = map.view().clone();
                if let Some(position) = map
                    .view()
                    .screen_to_map(mouse_event.screen_pointer_position)
                {
                    let features = layer.read().get_features_at(&position, &view);

                    for (layer, feature) in features {
                        println!("{layer}, {:?}", feature.properties);
                    }
                }

                EventPropagation::Stop
            }
            _ => EventPropagation::Propagate,
        })
        .build()
        .await
        .run();
}

fn tile_schema() -> TileSchema {
    const ORIGIN: Point2d = Point2d::new(-20037508.342787, 20037508.342787);
    const TOP_RESOLUTION: f64 = 156543.03392800014 / 4.0;

    let mut lods = vec![Lod::new(TOP_RESOLUTION, 0).expect("invalid config")];
    for i in 1..16 {
        lods.push(
            Lod::new(lods[(i - 1) as usize].resolution() / 2.0, i).expect("invalid tile schema"),
        );
    }

    TileSchema {
        origin: ORIGIN,
        bounds: Rect::new(
            -20037508.342787,
            -20037508.342787,
            20037508.342787,
            20037508.342787,
        ),
        lods: lods.into_iter().collect(),
        tile_width: 1024,
        tile_height: 1024,
        y_direction: VerticalDirection::TopToBottom,
        crs: Crs::EPSG3857,
    }
}
