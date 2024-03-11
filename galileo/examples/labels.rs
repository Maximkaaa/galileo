use galileo::layer::vector_tile_layer::style::VectorTileStyle;
use galileo::render::text::font_service::FontService;
use galileo::tile_scheme::{TileIndex, TileSchema, VerticalDirection};
use galileo::{Lod, MapBuilder};
use galileo_types::cartesian::{Point2d, Rect};
use galileo_types::geo::Crs;
use galileo_types::latlon;

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    run(MapBuilder::new()).await;
}

#[cfg(not(target_arch = "wasm32"))]
fn get_layer_style() -> anyhow::Result<VectorTileStyle> {
    const STYLE: &str = "galileo/examples/data/vt_style.json";
    Ok(serde_json::from_reader(std::fs::File::open(STYLE)?)?)
}

pub async fn run(builder: MapBuilder) {
    builder
        .with_vector_tiles(
            |&index: &TileIndex| {
                format!(
                    "https://d1zqyi8v6vm8p9.cloudfront.net/planet/{}/{}/{}.mvt",
                    index.z, index.x, index.y
                )
            },
            tile_scheme(),
            get_layer_style().expect("failed to load style file"),
        )
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
