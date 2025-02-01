//! This example shows how to create custom symbols for feature layers and set the appearance of
//! features based on their attributes.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use data::{City, Country};
use galileo::control::{EventPropagation, MouseButton, UserEvent, UserEventHandler};
use galileo::layer::feature_layer::symbol::{SimplePolygonSymbol, Symbol};
use galileo::layer::feature_layer::FeatureLayer;
use galileo::layer::Layer;
use galileo::render::point_paint::PointPaint;
use galileo::render::render_bundle::RenderPrimitive;
use galileo::{Color, Map, MapView};
use galileo_types::cartesian::{CartesianPoint3d, Point2d};
use galileo_types::geo::Crs;
use galileo_types::geometry::Geom;
use galileo_types::geometry_type::CartesianSpace2d;
use galileo_types::impls::{Contour, Polygon};
use galileo_types::latlon;
use num_traits::AsPrimitive;
use parking_lot::RwLock;

mod data;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    run()
}

pub(crate) fn run() {
    let countries_layer = Arc::new(RwLock::new(create_countries_layer()));
    let map = create_map(countries_layer.clone());
    let selected_index = AtomicUsize::new(usize::MAX);
    let handler = create_mouse_handler(countries_layer, selected_index);

    galileo_egui::init(map, [Box::new(handler) as Box<dyn UserEventHandler>])
        .expect("failed to initialize");
}

fn create_map(countries_layer: impl Layer + 'static) -> Map {
    let point_layer = FeatureLayer::new(load_cities(), CitySymbol {}, Crs::WGS84);

    Map::new(
        MapView::new(&latlon!(0.0, 0.0), 9783.939620500008),
        vec![Box::new(countries_layer), Box::new(point_layer)],
        None,
    )
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
    selected_index: AtomicUsize,
) -> impl UserEventHandler {
    move |ev: &UserEvent, map: &mut Map| {
        if let UserEvent::Click(button, event) = ev {
            if *button == MouseButton::Left {
                let mut layer = feature_layer.write();

                let Some(position) = map.view().screen_to_map(event.screen_pointer_position) else {
                    return EventPropagation::Stop;
                };

                for mut feature_container in
                    layer.get_features_at_mut(&position, map.view().resolution() * 2.0)
                {
                    log::info!(
                        "Found {} with bbox {:?}",
                        feature_container.as_ref().name,
                        feature_container.as_ref().bbox
                    );

                    if feature_container.is_hidden() {
                        feature_container.show();
                    } else {
                        feature_container.hide();
                    }
                }

                map.redraw();

                return EventPropagation::Stop;
            }
        }

        if let UserEvent::PointerMoved(event) = ev {
            let mut layer = feature_layer.write();

            let mut new_selected = usize::MAX;
            let Some(position) = map.view().screen_to_map(event.screen_pointer_position) else {
                return EventPropagation::Stop;
            };
            if let Some(feature_container) = layer
                .get_features_at_mut(&position, map.view().resolution() * 2.0)
                .next()
            {
                let index = feature_container.index();
                if index == selected_index.load(Ordering::Relaxed) {
                    return EventPropagation::Stop;
                }

                feature_container.edit_style().is_selected = true;
                new_selected = index;
            }

            let selected = selected_index.swap(new_selected, Ordering::Relaxed);
            if selected != usize::MAX {
                if let Some(feature) = layer.features_mut().get_mut(selected) {
                    feature.edit_style().is_selected = false;
                }
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
    fn render<'a, N, P>(
        &self,
        feature: &Country,
        geometry: &'a Geom<P>,
        min_resolution: f64,
    ) -> Vec<RenderPrimitive<'a, N, P, Contour<P>, Polygon<P>>>
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N> + Clone,
    {
        self.get_polygon_symbol(feature)
            .render(&(), geometry, min_resolution)
    }
}

struct CitySymbol {}

impl Symbol<City> for CitySymbol {
    fn render<'a, N, P>(
        &self,
        feature: &City,
        geometry: &'a Geom<P>,
        _min_resolution: f64,
    ) -> Vec<RenderPrimitive<'a, N, P, Contour<P>, Polygon<P>>>
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N> + Clone,
    {
        let size = (feature.population / 1000.0).log2() as f32;
        let mut primitives = vec![];
        let Geom::Point(point) = geometry else {
            return primitives;
        };

        match &feature.capital[..] {
            "primary" => {
                primitives.push(RenderPrimitive::new_point(
                    point.clone(),
                    PointPaint::circle(Color::BLACK, size * 2.0 + 4.0),
                ));
                primitives.push(RenderPrimitive::new_point(
                    point.clone(),
                    PointPaint::sector(
                        Color::from_hex("#ff8000"),
                        size * 2.0,
                        -5f32.to_radians(),
                        135f32.to_radians(),
                    ),
                ));
                primitives.push(RenderPrimitive::new_point(
                    point.clone(),
                    PointPaint::sector(
                        Color::from_hex("#ffff00"),
                        size * 2.0,
                        130f32.to_radians(),
                        270f32.to_radians(),
                    ),
                ));
                primitives.push(RenderPrimitive::new_point(
                    point.clone(),
                    PointPaint::sector(
                        Color::from_hex("#00ffff"),
                        size * 2.0,
                        265f32.to_radians(),
                        360f32.to_radians(),
                    ),
                ))
            }
            "admin" => primitives.push(RenderPrimitive::new_point(
                point.clone(),
                PointPaint::circle(Color::from_hex("#f5009b"), size),
            )),
            "minor" => primitives.push(RenderPrimitive::new_point(
                point.clone(),
                PointPaint::square(Color::from_hex("#0a85ed"), size)
                    .with_outline(Color::from_hex("#0d4101"), 2.0),
            )),
            _ => primitives.push(RenderPrimitive::new_point(
                point.clone(),
                PointPaint::circle(Color::from_hex("#4e00de"), size),
            )),
        };

        primitives
    }
}
