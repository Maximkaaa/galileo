//! This example shows how to create custom symbols for feature layers and set the appearance of
//! features based on their attributes.

use std::sync::Arc;

use data::{City, Country};
use galileo::control::{EventPropagation, MouseButton, UserEvent, UserEventHandler};
use galileo::layer::feature_layer::symbol::{SimplePolygonSymbol, Symbol};
use galileo::layer::feature_layer::{FeatureLayer, FeatureLayerOptions};
use galileo::layer::Layer;
use galileo::render::point_paint::PointPaint;
use galileo::render::render_bundle::RenderBundle;
use galileo::{Color, Map, MapBuilder};
use galileo_types::cartesian::{Point2d, Point3d};
use galileo_types::geo::Crs;
use galileo_types::geometry::Geom;
use galileo_types::geometry_type::CartesianSpace2d;
use parking_lot::{Mutex, RwLock};

mod data;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    run()
}

pub(crate) fn run() {
    let countries_layer = Arc::new(RwLock::new(create_countries_layer()));
    let map = create_map(countries_layer.clone());
    let handler = create_mouse_handler(countries_layer);

    galileo_egui::init(map, [Box::new(handler) as Box<dyn UserEventHandler>])
        .expect("failed to initialize");
}

fn create_map(countries_layer: impl Layer + 'static) -> Map {
    let point_layer = FeatureLayer::new(load_cities(), CitySymbol {}, Crs::WGS84);

    MapBuilder::default()
        .with_layer(countries_layer)
        .with_layer(point_layer)
        .build()
}

fn load_countries() -> Vec<Country> {
    bincode::serde::decode_from_slice(
        include_bytes!("data/countries.data"),
        bincode::config::legacy(),
    )
    .expect("invalid countries data")
    .0
}

fn load_cities() -> Vec<City> {
    let mut reader = csv::Reader::from_reader(&include_bytes!("data/worldcities.csv")[..]);
    let mut cities: Vec<City> = reader.deserialize().filter_map(|res| res.ok()).collect();
    cities.sort_by(|a, b| {
        if a.capital == b.capital {
            return std::cmp::Ordering::Equal;
        };

        match &a.capital[..] {
            "primary" => std::cmp::Ordering::Greater,
            "admin" if &b.capital != "primary" => std::cmp::Ordering::Greater,
            "minor" if &b.capital != "primary" && &b.capital != "admin" => {
                std::cmp::Ordering::Greater
            }
            _ => std::cmp::Ordering::Less,
        }
    });
    cities
}

fn create_mouse_handler(
    feature_layer: Arc<RwLock<FeatureLayer<Point2d, Country, CountrySymbol, CartesianSpace2d>>>,
) -> impl UserEventHandler {
    let selected_id = Mutex::new(None);
    move |ev: &UserEvent, map: &mut Map| {
        if let UserEvent::Click(button, event) = ev {
            if *button == MouseButton::Left {
                let mut layer = feature_layer.write();

                let Some(position) = map.view().screen_to_map(event.screen_pointer_position) else {
                    return EventPropagation::Stop;
                };

                let mut to_update = vec![];
                for (id, feature) in
                    layer.get_features_at_mut(&position, map.view().resolution() * 2.0)
                {
                    log::info!("Found {} with bbox {:?}", feature.name, feature.bbox);
                    feature.is_hidden = !feature.is_hidden;
                    to_update.push(id);
                }

                for id in to_update {
                    layer.update_feature(id);
                }

                map.redraw();

                return EventPropagation::Stop;
            }
        }

        if let UserEvent::PointerMoved(event) = ev {
            let mut layer = feature_layer.write();

            let Some(position) = map.view().screen_to_map(event.screen_pointer_position) else {
                return EventPropagation::Stop;
            };

            let new_selected = if let Some((id, feature)) = layer
                .get_features_at_mut(&position, map.view().resolution() * 2.0)
                .next()
            {
                if feature.is_selected {
                    return EventPropagation::Stop;
                }

                feature.is_selected = true;
                Some(id)
            } else {
                None
            };

            match new_selected {
                None => {}
                Some(id) => layer.update_feature(id),
            }

            if let Some(old_selected) = std::mem::replace(&mut *selected_id.lock(), new_selected) {
                layer
                    .features_mut()
                    .get_mut(old_selected)
                    .map(|f| !f.is_selected);
                layer.update_feature(old_selected)
            }

            map.redraw();

            return EventPropagation::Stop;
        }

        EventPropagation::Propagate
    }
}

fn create_countries_layer() -> FeatureLayer<Point2d, Country, CountrySymbol, CartesianSpace2d> {
    let countries = load_countries();

    FeatureLayer::with_lods(
        countries,
        CountrySymbol {},
        Crs::EPSG3857,
        &[8000.0, 1000.0, 1.0],
    )
    .with_options(FeatureLayerOptions {
        sort_by_depth: false,
        buffer_size_limit: 1_000_000,
        use_antialiasing: true,
    })
}

struct CountrySymbol {}

impl CountrySymbol {
    fn get_polygon_symbol(&self, feature: &Country) -> SimplePolygonSymbol {
        let stroke_color = feature.color;
        let fill_color = stroke_color.with_alpha(if feature.is_selected() { 255 } else { 150 });
        SimplePolygonSymbol::new(fill_color)
            .with_stroke_color(stroke_color)
            .with_stroke_width(2.0)
            .with_stroke_offset(-1.0)
    }
}

impl Symbol<Country> for CountrySymbol {
    fn render(
        &self,
        feature: &Country,
        geometry: &Geom<Point3d>,
        min_resolution: f64,
        bundle: &mut RenderBundle,
    ) {
        if !feature.is_hidden {
            self.get_polygon_symbol(feature)
                .render(&(), geometry, min_resolution, bundle)
        }
    }
}

struct CitySymbol {}

impl Symbol<City> for CitySymbol {
    fn render(
        &self,
        feature: &City,
        geometry: &Geom<Point3d>,
        min_resolution: f64,
        bundle: &mut RenderBundle,
    ) {
        let size = (feature.population / 1000.0).log2() as f32;
        let Geom::Point(point) = geometry else {
            return;
        };

        match &feature.capital[..] {
            "primary" => {
                bundle.add_point(
                    point,
                    &PointPaint::circle(Color::BLACK, size * 2.0 + 4.0),
                    min_resolution,
                );
                bundle.add_point(
                    point,
                    &PointPaint::sector(
                        Color::from_hex("#ff8000"),
                        size * 2.0,
                        -5f32.to_radians(),
                        135f32.to_radians(),
                    ),
                    min_resolution,
                );
                bundle.add_point(
                    point,
                    &PointPaint::sector(
                        Color::from_hex("#ffff00"),
                        size * 2.0,
                        130f32.to_radians(),
                        270f32.to_radians(),
                    ),
                    min_resolution,
                );
                bundle.add_point(
                    point,
                    &PointPaint::sector(
                        Color::from_hex("#00ffff"),
                        size * 2.0,
                        265f32.to_radians(),
                        360f32.to_radians(),
                    ),
                    min_resolution,
                )
            }
            "admin" => bundle.add_point(
                point,
                &PointPaint::circle(Color::from_hex("#f5009b"), size),
                min_resolution,
            ),
            "minor" => bundle.add_point(
                point,
                &PointPaint::square(Color::from_hex("#0a85ed"), size)
                    .with_outline(Color::from_hex("#0d4101"), 2.0),
                min_resolution,
            ),
            _ => bundle.add_point(
                point,
                &PointPaint::circle(Color::from_hex("#4e00de"), size),
                min_resolution,
            ),
        };
    }
}
