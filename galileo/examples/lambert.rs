use data::Country;
use galileo::control::{EventPropagation, UserEvent};
use galileo::galileo_map::MapBuilder;
use galileo::layer::feature_layer::symbol::polygon::SimplePolygonSymbol;
use galileo::layer::feature_layer::symbol::Symbol;
use galileo::layer::feature_layer::FeatureLayer;
use galileo::render::render_bundle::RenderPrimitive;
use galileo::view::MapView;
use galileo::Color;
use galileo_types::cartesian::impls::point::Point2d;
use galileo_types::cartesian::traits::cartesian_point::CartesianPoint3d;
use galileo_types::geo::crs::{Crs, ProjectionType};
use galileo_types::geo::datum::Datum;
use galileo_types::geo::impls::point::GeoPoint2d;
use galileo_types::geo::traits::point::NewGeoPoint;
use galileo_types::geo::traits::projection::{ChainProjection, InvertedProjection, Projection};
use galileo_types::geometry::Geom;
use galileo_types::impls::contour::Contour;
use galileo_types::impls::polygon::Polygon;
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
    bincode::deserialize(include_bytes!("data/countries_simpl.data")).unwrap()
}

pub async fn run(builder: MapBuilder) {
    let countries = load_countries();

    let feature_layer = FeatureLayer::new(countries, CountrySymbol {}, Crs::EPSG3857);
    let feature_layer = Arc::new(RwLock::new(feature_layer));

    let selected_index = Arc::new(AtomicUsize::new(usize::MAX));
    builder
        .with_view(MapView::new_with_crs(
            &GeoPoint2d::latlon(52.0, 10.0),
            10_000.0,
            Crs::new(
                Datum::WGS84,
                ProjectionType::Other("laea lon_0=10 lat_0=52 x_0=4321000 y_0=3210000".into()),
            ),
        ))
        .with_layer(feature_layer.clone())
        .with_event_handler(move |ev, map, _backend| {
            if let UserEvent::PointerMoved(event) = ev {
                let mut layer = feature_layer.write().unwrap();

                let mut new_selected = usize::MAX;
                let Some(position) = map.view().screen_to_map(event.screen_pointer_position) else {
                    return EventPropagation::Stop;
                };

                let projection = ChainProjection::new(
                    Box::new(InvertedProjection::new(
                        map.view().crs().get_projection::<GeoPoint2d, _>().unwrap(),
                    )),
                    layer.crs().get_projection::<_, Point2d>().unwrap(),
                );

                let Some(projected) = projection.project(&position) else {
                    return EventPropagation::Stop;
                };

                if let Some(feature_container) = layer
                    .get_features_at_mut(&projected, map.view().resolution() * 2.0)
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
                    let feature = layer.features_mut().get_mut(selected).unwrap();
                    feature.edit_style().is_selected = false;
                }

                map.redraw();

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
