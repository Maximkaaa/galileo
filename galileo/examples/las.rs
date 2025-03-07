//! This example demonstrates rendering of 19_000_000 points from lidar scanning on the map. To be run it requires
//! a file `Clifton_Suspension_Bridge.laz` to be added to the `./data` directory. This file is too large to be added
//! to git repository, but it can be downloaded from https://geoslam.com/sample-data/
//!
//! Running this example requires powerful GPU. It doesn't do any optimizations and simplifications on purpose,
//! to demonstrate the capability of Galileo engine to render large amounts of data. In real applications the input
//! data should probably be preprocessed, as many of those 19M points have virtually the same coordinate. These
//! optimizations may be done by Galileo in future, but at this point it's up to the application.

use galileo::layer::feature_layer::{Feature, FeatureLayerOptions};
use galileo::layer::raster_tile_layer::RasterTileLayerBuilder;
use galileo::layer::FeatureLayer;
use galileo::render::point_paint::PointPaint;
use galileo::render::render_bundle::RenderBundle;
use galileo::symbol::Symbol;
use galileo::{Color, Map, MapBuilder};
use galileo_types::cartesian::Point3;
use galileo_types::geo::Crs;
use galileo_types::geometry::Geom;
use las::Read;
use nalgebra::{Rotation3, Translation3, Vector3};

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    run()
}

pub(crate) fn run() {
    galileo_egui::init(create_map(), []).expect("failed to initialize");
}

fn load_points() -> Vec<ColoredPoint> {
    let mut reader =
        las::Reader::from_path("./galileo/examples/data/Clifton_Suspension_Bridge.laz")
            .expect("invalid laz file");
    log::info!("Header: {:?}", reader.header());

    log::info!(
        "{}",
        u16::from_be_bytes(
            reader.header().vlrs()[0].data[0..2]
                .try_into()
                .expect("invalid laz file")
        )
    );
    let x = -292613.7773218893 - 249.0;
    let y = 6702106.413514771 - 142.0;
    let z = 5.0;

    let lat = 51f64;
    let scale_lat = 1.0 / lat.to_radians().cos();

    let rotation = Rotation3::new(Vector3::new(0.0, 0.0, -37f64.to_radians()));
    let translation = Translation3::new(x, y, z);
    let transform = translation * rotation;

    let mut counter = 0;
    reader
        .points()
        .map(|p| {
            if counter % 1_000_000 == 0 {
                eprintln!("Loaded {counter} points");
            }

            counter += 1;

            let p = p.expect("invalid laz file");
            let color = p.color.expect("invalid laz file");
            let point = nalgebra::Point3::new(p.x * scale_lat, p.y * scale_lat, p.z * scale_lat);
            let point = transform * point;
            let point = Point3::new(point.x, point.y, point.z);
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
    point: Point3,
    color: Color,
}

impl Feature for ColoredPoint {
    type Geom = Point3;

    fn geometry(&self) -> &Self::Geom {
        &self.point
    }
}

struct ColoredPointSymbol {}
impl Symbol<ColoredPoint> for ColoredPointSymbol {
    fn render(
        &self,
        feature: &ColoredPoint,
        geometry: &Geom<Point3>,
        min_resolution: f64,
        bundle: &mut RenderBundle,
    ) {
        if let Geom::Point(point) = geometry {
            bundle.add_point(point, &PointPaint::dot(feature.color), min_resolution);
        }
    }
}

fn create_map() -> Map {
    let raster_layer = RasterTileLayerBuilder::new_osm()
        .with_file_cache_checked(".tile_cache")
        .build()
        .expect("failed to create layer");

    let points = load_points();
    let feature_layer = FeatureLayer::new(points, ColoredPointSymbol {}, Crs::EPSG3857)
        .with_options(FeatureLayerOptions {
            buffer_size_limit: 200_000_000,
            use_antialiasing: true,
            ..Default::default()
        });

    MapBuilder::default()
        .with_latlon(51.4549, -2.6279)
        .with_z_level(17)
        .with_layer(raster_layer)
        .with_layer(feature_layer)
        .build()
}
