//! This examples shows the map performance when rendering millions of points over it.

use galileo::layer::feature_layer::symbol::Symbol;
use galileo::layer::feature_layer::{Feature, FeatureLayer};
use galileo::render::point_paint::PointPaint;
use galileo::render::render_bundle::RenderPrimitive;
use galileo::tile_scheme::TileSchema;
use galileo::{Color, Map, MapBuilder, MapBuilderOld};
use galileo_types::cartesian::{CartesianPoint3d, Point3d};
use galileo_types::geo::Crs;
use galileo_types::geometry::Geom;
use galileo_types::impls::{Contour, Polygon};
use num_traits::AsPrimitive;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    run()
}

pub(crate) fn run() {
    galileo_egui::init(create_map(), []).expect("failed to initialize");
}

struct ColoredPoint {
    point: Point3d,
    color: Color,
}

impl Feature for ColoredPoint {
    type Geom = Point3d;

    fn geometry(&self) -> &Self::Geom {
        &self.point
    }
}

struct ColoredPointSymbol {}
impl Symbol<ColoredPoint> for ColoredPointSymbol {
    fn render<'a, N, P>(
        &self,
        feature: &ColoredPoint,
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
                PointPaint::dot(feature.color),
            )]
        } else {
            vec![]
        }
    }
}

fn generate_points() -> Vec<ColoredPoint> {
    const LEVELS: u32 = 100;
    let phi = std::f64::consts::PI * (5f64.sqrt() - 1.0);
    let mut points = vec![];

    for level in 1..=LEVELS {
        let points_count = level * level * 10;
        let radius = 50_000.0 * level as f64;

        let color = ((level - 1) as f32 / (LEVELS - 1) as f32 * 150.0) as u8;
        let color = Color::rgba(255, color, 0, 150);

        for i in 0..points_count {
            let z = 1.0 - (i as f64 / (points_count - 1) as f64);
            let rel_radius = (1.0 - z * z).sqrt();
            let theta = phi * i as f64;
            let x = theta.cos() * rel_radius;
            let y = theta.sin() * rel_radius;

            let point = Point3d::new(x * radius, y * radius, z * radius);
            points.push(ColoredPoint { point, color });
        }
    }

    log::info!("Generated {} points", points.len());

    points
}

fn create_map() -> Map {
    let tile_layer = MapBuilderOld::create_raster_tile_layer(
        |index| {
            format!(
                "https://tile.openstreetmap.org/{}/{}/{}.png",
                index.z, index.x, index.y
            )
        },
        TileSchema::web(18),
    );
    let feature_layer = FeatureLayer::new(generate_points(), ColoredPointSymbol {}, Crs::EPSG3857);

    MapBuilder::default()
        .with_layer(tile_layer)
        .with_layer(feature_layer)
        .build()
}
