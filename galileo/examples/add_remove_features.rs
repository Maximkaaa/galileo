//! This exmpales shows how to add and remove features from the feature layer.

use std::sync::Arc;

use galileo::control::{EventPropagation, UserEvent, UserEventHandler};
use galileo::layer::raster_tile_layer::RasterTileLayerBuilder;
use galileo::layer::FeatureLayer;
use galileo::symbol::CirclePointSymbol;
use galileo::{Color, Map, MapBuilder};
use galileo_types::cartesian::Point2;
use galileo_types::geo::Crs;
use galileo_types::geometry_type::CartesianSpace2d;
use parking_lot::RwLock;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    run()
}

pub(crate) fn run() {
    let layer = FeatureLayer::new(
        vec![],
        CirclePointSymbol::new(Color::PURPLE, 7.0),
        Crs::EPSG3857,
    );
    let layer = Arc::new(RwLock::new(layer));
    let handler = create_mouse_handler(layer.clone());
    galileo_egui::InitBuilder::new(create_map(layer))
        .with_handlers([Box::new(handler) as Box<dyn UserEventHandler>])
        .init()
        .expect("failed to initialize");
}

fn create_mouse_handler(
    feature_layer: Arc<RwLock<FeatureLayer<Point2, Point2, CirclePointSymbol, CartesianSpace2d>>>,
) -> impl UserEventHandler {
    move |ev: &UserEvent, map: &mut Map| {
        if let UserEvent::Click(_, event) = ev {
            let mut layer = feature_layer.write();

            let Some(position) = map.view().screen_to_map(event.screen_pointer_position) else {
                return EventPropagation::Stop;
            };

            let clicked: Vec<_> = layer
                .get_features_at(&position, map.view().resolution() * 7.0)
                .map(|(id, _)| id)
                .collect();
            if clicked.is_empty() {
                println!("Adding point: {position:?}");
                let id = layer.features_mut().add(position);
                layer.update_feature(id);
            } else {
                for id in clicked {
                    layer.features_mut().remove(id);
                    layer.update_feature(id);
                }
            }

            map.redraw();

            return EventPropagation::Stop;
        }

        EventPropagation::Propagate
    }
}

fn create_map(
    feature_layer: Arc<RwLock<FeatureLayer<Point2, Point2, CirclePointSymbol, CartesianSpace2d>>>,
) -> Map {
    let raster_layer = RasterTileLayerBuilder::new_osm()
        .with_file_cache_checked(".tile_cache")
        .build()
        .expect("failed to create layer");

    MapBuilder::default()
        .with_latlon(37.566, 128.9784)
        .with_z_level(8)
        .with_layer(raster_layer)
        .with_layer(feature_layer)
        .build()
}
