use galileo::galileo_map::MapBuilder;
use galileo::layer::feature_layer::feature::Feature;
use galileo::layer::feature_layer::symbol::Symbol;
use galileo::layer::feature_layer::FeatureLayer;
use galileo::primitives::Color;
use galileo::render::{PointPaint, PrimitiveId, RenderBundle};
use galileo::tile_scheme::TileScheme;
use galileo_types::cartesian::impls::point::Point3d;
use galileo_types::geo::crs::Crs;

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    run(MapBuilder::new()).await;
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
impl Symbol<ColoredPoint, Point3d> for ColoredPointSymbol {
    fn render(
        &self,
        feature: &ColoredPoint,
        geometry: &Point3d,
        bundle: &mut Box<dyn RenderBundle>,
    ) -> Vec<PrimitiveId> {
        vec![bundle.add_point(
            geometry,
            PointPaint {
                color: feature.color,
                size: 3.0,
            },
        )]
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
        let color = Color::rgba(255, color, 0, 20);

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

pub async fn run(builder: MapBuilder) {
    builder
        .with_raster_tiles(
            |index| {
                format!(
                    "https://tile.openstreetmap.org/{}/{}/{}.png",
                    index.z, index.x, index.y
                )
            },
            TileScheme::web(18),
        )
        .with_layer(FeatureLayer::new(
            generate_points(),
            ColoredPointSymbol {},
            Crs::EPSG3857,
        ))
        .build()
        .await
        .run();
}
