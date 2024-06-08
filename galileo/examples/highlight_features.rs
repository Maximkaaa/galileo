use std::sync::{Arc, RwLock};

use galileo::control::{EventPropagation, UserEvent};
use galileo::layer::feature_layer::symbol::Symbol;
use galileo::layer::feature_layer::{Feature, FeatureLayer};
use galileo::render::point_paint::PointPaint;
use galileo::render::render_bundle::RenderPrimitive;
use galileo::tile_scheme::TileSchema;
use galileo::{Color, MapBuilder};
use galileo_types::cartesian::{CartesianPoint3d, Point2d};
use galileo_types::geo::{Crs, Projection};
use galileo_types::geometry::Geom;
use galileo_types::impls::{Contour, Polygon};
use galileo_types::{CartesianGeometry2d, Geometry};
use num_traits::AsPrimitive;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    run(MapBuilder::new()).await;
}

#[derive(Debug)]
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

fn generate_points() -> Vec<PointMarker> {
    const LEVELS: u32 = 2;
    let phi = std::f64::consts::PI * (5f64.sqrt() - 1.0);
    let mut points = vec![];

    for level in 1..=LEVELS {
        let points_count = level * level * 10;
        let radius = 50_000.0 * level as f64;

        for i in 0..points_count {
            let z = 1.0 - (i as f64 / (points_count - 1) as f64);
            let rel_radius = (1.0 - z * z).sqrt();
            let theta = phi * i as f64;
            let x = theta.cos() * rel_radius;
            let y = theta.sin() * rel_radius;

            let point = Point2d::new(x * radius, y * radius);
            points.push(PointMarker {
                point,
                highlighted: false,
            });
        }
    }

    log::info!("Generated {} points", points.len());

    points
}

pub async fn run(builder: MapBuilder) {
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

    let generated_point_markers = generate_points();

    let feature_layer = FeatureLayer::new(
        generated_point_markers,
        ColoredPointSymbol {},
        Crs::EPSG3857,
    );

    let feature_layer = Arc::new(RwLock::new(feature_layer));

    builder
        .with_layer(feature_layer.clone())
        .with_event_handler(move |ev, map| {
            if let UserEvent::PointerMoved(event) = ev {
                let mut layer = feature_layer.write().unwrap();

                let Some(position) = map.view().screen_to_map(event.screen_pointer_position) else {
                    return EventPropagation::Stop;
                };

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
            vec![RenderPrimitive::new_point_ref(
                point,
                PointPaint::circle(
                    if feature.highlighted {
                        Color::rgba(255, 0, 0, 128)
                    } else {
                        Color::rgba(0, 255, 0, 128)
                    },
                    100.0,
                ),
            )]
        } else {
            vec![]
        }
    }
}
