/// This example demonstrates rendering of 19_000_000 points from lidar scanning on the map. To be run it requires
/// a file `Clifton_Suspension_Bridge.laz` to be added to the `./data` directory. This file is too large to be added
/// to git repository, but it can be downloaded from https://geoslam.com/sample-data/
///
/// Running this example requires powerful GPU. It doesn't do any optimizations and simplifications on purpose,
/// to demonstrate the capability of Galileo engine to render large amounts of data. In real applications the input
/// data should probably be preprocessed, as many of those 19M points have virtually the same coordinate. These
/// optimizations may be done by Galileo in future, but at this point it's up to the application.
use galileo::galileo_map::MapBuilder;
use galileo::layer::feature_layer::feature::Feature;
use galileo::layer::FeatureLayer;
use galileo::render::point_paint::PointPaint;
use galileo::render::render_bundle::RenderBundle;
use galileo::render::PrimitiveId;
use galileo::symbol::Symbol;
use galileo::tile_scheme::TileSchema;
use galileo::Color;
use galileo_types::cartesian::impls::point::Point3d;
use galileo_types::cartesian::traits::cartesian_point::CartesianPoint3d;
use galileo_types::geo::crs::Crs;
use galileo_types::geometry::Geom;
use galileo_types::latlon;
use las::Read;
use nalgebra::{Rotation3, Translation3, Vector3};
use num_traits::AsPrimitive;

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    run(MapBuilder::new()).await;
}

fn load_points() -> Vec<ColoredPoint> {
    let mut reader =
        las::Reader::from_path("./galileo/examples/data/Clifton_Suspension_Bridge.laz").unwrap();
    log::info!("Header: {:?}", reader.header());

    log::info!(
        "{}",
        u16::from_be_bytes(reader.header().vlrs()[0].data[0..2].try_into().unwrap())
    );
    let x = -292613.7773218893 - 249.0;
    let y = 6702106.413514771 - 142.0;
    let z = 5.0;

    let lat = 51f64;
    let scale_lat = 1.0 / lat.to_radians().cos();

    let rotation = Rotation3::new(Vector3::new(0.0, 0.0, -37f64.to_radians()));
    let translation = Translation3::new(x, y, z);
    let transform = translation * rotation;

    reader
        .points()
        .map(|p| {
            let p = p.unwrap();
            let color = p.color.unwrap();
            let point = Point3d::new(p.x * scale_lat, p.y * scale_lat, p.z * scale_lat);
            let point: Point3d = transform * point;
            ColoredPoint {
                point,
                color: Color::rgba(
                    (color.red / 255) as u8,
                    (color.green / 255) as u8,
                    (color.blue / 255) as u8,
                    255,
                ),
            }
        })
        .collect()
}

#[derive(Clone)]
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
    fn render<N: AsPrimitive<f32>, P: CartesianPoint3d<Num = N>>(
        &self,
        feature: &ColoredPoint,
        geometry: &Geom<P>,
        bundle: &mut RenderBundle,
        _min_resolution: f64,
    ) -> Vec<PrimitiveId> {
        if let Geom::Point(point) = geometry {
            vec![bundle.add_point(point, PointPaint::dot(feature.color))]
        } else {
            vec![]
        }
    }

    fn use_antialiasing(&self) -> bool {
        false
    }
}

pub async fn run(builder: MapBuilder) {
    let points = load_points();
    builder
        .center(latlon!(51.4549, -2.6279))
        .resolution(TileSchema::web(18).lod_resolution(14).unwrap())
        .with_raster_tiles(
            |index| {
                format!(
                    "https://tile.openstreetmap.org/{}/{}/{}.png",
                    index.z, index.x, index.y
                )
            },
            TileSchema::web(18),
        )
        .with_layer(FeatureLayer::new(
            points,
            ColoredPointSymbol {},
            Crs::EPSG3857,
        ))
        .build()
        .await
        .run();
}
