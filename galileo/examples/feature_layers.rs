use data::{City, Country};
use galileo::control::{EventPropagation, MouseButton, UserEvent};
use galileo::galileo_map::MapBuilder;
use galileo::layer::feature_layer::symbol::polygon::SimplePolygonSymbol;
use galileo::layer::feature_layer::symbol::Symbol;
use galileo::layer::feature_layer::FeatureLayer;
use galileo::render::point_paint::PointPaint;
use galileo::render::render_bundle::RenderPrimitive;
use galileo::Color;
use galileo_types::cartesian::impls::contour::Contour;
use galileo_types::cartesian::impls::polygon::Polygon;
use galileo_types::cartesian::traits::cartesian_point::CartesianPoint3d;
use galileo_types::geo::crs::Crs;
use galileo_types::geometry::Geom;
use num_traits::AsPrimitive;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};

mod data;

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    run(MapBuilder::new()).await;
}

pub fn load_countries() -> Vec<Country> {
    bincode::deserialize(include_bytes!("data/countries.data")).unwrap()
}

pub fn load_cities() -> Vec<City> {
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

pub async fn run(builder: MapBuilder) {
    let countries = load_countries();

    let feature_layer = FeatureLayer::with_lods(
        countries,
        CountrySymbol {},
        Crs::EPSG3857,
        &vec![8000.0, 1000.0, 1.0],
    );
    let feature_layer = Arc::new(RwLock::new(feature_layer));

    let point_layer = FeatureLayer::new(load_cities(), CitySymbol {}, Crs::WGS84);

    let selected_index = Arc::new(AtomicUsize::new(usize::MAX));
    builder
        .with_layer(feature_layer.clone())
        .with_layer(point_layer)
        .with_event_handler(move |ev, map, _backend| {
            if let UserEvent::Click(button, event) = ev {
                if *button == MouseButton::Left {
                    let layer = feature_layer.write().unwrap();

                    let Some(position) = map.view().screen_to_map(event.screen_pointer_position)
                    else {
                        return EventPropagation::Stop;
                    };

                    for (_idx, feature) in
                        layer.get_features_at(&position, map.view().resolution() * 2.0)
                    {
                        log::info!("Found {} with bbox {:?}", feature.name, feature.bbox);
                    }

                    return EventPropagation::Stop;
                }
            }

            if let UserEvent::PointerMoved(event) = ev {
                let mut layer = feature_layer.write().unwrap();

                let mut to_update = vec![];

                let mut new_selected = usize::MAX;
                let Some(position) = map.view().screen_to_map(event.screen_pointer_position) else {
                    return EventPropagation::Stop;
                };
                if let Some((index, feature)) = layer
                    .get_features_at_mut(&position, map.view().resolution() * 2.0)
                    .first_mut()
                {
                    if *index == selected_index.load(Ordering::Relaxed) {
                        return EventPropagation::Stop;
                    }
                    feature.is_selected = true;
                    new_selected = *index;
                    to_update.push(*index);
                }

                let selected = selected_index.swap(new_selected, Ordering::Relaxed);
                if selected != usize::MAX {
                    let feature = layer.features_mut().skip(selected).next().unwrap();
                    feature.is_selected = false;
                    to_update.push(selected);
                }

                if !to_update.is_empty() {
                    layer.update_features(&to_update, map.view(), true);
                }

                return EventPropagation::Stop;
            }

            EventPropagation::Propagate
        })
        .build()
        .await
        .run();
}

struct CountrySymbol {}

impl CountrySymbol {
    fn get_polygon_symbol(&self, feature: &Country) -> SimplePolygonSymbol {
        let stroke_color = feature.color;
        let fill_color = Color {
            a: if feature.is_selected() { 255 } else { 150 },
            ..stroke_color
        };
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

        let _ = match &feature.capital[..] {
            "primary" => {
                primitives.push(RenderPrimitive::new_point_ref(
                    point,
                    PointPaint::circle(Color::BLACK, size * 2.0 + 4.0),
                ));
                primitives.push(RenderPrimitive::new_point_ref(
                    point,
                    PointPaint::sector(
                        Color::from_hex("#ff8000"),
                        size * 2.0,
                        -5f32.to_radians(),
                        135f32.to_radians(),
                    ),
                ));
                primitives.push(RenderPrimitive::new_point_ref(
                    point,
                    PointPaint::sector(
                        Color::from_hex("#ffff00"),
                        size * 2.0,
                        130f32.to_radians(),
                        270f32.to_radians(),
                    ),
                ));
                primitives.push(RenderPrimitive::new_point_ref(
                    point,
                    PointPaint::sector(
                        Color::from_hex("#00ffff"),
                        size * 2.0,
                        265f32.to_radians(),
                        360f32.to_radians(),
                    ),
                ))
            }
            "admin" => primitives.push(RenderPrimitive::new_point_ref(
                point,
                PointPaint::circle(Color::from_hex("#f5009b"), size),
            )),
            "minor" => primitives.push(RenderPrimitive::new_point_ref(
                point,
                PointPaint::square(Color::from_hex("#0a85ed"), size)
                    .with_outline(Color::from_hex("#0d4101"), 2.0),
            )),
            _ => primitives.push(RenderPrimitive::new_point_ref(
                point,
                PointPaint::circle(Color::from_hex("#4e00de"), size),
            )),
        };

        primitives
    }
}
