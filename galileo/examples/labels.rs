//! Labels in feature layers

use std::sync::Arc;

use egui::Color32;
use galileo::layer::feature_layer::Feature;
use galileo::layer::raster_tile_layer::RasterTileLayerBuilder;
use galileo::layer::FeatureLayer;
use galileo::render::point_paint::PointPaint;
use galileo::render::render_bundle::RenderBundle;
use galileo::render::text::text_service::TextService;
use galileo::render::text::{
    FontStyle, FontWeight, HorizontalAlignment, RustybuzzRasterizer, TextStyle, VerticalAlignment,
};
use galileo::symbol::Symbol;
use galileo::{Color, Map, MapBuilder};
use galileo_egui::{EguiMap, EguiMapState};
use galileo_types::cartesian::{Point3, Vector2};
use galileo_types::geo::impls::GeoPoint2d;
use galileo_types::geo::Crs;
use galileo_types::geometry::Geom;
use galileo_types::geometry_type::GeoSpace2d;
use galileo_types::latlon;
use parking_lot::RwLock;

struct EguiMapApp {
    map: EguiMapState,
    feature_layer: Arc<RwLock<FeatureLayer<GeoPoint2d, LabeledPoint, LabeledSymbol, GeoSpace2d>>>,
    font_size: f32,
    horizontal_align: HorizontalAlignment,
    vertical_align: VerticalAlignment,
    is_bold: bool,
    is_italic: bool,
    outline_width: f32,
    outline_color: Color32,
    attach_to_map: bool,
}

impl EguiMapApp {
    fn new(mut egui_map_state: EguiMapState) -> Self {
        let layer = FeatureLayer::new(points(), LabeledSymbol::new(), Crs::EPSG3857);
        let layer = Arc::new(RwLock::new(layer));

        egui_map_state.map_mut().layers_mut().push(layer.clone());

        Self {
            map: egui_map_state,
            feature_layer: layer,
            font_size: 20.0,
            horizontal_align: HorizontalAlignment::Center,
            vertical_align: VerticalAlignment::Middle,
            is_bold: false,
            is_italic: false,
            outline_width: 0.0,
            outline_color: Color32::WHITE,
            attach_to_map: false,
        }
    }

    fn update_symbol(&mut self) {
        let weight = match self.is_bold {
            true => FontWeight::BOLD,
            false => FontWeight::NORMAL,
        };
        let style = match self.is_italic {
            true => FontStyle::Italic,
            false => FontStyle::Normal,
        };

        let symbol = LabeledSymbol {
            style: TextStyle {
                font_family: LabeledSymbol::new().style.font_family,
                font_size: self.font_size,
                font_color: Color::BLACK,
                horizontal_alignment: self.horizontal_align,
                vertical_alignment: self.vertical_align,
                weight,
                style,
                outline_width: self.outline_width,
                outline_color: Color::rgba(
                    self.outline_color.r(),
                    self.outline_color.g(),
                    self.outline_color.b(),
                    self.outline_color.a(),
                ),
            },
            attach_to_map: self.attach_to_map,
        };

        self.feature_layer.write().set_symbol(symbol);
    }

    fn horizontal_alignment(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.horizontal_align == HorizontalAlignment::Left, "Left")
                .clicked()
                && self.horizontal_align != HorizontalAlignment::Left
            {
                self.horizontal_align = HorizontalAlignment::Left;
                self.update_symbol();
            }

            if ui
                .selectable_label(
                    self.horizontal_align == HorizontalAlignment::Center,
                    "Center",
                )
                .clicked()
                && self.horizontal_align != HorizontalAlignment::Center
            {
                self.horizontal_align = HorizontalAlignment::Center;
                self.update_symbol();
            }

            if ui
                .selectable_label(self.horizontal_align == HorizontalAlignment::Right, "Right")
                .clicked()
                && self.horizontal_align != HorizontalAlignment::Right
            {
                self.horizontal_align = HorizontalAlignment::Right;
                self.update_symbol();
            }
        });
    }

    fn vertical_alignment(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.vertical_align == VerticalAlignment::Top, "Top")
                .clicked()
                && self.vertical_align != VerticalAlignment::Top
            {
                self.vertical_align = VerticalAlignment::Top;
                self.update_symbol();
            }

            if ui
                .selectable_label(self.vertical_align == VerticalAlignment::Middle, "Middle")
                .clicked()
                && self.vertical_align != VerticalAlignment::Middle
            {
                self.vertical_align = VerticalAlignment::Middle;
                self.update_symbol();
            }

            if ui
                .selectable_label(self.vertical_align == VerticalAlignment::Bottom, "Bottom")
                .clicked()
                && self.vertical_align != VerticalAlignment::Bottom
            {
                self.vertical_align = VerticalAlignment::Bottom;
                self.update_symbol();
            }
        });
    }
}

impl eframe::App for EguiMapApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            EguiMap::new(&mut self.map).show_ui(ui);

            egui::Window::new("Galileo map").show(ctx, |ui| {
                ui.label("Text format:");

                ui.horizontal(|ui| {
                    ui.label("Font size: ");
                    if ui
                        .add(
                            egui::DragValue::new(&mut self.font_size)
                                .speed(0.1)
                                .range(1.0..=50.0),
                        )
                        .changed()
                    {
                        self.update_symbol();
                    }
                });

                self.horizontal_alignment(ui);
                self.vertical_alignment(ui);

                ui.horizontal(|ui| {
                    if ui.selectable_label(self.is_bold, "Bold").clicked() {
                        self.is_bold = !self.is_bold;
                        self.update_symbol();
                    }

                    if ui.selectable_label(self.is_italic, "Italic").clicked() {
                        self.is_italic = !self.is_italic;
                        self.update_symbol();
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Outline");
                    if ui
                        .add(
                            egui::DragValue::new(&mut self.outline_width)
                                .speed(0.01)
                                .range(0.0..=5.0),
                        )
                        .changed()
                    {
                        self.update_symbol();
                    }

                    if ui
                        .color_edit_button_srgba(&mut self.outline_color)
                        .changed()
                    {
                        self.update_symbol();
                    }
                });

                if ui
                    .selectable_label(self.attach_to_map, "Attach to map")
                    .clicked()
                {
                    self.attach_to_map = !self.attach_to_map;
                    self.update_symbol();
                }
            });
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    run()
}

pub(crate) fn run() {
    initialize_font_service();
    let map = create_map();
    galileo_egui::InitBuilder::new(map)
        .with_app_builder(|egui_map_state, _| Box::new(EguiMapApp::new(egui_map_state)))
        .init()
        .expect("failed to initialize");
}

fn initialize_font_service() {
    let rasterizer = RustybuzzRasterizer::default();
    TextService::initialize(rasterizer).load_fonts("galileo/examples/data/fonts");
}

fn create_map() -> Map {
    let layer = RasterTileLayerBuilder::new_osm()
        .with_file_cache_checked(".tile_cache")
        .build()
        .expect("failed to create layer");

    MapBuilder::default().with_layer(layer).build()
}

fn points() -> Vec<LabeledPoint> {
    vec![
        LabeledPoint {
            position: latlon!(0.0, 0.0),
            label: "Behold Galileo - cross-platform map rendering engine",
        },
        LabeledPoint {
            position: latlon!(5.0, 0.0),
            label: "Вот Galileo – кроссплатформенный движок рендеринга карт",
        },
        LabeledPoint {
            position: latlon!(10.0, 0.0),
            label: "보라, Galileo - 크로스 플랫폼 지도 렌더링 엔진",
        },
        LabeledPoint {
            position: latlon!(15.0, 0.0),
            label: "ها هو Galileo - محرك عرض الخرائط عبر الأنظمة الأساسية",
        },
        LabeledPoint {
            position: latlon!(20.0, 0.0),
            label: "הנה Galileo - מנוע רינדור מפות חוצה פלטפורמות",
        },
        LabeledPoint {
            position: latlon!(25.0, 0.0),
            label: "देखो Galileo - क्रॉस-प्लेटफ़ॉर्म मैप रेंडरिंग इंजन",
        },
        LabeledPoint {
            position: latlon!(30.0, 0.0),
            label: "看哪，Galileo——跨平台地图渲染引擎",
        },
        LabeledPoint {
            position: latlon!(35.0, 0.0),
            label: "हेर, Galileo - क्रस-प्लेटफर्म नक्सा रेन्डरिङ इन्जिन।",
        },
    ]
}

struct LabeledPoint {
    position: GeoPoint2d,
    label: &'static str,
}

impl Feature for LabeledPoint {
    type Geom = GeoPoint2d;

    fn geometry(&self) -> &Self::Geom {
        &self.position
    }
}

struct LabeledSymbol {
    style: TextStyle,
    attach_to_map: bool,
}

impl LabeledSymbol {
    fn new() -> Self {
        Self {
            style: TextStyle {
                font_family: vec![
                    "DejaVu Sans".to_string(),
                    "Noto Sans CJK KR".to_string(),
                    "Noto Sans CJK JP".to_string(),
                    "Noto Sans CJK HK".to_string(),
                    "Noto Sans CJK SC".to_string(),
                    "Noto Sans".to_string(),
                ],
                font_size: 20.0,
                font_color: Color::BLACK,
                horizontal_alignment: Default::default(),
                vertical_alignment: Default::default(),
                weight: Default::default(),
                style: Default::default(),
                outline_width: Default::default(),
                outline_color: Default::default(),
            },
            attach_to_map: false,
        }
    }
}

impl Symbol<LabeledPoint> for LabeledSymbol {
    fn render(
        &self,
        feature: &LabeledPoint,
        geometry: &Geom<Point3>,
        min_resolution: f64,
        bundle: &mut RenderBundle,
    ) {
        let Geom::Point(point) = geometry else {
            return;
        };

        bundle.add_point(
            point,
            &PointPaint::circle(Color::BLUE, 10.0),
            min_resolution,
        );
        bundle.add_label(
            point,
            feature.label,
            &self.style,
            Vector2::new(0.0, 0.0),
            self.attach_to_map,
        );
    }
}
