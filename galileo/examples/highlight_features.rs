//! This example shows how to change a feature appearance based on use input - pointing with a
//! mouse in this case.

use std::sync::Arc;

use galileo::control::{EventPropagation, UserEvent, UserEventHandler};
use galileo::decoded_image::DecodedImage;
use galileo::layer::feature_layer::symbol::Symbol;
use galileo::layer::feature_layer::{Feature, FeatureLayer};
use galileo::layer::raster_tile_layer::RasterTileLayerBuilder;
use galileo::render::point_paint::MarkerStyle;
use galileo::render::render_bundle::RenderBundle;
use galileo::{Map, MapBuilder};
use galileo_types::cartesian::{Point2, Point3, Vector2};
use galileo_types::geo::{Crs, Projection};
use galileo_types::geometry::Geom;
use galileo_types::geometry_type::CartesianSpace2d;
use galileo_types::{latlon, CartesianGeometry2d, Geometry};
use parking_lot::RwLock;

const YELLOW_PIN: &[u8] = include_bytes!("data/pin-yellow.png");
const GREEN_PIN: &[u8] = include_bytes!("data/pin-green.png");

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    run()
}

pub(crate) fn run() {
    let projection = Crs::EPSG3857
        .get_projection()
        .expect("must find projection");

    let feature_layer = FeatureLayer::new(
        [
            latlon!(53.732562, -1.863383),
            latlon!(53.728265, -1.839966),
            latlon!(53.704014, -1.786128),
        ]
        .iter()
        .map(|point| PointMarker {
            point: projection
                .project(point)
                .expect("point cannot be projected"),
            ..Default::default()
        })
        .collect(),
        ColoredPointSymbol {
            default_image: Arc::new(
                DecodedImage::decode(YELLOW_PIN).expect("Must have Yellow Pin Image"),
            ),
            highlighted_image: Arc::new(
                DecodedImage::decode(GREEN_PIN).expect("Must have Green Pin Image"),
            ),
        },
        Crs::EPSG3857,
    );

    let feature_layer = Arc::new(RwLock::new(feature_layer));
    let handler = create_mouse_handler(feature_layer.clone());

    let mut map = create_map();
    map.layers_mut().push(feature_layer);

    galileo_egui::init(map, [Box::new(handler) as Box<dyn UserEventHandler>])
        .expect("failed to initialize");
}

#[derive(Debug, PartialEq, Default)]
pub(crate) struct PointMarker {
    pub(crate) point: Point2,
    pub(crate) highlighted: bool,
}

impl Feature for PointMarker {
    type Geom = Self;

    fn geometry(&self) -> &Self::Geom {
        self
    }
}

impl Geometry for PointMarker {
    type Point = Point2;

    fn project<P: Projection<InPoint = Self::Point> + ?Sized>(
        &self,
        projection: &P,
    ) -> Option<Geom<P::OutPoint>> {
        self.point.project(projection)
    }
}

impl CartesianGeometry2d<Point2> for PointMarker {
    fn is_point_inside<
        Other: galileo_types::cartesian::CartesianPoint2d<
            Num = <Point2 as galileo_types::cartesian::CartesianPoint2d>::Num,
        >,
    >(
        &self,
        point: &Other,
        tolerance: <Point2 as galileo_types::cartesian::CartesianPoint2d>::Num,
    ) -> bool {
        self.point.is_point_inside(point, tolerance)
    }

    fn bounding_rectangle(
        &self,
    ) -> Option<
        galileo_types::cartesian::Rect<<Point2 as galileo_types::cartesian::CartesianPoint2d>::Num>,
    > {
        None
    }
}

fn create_mouse_handler(
    feature_layer: Arc<
        RwLock<FeatureLayer<Point2, PointMarker, ColoredPointSymbol, CartesianSpace2d>>,
    >,
) -> impl UserEventHandler {
    move |ev: &UserEvent, map: &mut Map| {
        if let UserEvent::PointerMoved(event) = ev {
            let mut layer = feature_layer.write();

            let Some(position) = map.view().screen_to_map(event.screen_pointer_position) else {
                return EventPropagation::Stop;
            };

            for (_, feature) in layer.features_mut().iter_mut() {
                if feature.highlighted {
                    feature.highlighted = false;
                }
            }

            for (_, feature) in layer.get_features_at_mut(&position, map.view().resolution() * 20.0)
            {
                feature.highlighted = true;
            }

            layer.update_all_features();

            map.redraw();
        }
        galileo::control::EventPropagation::Propagate
    }
}

fn create_map() -> Map {
    let raster_layer = RasterTileLayerBuilder::new_osm()
        .with_file_cache_checked(".tile_cache")
        .build()
        .expect("failed to create layer");

    MapBuilder::default()
        .with_latlon(53.732562, -1.863383)
        .with_resolution(30.0)
        .with_layer(raster_layer)
        .build()
}

struct ColoredPointSymbol {
    default_image: Arc<DecodedImage>,
    highlighted_image: Arc<DecodedImage>,
}

impl Symbol<PointMarker> for ColoredPointSymbol {
    fn render(
        &self,
        feature: &PointMarker,
        geometry: &Geom<Point3>,
        _min_resolution: f64,
        bundle: &mut RenderBundle,
        view: &MapView,
    ) {
        if let Geom::Point(point) = geometry {
            let image = if feature.highlighted {
                self.default_image.clone()
            } else {
                self.highlighted_image.clone()
            };
            bundle.add_marker(
                point,
                &MarkerStyle::Image {
                    image,
                    anchor: Vector2::new(0.5, 1.0),
                    size: None,
                },
                view
            );
        }
    }
}
