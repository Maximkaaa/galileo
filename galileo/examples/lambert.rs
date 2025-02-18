//! This examples demonstrates working with the map in a projection different from usual Web
//! Mercator projection.

use std::sync::Arc;

use data::Country;
use galileo::control::{EventPropagation, UserEvent, UserEventHandler};
use galileo::layer::feature_layer::symbol::{SimplePolygonSymbol, Symbol};
use galileo::layer::feature_layer::{FeatureLayer, FeatureLayerOptions};
use galileo::layer::Layer;
use galileo::render::render_bundle::RenderBundle;
use galileo::{Map, MapBuilder};
use galileo_types::cartesian::{Point2d, Point3d};
use galileo_types::geo::impls::GeoPoint2d;
use galileo_types::geo::{
    ChainProjection, Crs, Datum, InvertedProjection, Projection, ProjectionType,
};
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

fn load_countries() -> Vec<Country> {
    bincode::serde::decode_from_slice(
        include_bytes!("data/countries_simpl.data"),
        bincode::config::legacy(),
    )
    .expect("invalid countries data")
    .0
}

fn create_mouse_handler(
    feature_layer: Arc<RwLock<FeatureLayer<Point2d, Country, CountrySymbol, CartesianSpace2d>>>,
) -> impl UserEventHandler {
    let selected_id = Mutex::new(None);

    move |ev: &UserEvent, map: &mut Map| {
        if let UserEvent::PointerMoved(event) = ev {
            let mut layer = feature_layer.write();

            let Some(position) = map.view().screen_to_map(event.screen_pointer_position) else {
                return EventPropagation::Stop;
            };

            let projection = ChainProjection::new(
                Box::new(InvertedProjection::new(
                    map.view()
                        .crs()
                        .get_projection::<GeoPoint2d, _>()
                        .expect("cannot project point"),
                )),
                layer
                    .crs()
                    .get_projection::<_, Point2d>()
                    .expect("cannot find projection"),
            );

            let Some(projected) = projection.project(&position) else {
                return EventPropagation::Stop;
            };

            let new_selected = if let Some((id, feature)) = layer
                .get_features_at_mut(&projected, map.view().resolution() * 2.0)
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
        buffer_size_limit: 1_000_000,
        ..Default::default()
    })
}

fn create_map(feature_layer: impl Layer + 'static) -> Map {
    MapBuilder::default()
        .with_latlon(52.0, 10.0)
        .with_resolution(10_000.0)
        .with_crs(Crs::new(
            Datum::WGS84,
            ProjectionType::Other("laea lon_0=10 lat_0=52 x_0=4321000 y_0=3210000".into()),
        ))
        .with_layer(feature_layer)
        .build()
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
        self.get_polygon_symbol(feature)
            .render(&(), geometry, min_resolution, bundle)
    }
}
