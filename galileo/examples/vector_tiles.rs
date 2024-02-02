use galileo::bounding_box::BoundingBox;
use galileo::control::{EventPropagation, MouseButton, UserEvent};
use galileo::galileo_map::{MapBuilder, VectorTileProvider};
use galileo::layer::vector_tile_layer::style::VectorTileStyle;
use galileo::layer::vector_tile_layer::VectorTileLayer;
use galileo::lod::Lod;
use galileo::tile_scheme::{TileIndex, TileSchema, VerticalDirection};
use galileo_types::cartesian::impls::point::Point2d;
use galileo_types::geo::crs::Crs;
use std::sync::{Arc, RwLock};

#[cfg(not(target_arch = "wasm32"))]
fn get_layer_style() -> Option<VectorTileStyle> {
    const STYLE: &str = "galileo/examples/data/vt_style.json";
    serde_json::from_reader(std::fs::File::open(STYLE).ok()?).ok()
}

thread_local!(
    pub static LAYER: Arc<RwLock<VectorTileLayer<VectorTileProvider>>> =
        Arc::new(RwLock::new(MapBuilder::create_vector_tile_layer(
            |&index: &TileIndex| {
                format!(
                    "https://d1zqyi8v6vm8p9.cloudfront.net/planet/{}/{}/{}.mvt",
                    index.z, index.x, index.y
                )
            },
            tile_scheme(),
            VectorTileStyle::default(),
        )));
);

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    run(MapBuilder::new(), get_layer_style().unwrap()).await;
}

pub async fn run(builder: MapBuilder, style: VectorTileStyle) {
    let layer = LAYER.with(|v| v.clone());
    layer.write().unwrap().update_style(style);

    builder
        .with_layer(layer.clone())
        .with_event_handler(move |ev, map, _| match ev {
            UserEvent::Click(MouseButton::Left, mouse_event) => {
                let view = map.view().clone();
                let position = map
                    .view()
                    .screen_to_map(mouse_event.screen_pointer_position)
                    .unwrap();
                let features = layer.read().unwrap().get_features_at(&position, &view);

                for (layer, feature) in features {
                    println!("{layer}, {:?}", feature.properties);
                }

                EventPropagation::Stop
            }
            _ => EventPropagation::Propagate,
        })
        .build()
        .await
        .run();
}

pub fn tile_scheme() -> TileSchema {
    const ORIGIN: Point2d = Point2d::new(-20037508.342787, 20037508.342787);
    const TOP_RESOLUTION: f64 = 156543.03392800014 / 4.0;

    let mut lods = vec![Lod::new(TOP_RESOLUTION, 0).unwrap()];
    for i in 1..16 {
        lods.push(Lod::new(lods[(i - 1) as usize].resolution() / 2.0, i).unwrap());
    }

    TileSchema {
        origin: ORIGIN,
        bounds: BoundingBox::new(
            -20037508.342787,
            -20037508.342787,
            20037508.342787,
            20037508.342787,
        ),
        lods: lods.into_iter().collect(),
        tile_width: 1024,
        tile_height: 1024,
        y_direction: VerticalDirection::TopToBottom,
        max_tile_scale: 8.0,
        cycle_x: true,
        crs: Crs::EPSG3857,
    }
}
