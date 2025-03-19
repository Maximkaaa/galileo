//! This examples shows how to render labels for vector tile points.

use galileo::layer::vector_tile_layer::style::{
    VectorTileDefaultSymbol, VectorTileLabelSymbol, VectorTileStyle,
};
use galileo::layer::vector_tile_layer::{VectorTileLayer, VectorTileLayerBuilder};
use galileo::render::text::text_service::TextService;
use galileo::render::text::{FontWeight, RustybuzzRasterizer, TextStyle};
use galileo::tile_schema::{TileIndex, TileSchema, VerticalDirection};
use galileo::{Color, Lod, MapBuilder};
use galileo_types::cartesian::{Point2, Rect};
use galileo_types::geo::Crs;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    run()
}

pub(crate) fn run() {
    let Some(api_key) = std::option_env!("VT_API_KEY") else {
        panic!("Set the MapTiler API key into VT_API_KEY library when building this example");
    };

    initialize_font_service();

    let graphics_layer = VectorTileLayerBuilder::new_rest(move |&index: &TileIndex| {
        format!(
            "https://api.maptiler.com/tiles/v3-openmaptiles/{z}/{x}/{y}.pbf?key={api_key}",
            z = index.z,
            x = index.x,
            y = index.y
        )
    })
    .with_file_cache_checked(".tile_cache")
    .with_style(default_style())
    .with_tile_schema(tile_schema())
    .with_attribution(
        "© MapTiler© OpenStreetMap contributors".to_string(),
        "https://www.maptiler.com/copyright/".to_string(),
    )
    .build()
    .expect("failed to create layer");

    let labels_style = VectorTileStyle {
        rules: vec![],
        default_symbol: VectorTileDefaultSymbol {
            label: Some(VectorTileLabelSymbol {
                pattern: "{name}".into(),
                text_style: TextStyle {
                    font_family: vec![
                        "Noto Sans".to_string(),
                        "Noto Sans Arabic".to_string(),
                        "Noto Sans Hebrew".to_string(),
                        "Noto Sans SC".to_string(),
                        "Noto Sans KR".to_string(),
                        "Noto Sans JP".to_string(),
                    ],
                    font_size: 12.0,
                    font_color: Color::BLACK,
                    horizontal_alignment: Default::default(),
                    vertical_alignment: Default::default(),
                    weight: FontWeight::BOLD,
                    style: Default::default(),
                    outline_width: 2.0,
                    outline_color: Color::WHITE,
                },
            }),
            ..Default::default()
        },
        background: Default::default(),
    };

    let label_layer = VectorTileLayer::new(
        graphics_layer.provider().clone(),
        labels_style,
        tile_schema(),
        None,
    );

    let map = MapBuilder::default()
        .with_layer(graphics_layer)
        .with_layer(label_layer)
        .build();

    galileo_egui::init(map, []).expect("failed to initialize");
}

fn initialize_font_service() {
    let rasterizer = RustybuzzRasterizer::default();
    TextService::initialize(rasterizer).load_fonts("galileo/examples/data/fonts");
}

fn default_style() -> VectorTileStyle {
    serde_json::from_str(include_str!("data/vt_style.json")).expect("invalid style json")
}

fn tile_schema() -> TileSchema {
    const ORIGIN: Point2 = Point2::new(-20037508.342787, 20037508.342787);
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
