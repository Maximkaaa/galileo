use galileo::control::{EventPropagation, MouseButton, UserEvent};
use galileo::galileo_map::MapBuilder;
use galileo::layer::feature::{CirclePointSymbol, FeatureLayer, SimplePolygonSymbol, Symbol};
use galileo::primitives::Color;
use galileo::render::{RenderBundle, UnpackedBundle};
use galileo_types::cartesian::impls::point::Point2d;
use galileo_types::geo::crs::Crs;
use galileo_types::geo::impls::point::GeoPoint2d;
use galileo_types::geo::traits::point::NewGeoPoint;
use galileo_types::geometry::Geom;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};

mod data;
use data::{load_countries, Country};

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    run(MapBuilder::new()).await;
}

pub async fn run(builder: MapBuilder) {
    let countries = load_countries();

    let feature_layer = FeatureLayer::new(countries, CountrySymbol {}, Crs::EPSG3857);
    let feature_layer = Arc::new(RwLock::new(feature_layer));

    let point_layer = FeatureLayer::new(
        vec![GeoPoint2d::latlon(55.0, 37.0)],
        CirclePointSymbol {
            color: Color::rgba(255, 0, 0, 255),
            size: 20.0,
        },
        Crs::WGS84,
    );

    let selected_index = Arc::new(AtomicUsize::new(usize::MAX));
    builder
        .with_layer(feature_layer.clone())
        .with_layer(point_layer)
        .with_event_handler(move |ev, map, backend| {
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
                    layer.update_features(&to_update, backend);
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
        SimplePolygonSymbol {
            fill_color,
            stroke_color,
            stroke_width: 1.0,
            stroke_offset: -0.5,
        }
    }
}

impl Symbol<Country, Geom<Point2d>> for CountrySymbol {
    fn render(
        &self,
        feature: &Country,
        geometry: &Geom<Point2d>,
        bundle: &mut Box<dyn RenderBundle>,
    ) -> Vec<usize> {
        let mut ids = vec![];
        let Geom::MultiPolygon(geometry) = geometry else {
            return ids;
        };

        for polygon in geometry.parts() {
            ids.append(
                &mut self
                    .get_polygon_symbol(feature)
                    .render(&(), &polygon, bundle),
            )
        }

        ids
    }

    fn update(
        &self,
        feature: &Country,
        render_ids: &[usize],
        bundle: &mut Box<dyn UnpackedBundle>,
    ) {
        let renders_by_feature = render_ids.len() / feature.geometry.parts().len();
        let mut next_index = 0;
        for _ in feature.geometry.parts() {
            self.get_polygon_symbol(feature).update(
                &(),
                &render_ids[next_index..next_index + renders_by_feature],
                bundle,
            );

            next_index += renders_by_feature;
        }
    }
}
