//! This example shows how to change a feature appearance based on use input - pointing with a
//! mouse in this case.

use std::sync::Arc;

use galileo::control::{EventPropagation, UserEvent};
use galileo::decoded_image::DecodedImage;
use galileo::layer::feature_layer::symbol::Symbol;
use galileo::layer::feature_layer::{Feature, FeatureLayer};
use galileo::render::point_paint::PointPaint;
use galileo::render::render_bundle::RenderPrimitive;
use galileo::tile_scheme::TileSchema;
use galileo::MapBuilder;
use galileo_types::cartesian::{CartesianPoint3d, Point2d};
use galileo_types::geo::{Crs, Projection};
use galileo_types::geometry::Geom;
use galileo_types::impls::{Contour, Polygon};
use galileo_types::{latlon, CartesianGeometry2d, Geometry};
use lazy_static::lazy_static;
use nalgebra::Vector2;
use num_traits::AsPrimitive;
use parking_lot::RwLock;

const YELLOW_PIN: &[u8] = include_bytes!("data/pin-yellow.png");
const GREEN_PIN: &[u8] = include_bytes!("data/pin-green.png");

lazy_static! {
    static ref YELLOW_PIN_IMAGE: Arc<DecodedImage> =
        Arc::new(DecodedImage::decode(YELLOW_PIN).expect("Must have Yellow Pin Image"));
    static ref GREEN_PIN_IMAGE: Arc<DecodedImage> =
        Arc::new(DecodedImage::decode(GREEN_PIN).expect("Must have Green Pin Image"));
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    run(MapBuilder::new()).await;
}

#[derive(Debug, PartialEq, Default)]
pub(crate) struct PointMarker {
    pub(crate) point: Point2d,
    pub(crate) highlighted: bool,
}

impl Feature for PointMarker {
    type Geom = Self;

    fn geometry(&self) -> &Self::Geom {
        self
    }
}

impl Geometry for PointMarker {
    type Point = Point2d;

    fn project<P: Projection<InPoint = Self::Point> + ?Sized>(
        &self,
        projection: &P,
    ) -> Option<Geom<P::OutPoint>> {
        self.point.project(projection)
    }
}

impl CartesianGeometry2d<Point2d> for PointMarker {
    fn is_point_inside<
        Other: galileo_types::cartesian::CartesianPoint2d<
            Num = <Point2d as galileo_types::cartesian::CartesianPoint2d>::Num,
        >,
    >(
        &self,
        point: &Other,
        tolerance: <Point2d as galileo_types::cartesian::CartesianPoint2d>::Num,
    ) -> bool {
        self.point.is_point_inside(point, tolerance)
    }

    fn bounding_rectangle(
        &self,
    ) -> Option<
        galileo_types::cartesian::Rect<
            <Point2d as galileo_types::cartesian::CartesianPoint2d>::Num,
        >,
    > {
        None
    }
}

struct ColoredPointSymbol {}

pub(crate) async fn run(builder: MapBuilder) {
    #[cfg(not(target_arch = "wasm32"))]
    let builder = builder.with_raster_tiles(
        |index| {
            format!(
                "https://tile.openstreetmap.org/{}/{}/{}.png",
                index.z, index.x, index.y
            )
        },
        TileSchema::web(18),
    );

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
        ColoredPointSymbol {},
        Crs::EPSG3857,
    );

    let feature_layer = Arc::new(RwLock::new(feature_layer));

    builder
        .center(latlon!(53.732562, -1.863383))
        .resolution(30.0)
        .with_layer(feature_layer.clone())
        .with_event_handler(move |ev, map| {
            if let UserEvent::PointerMoved(event) = ev {
                let mut layer = feature_layer.write();

                let Some(position) = map.view().screen_to_map(event.screen_pointer_position) else {
                    return EventPropagation::Stop;
                };

                for mut feature_container in layer.features_mut().iter_mut() {
                    feature_container.as_mut().highlighted = false;
                }

                for mut feature_container in
                    layer.get_features_at_mut(&position, map.view().resolution() * 20.0)
                {
                    let state = feature_container.as_ref().highlighted;
                    feature_container.as_mut().highlighted = !state;
                }

                map.redraw();
            }
            galileo::control::EventPropagation::Propagate
        })
        .build()
        .await
        .run();
}

impl Symbol<PointMarker> for ColoredPointSymbol {
    fn render<'a, N, P>(
        &self,
        feature: &PointMarker,
        geometry: &'a Geom<P>,
        _min_resolution: f64,
    ) -> Vec<RenderPrimitive<'a, N, P, Contour<P>, Polygon<P>>>
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N> + Clone,
    {
        if let Geom::Point(point) = geometry {
            vec![RenderPrimitive::new_point(
                point.clone(),
                PointPaint::image(
                    if feature.highlighted {
                        GREEN_PIN_IMAGE.clone()
                    } else {
                        YELLOW_PIN_IMAGE.clone()
                    },
                    Vector2::new(0.5, 0.5),
                    1.0,
                ),
            )]
        } else {
            vec![]
        }
    }
}
