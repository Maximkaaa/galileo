use bytes::Bytes;
use galileo::layer::vector_tile_layer::style::{VectorTileStyle, VectorTileSymbol};
#[cfg(target_arch = "wasm32")]
use galileo::layer::vector_tile_layer::tile_provider::WebWorkerVectorTileProvider;
use galileo::layer::vector_tile_layer::VectorTileLayer;
#[cfg(not(target_arch = "wasm32"))]
use galileo::layer::{
    data_provider::{FileCacheController, UrlDataProvider},
    vector_tile_layer::tile_provider::{ThreadedProvider, VtProcessor},
};
use galileo::render::point_paint::PointPaint;
use galileo::render::text::font_service::FontService;
use galileo::render::text::TextStyle;
use galileo::tile_scheme::{TileIndex, TileSchema, VerticalDirection};
use galileo::{Color, Lod, MapBuilder};
use galileo_types::cartesian::Point2d;
use galileo_types::cartesian::Rect;
use galileo_types::geo::Crs;

#[cfg(not(target_arch = "wasm32"))]
type VectorTileProvider =
    ThreadedProvider<UrlDataProvider<TileIndex, VtProcessor, FileCacheController>>;

#[cfg(target_arch = "wasm32")]
type VectorTileProvider = WebWorkerVectorTileProvider;

#[cfg(not(target_arch = "wasm32"))]
fn get_layer_style() -> Option<VectorTileStyle> {
    const STYLE: &str = "galileo/examples/data/vt_style.json";
    serde_json::from_reader(std::fs::File::open(STYLE).ok()?).ok()
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("warn,galileo=trace"),
    )
    .init();
    run(MapBuilder::new(), get_layer_style().unwrap()).await;
}

pub async fn run(builder: MapBuilder, style: VectorTileStyle) {
    FontService::with_mut(|service| {
        let font = include_bytes!("data/NotoSansAdlam-Regular.ttf");
        service
            .load_fonts(Bytes::from_static(font))
            .expect("failed to load font");
    });

    let tile_provider = MapBuilder::create_vector_tile_provider(
        |&index: &TileIndex| {
            format!(
                "https://d1zqyi8v6vm8p9.cloudfront.net/planet/{}/{}/{}.mvt",
                index.z, index.x, index.y
            )
        },
        tile_schema(),
    );

    let graphics_layer =
        VectorTileLayer::from_url(tile_provider.clone(), style, tile_schema()).await;

    let style = VectorTileStyle {
        rules: vec![],
        default_symbol: VectorTileSymbol {
            point: Some(PointPaint::label_owed(
                "{name_en}".into(),
                TextStyle {
                    font_name: "Noto Sans".to_string(),
                    font_size: 12.0,
                    font_color: Color::BLACK,
                    horizontal_alignment: Default::default(),
                    vertical_alignment: Default::default(),
                },
            )),
            line: None,
            polygon: None,
        },
        background: Default::default(),
    };
    let label_layer = VectorTileLayer::from_url(tile_provider, style, tile_schema()).await;

    builder
        .with_layer(graphics_layer)
        .with_layer(label_layer)
        .build()
        .await
        .run();
}

pub fn tile_schema() -> TileSchema {
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
