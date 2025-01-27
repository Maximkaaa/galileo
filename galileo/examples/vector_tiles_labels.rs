//! This examples shows how to render labels for vector tile points.

use bytes::Bytes;
use galileo::layer::vector_tile_layer::style::{
    VectorTileDefaultSymbol, VectorTileLabelSymbol, VectorTileStyle,
};
#[cfg(target_arch = "wasm32")]
use galileo::layer::vector_tile_layer::tile_provider::WebWorkerVectorTileProvider;
use galileo::layer::vector_tile_layer::VectorTileLayer;
use galileo::render::text::font_service::FontService;
use galileo::render::text::TextStyle;
use galileo::tile_scheme::{TileIndex, TileSchema, VerticalDirection};
use galileo::{Color, Lod, MapBuilder};
use galileo_types::cartesian::{Point2d, Rect};
use galileo_types::geo::Crs;

#[cfg(target_arch = "wasm32")]
type VectorTileProvider = WebWorkerVectorTileProvider;

#[cfg(not(target_arch = "wasm32"))]
fn get_layer_style() -> Option<VectorTileStyle> {
    const STYLE: &str = "galileo/examples/data/vt_style.json";
    Some(serde_json::from_reader(std::fs::File::open(STYLE).ok()?).expect("failed to parse style"))
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    let Some(api_key) = std::env::var_os("VT_API_KEY") else {
        eprintln!("You must set VT_API_KEY environment variable with a valid MapTiler API key to run this example");
        eprintln!("You can obtain your free API key at https://maptiler.com");

        return;
    };

    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("warn,galileo=trace"),
    )
    .init();
    run(
        MapBuilder::new(),
        get_layer_style().expect("failed to load style"),
        api_key.into_string().expect("invalid VT API key"),
    )
    .await;
}

pub(crate) async fn run(builder: MapBuilder, style: VectorTileStyle, api_key: String) {
    FontService::with_mut(|service| {
        let font = include_bytes!("data/NotoSansAdlam-Regular.ttf");
        service
            .load_fonts(Bytes::from_static(font))
            .expect("failed to load font");
    });

    // You can get your fee API key at https://maptiler.com
    let tile_provider = MapBuilder::create_vector_tile_provider(
        move |&index: &TileIndex| {
            format!(
                "https://api.maptiler.com/tiles/v3-openmaptiles/{z}/{x}/{y}.pbf?key={api_key}",
                z = index.z,
                x = index.x,
                y = index.y
            )
        },
        tile_schema(),
    );

    let graphics_layer = VectorTileLayer::new(tile_provider.clone(), style, tile_schema());

    let style = VectorTileStyle {
        rules: vec![],
        default_symbol: VectorTileDefaultSymbol {
            label: Some(VectorTileLabelSymbol {
                pattern: "{name_en}".into(),
                text_style: TextStyle {
                    font_name: "Noto Sans".to_string(),
                    font_size: 12.0,
                    font_color: Color::BLACK,
                    horizontal_alignment: Default::default(),
                    vertical_alignment: Default::default(),
                },
            }),
            ..Default::default()
        },
        background: Default::default(),
    };
    let label_layer = VectorTileLayer::new(tile_provider, style, tile_schema());

    builder
        .with_layer(graphics_layer)
        .with_layer(label_layer)
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
