use crate::primitives::Point2d;
use galileo_types::bounding_rect::BoundingRect;
use galileo_types::size::Size;
use galileo_types::CartesianPoint2d;
use nalgebra::{
    Matrix4, OMatrix, Perspective3, Point2, Point3, Rotation3, Scale3, Translation3, Vector2,
    Vector3, Vector4, U4,
};
use num_traits::Zero;

#[derive(Debug, Clone, Copy)]
pub struct MapView {
    position: Point3<f64>,
    resolution: f64,
    rotation_x: f64,
    rotation_z: f64,
    size: Size,
}

impl Default for MapView {
    fn default() -> Self {
        Self {
            position: Default::default(),
            resolution: 1.0,
            rotation_x: 0.0,
            rotation_z: 0.0,
            size: Size::new(0.0, 0.0),
        }
    }
}

impl MapView {
    pub fn new(position: impl CartesianPoint2d<Num = f64>, resolution: f64) -> Self {
        Self {
            position: Point3::new(position.x(), position.y(), 0.0),
            resolution,
            ..Default::default()
        }
    }

    pub fn resolution(&self) -> f64 {
        self.resolution
    }

    pub fn with_resolution(&self, resolution: f64) -> Self {
        Self {
            resolution,
            ..*self
        }
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn with_size(&self, new_size: Size) -> Self {
        Self {
            size: new_size,
            ..*self
        }
    }

    pub fn get_bbox(&self) -> BoundingRect {
        let points = [
            Point2::new(0.0, 0.0),
            Point2::new(self.size.width(), 0.0),
            Point2::new(0.0, self.size.height()),
            Point2::new(self.size.width(), self.size.height()),
        ];

        let points = points.map(|p| self.screen_to_map(p));
        let bbox = BoundingRect::from_points(points.iter()).unwrap();
        let max_bbox = BoundingRect::new(
            self.position.x - self.size.half_width() * self.resolution,
            self.position.y - self.size.half_height() * self.resolution,
            self.position.x + self.size.half_width() * self.resolution,
            self.position.y + self.size.half_height() * self.resolution,
        )
        .magnify(4.0);

        bbox.limit(max_bbox)
    }

    fn map_to_screen_center_transform(&self) -> OMatrix<f64, U4, U4> {
        if self.size.is_zero() {
            return Matrix4::identity();
        }

        let translate = Translation3::new(-self.position.x, -self.position.y, -self.position.z)
            .to_homogeneous();
        let rotation_x = Rotation3::new(Vector3::new(-self.rotation_x, 0.0, 0.0)).to_homogeneous();
        let rotation_z = Rotation3::new(Vector3::new(0.0, 0.0, self.rotation_z)).to_homogeneous();

        let scale = Scale3::new(
            1.0 / self.resolution,
            1.0 / self.resolution,
            1.0 / self.resolution,
        )
        .to_homogeneous();

        let translate_z = Translation3::new(0.0, 0.0, -self.size.height() / 2.0).to_homogeneous();
        let perspective = self.perspective();
        perspective * translate_z * scale * rotation_x * rotation_z * translate
    }

    fn perspective(&self) -> Matrix4<f64> {
        Perspective3::new(
            self.size.width() / self.size.height(),
            std::f64::consts::PI / 2.0,
            1.0 / self.size.height(),
            self.size.height(),
        )
        .to_homogeneous()
    }

    fn map_to_screen_transform(&self) -> Matrix4<f64> {
        let translate = Translation3::new(self.size.half_width(), -self.size.half_height(), 0.0)
            .to_homogeneous();
        let scale = Scale3::new(1.0, -1.0, 1.0).to_homogeneous();
        scale * translate * self.map_to_screen_center_transform()
    }

    fn screen_to_map_transform(&self) -> Matrix4<f64> {
        self.map_to_screen_transform().try_inverse().unwrap()
    }

    fn map_to_scene_transform(&self) -> Option<OMatrix<f64, U4, U4>> {
        let width = self.size.width();
        let height = self.size.height();

        if width.is_zero() || height.is_zero() || !width.is_finite() || !height.is_finite() {
            return None;
        }

        let scale = Scale3::new(1.0, 1.0, 0.5).to_homogeneous();
        Some(scale * self.map_to_screen_center_transform())
    }

    pub fn map_to_scene_mtx(&self) -> Option<[[f32; 4]; 4]> {
        Some(self.map_to_scene_transform()?.cast::<f32>().data.0)
    }

    pub fn rotation_x(&self) -> f64 {
        self.rotation_x
    }

    pub fn rotation_z(&self) -> f64 {
        self.rotation_z
    }

    pub fn with_rotation_x(&self, rotation_x: f64) -> Self {
        Self {
            rotation_x,
            ..*self
        }
    }

    pub fn with_rotation_z(&self, rotation_z: f64) -> Self {
        Self {
            rotation_z,
            ..*self
        }
    }

    pub fn with_rotation(&self, rotation_x: f64, rotation_z: f64) -> Self {
        Self {
            rotation_x,
            rotation_z,
            ..*self
        }
    }

    pub fn screen_to_map(&self, px_position: Point2d) -> Point2d {
        // todo: this must be calculated with matrices somehow but I'm not bright enough
        // to figure out how to do it...
        let x = px_position.x;
        let y = px_position.y;
        let a = (self.size.half_height() - y) * std::f64::consts::FRAC_PI_4.tan()
            / self.size.half_height();

        let s = 1.0 / ((std::f64::consts::FRAC_PI_2 - self.rotation_x).tan() / a - 1.0) + 1.0;

        let x0 = (x - self.size.half_width()) * self.resolution;
        let y0 = (self.size.half_height() - y) * self.resolution;

        if s.is_infinite() || s <= 0.0 {
            let x = if x0 >= 0.0 {
                f64::INFINITY
            } else {
                f64::NEG_INFINITY
            };
            let y = if y0 >= 0.0 {
                f64::INFINITY
            } else {
                f64::NEG_INFINITY
            };
            return Point2d::new(x, y);
        }

        let y0_ang = y0 / self.rotation_x.cos();

        let x0_scaled = x0 * s;
        let y0_scaled = y0_ang * s;

        let rotation_z = Rotation3::new(Vector3::new(0.0, 0.0, -self.rotation_z));
        let translation = Translation3::new(self.position.x, self.position.y, self.position.z);

        let p = Point3::new(x0_scaled, y0_scaled, 0.0);
        let transformed = translation * rotation_z * p;

        Point2::new(transformed.x, transformed.y)
    }

    pub fn translate_by_pixels(&self, from: Point2d, to: Point2d) -> Self {
        let transform = self.screen_to_map_transform();
        let from_projected = transform * Vector4::new(from.x, from.y, 0.0, 1.0);
        let to_projected = transform * Vector4::new(to.x, to.y, 0.0, 1.0);
        let delta = to_projected - from_projected;
        self.translate(delta.xy())
    }

    pub fn translate(&self, delta: Vector2<f64>) -> Self {
        let delta3 = Vector3::new(delta.x, delta.y, 0.0);
        Self {
            position: self.position - delta3,
            ..*self
        }
    }

    pub(crate) fn zoom(&self, zoom: f64, base_point: Point2d) -> Self {
        let base_point = self.screen_to_map(base_point);
        let resolution = self.resolution * zoom;
        let position2d = Point2::new(self.position.x, self.position.y);
        let new_position = base_point.add(((position2d - base_point) * zoom).into());
        Self {
            position: Point3::new(new_position.x, new_position.y, self.position.z),
            resolution,
            ..*self
        }
    }

    pub(crate) fn interpolate(&self, target: MapView, k: f64) -> Self {
        Self {
            position: self.position + (target.position - self.position) * k,
            resolution: self.resolution + (target.resolution - self.resolution) * k,
            ..*self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    #[test]
    fn screen_to_map_size() {
        let view = MapView::default().with_size(Size::new(100.0, 100.0));

        assert_abs_diff_eq!(
            view.screen_to_map(Point2d::new(0.0, 0.0)),
            Point2d::new(-50.0, 50.0),
            epsilon = 0.0001,
        );
        assert_abs_diff_eq!(
            view.screen_to_map(Point2d::new(50.0, 50.0)),
            Point2d::new(0.0, 0.0),
            epsilon = 0.0001,
        );

        let view = MapView::default().with_size(Size::new(200.0, 50.0));

        assert_abs_diff_eq!(
            view.screen_to_map(Point2d::new(0.0, 0.0)),
            Point2d::new(-100.0, 25.0),
            epsilon = 0.0001,
        );
        assert_abs_diff_eq!(
            view.screen_to_map(Point2d::new(25.0, 49.0)),
            Point2d::new(-75.0, -24.0),
            epsilon = 0.0001,
        );
    }

    #[test]
    fn screen_to_map_zero_size() {
        // todo: should this work like this?
        let view = MapView::default().with_size(Size::new(0.0, 0.0));
        let projected = view.screen_to_map(Point2d::new(0.0, 0.0));
        assert!(projected.x.is_nan());
        assert!(projected.y.is_nan());
    }

    #[test]
    fn screen_to_map_position() {
        let view = MapView {
            position: Point3::new(-100.0, -100.0, 0.0),
            size: Size::new(100.0, 100.0),
            ..Default::default()
        };

        // println!("-150, -50")

        assert_abs_diff_eq!(
            view.screen_to_map(Point2d::new(0.0, 0.0)),
            Point2d::new(-150.0, -50.0),
            epsilon = 0.0001,
        );
        assert_abs_diff_eq!(
            view.screen_to_map(Point2d::new(50.0, 50.0)),
            Point2d::new(-100.0, -100.0),
            epsilon = 0.0001,
        );
        assert_abs_diff_eq!(
            view.screen_to_map(Point2d::new(100.0, 100.0)),
            Point2d::new(-50.0, -150.0),
            epsilon = 0.0001,
        );
    }

    #[test]
    fn screen_to_map_resolution() {
        let view = MapView {
            resolution: 2.0,
            size: Size::new(100.0, 100.0),
            ..Default::default()
        };

        assert_abs_diff_eq!(
            view.screen_to_map(Point2d::new(0.0, 0.0)),
            Point2d::new(-100.0, 100.0),
            epsilon = 0.0001,
        );
        assert_abs_diff_eq!(
            view.screen_to_map(Point2d::new(100.0, 100.0)),
            Point2d::new(100.0, -100.0),
            epsilon = 0.0001,
        );
    }

    #[test]
    fn screen_to_map_rotation_x() {
        let view = MapView {
            rotation_x: std::f64::consts::PI / 4.0,
            size: Size::new(100.0, 100.0),
            ..Default::default()
        };

        assert_abs_diff_eq!(
            view.screen_to_map(Point2d::new(50.0, 50.0)),
            Point2d::new(0.0, 0.0),
            epsilon = 0.0001,
        );

        let projected = view.screen_to_map(Point2d::new(0.0, 0.0));
        assert_eq!(projected.x, f64::NEG_INFINITY);
        assert_eq!(projected.y, f64::INFINITY);

        assert_abs_diff_eq!(
            view.screen_to_map(Point2d::new(100.0, 100.0)),
            Point2d::new(25.0, -35.35),
            epsilon = 0.1,
        );
    }

    #[test]
    fn map_to_scene() {
        let view = MapView::default().with_size(Size::new(100.0, 100.0));
        let point = Point3::new(-50.0, 50.0, 0.0).to_homogeneous();
        let transform = view.map_to_scene_transform().unwrap();
        let projected = transform * point;

        assert_abs_diff_eq!(
            projected.unscale(projected.w),
            Point3::new(-1.0, 1.0, 0.5).to_homogeneous(),
            epsilon = 0.01
        );

        let point = Point3::new(0.0, 0.0, 0.0).to_homogeneous();
        let transform = view.map_to_scene_transform().unwrap();
        let projected = transform * point;

        println!("{projected}");

        assert_abs_diff_eq!(
            projected.unscale(projected.w),
            Point3::new(0.0, 0.0, 0.5).to_homogeneous(),
            epsilon = 0.01
        );
    }
}
