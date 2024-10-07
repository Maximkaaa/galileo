use galileo_types::cartesian::{
    CartesianPoint2d, CartesianPoint3d, Point2, Point3, Rect, Size, Vector2, Vector3,
};
use galileo_types::geo::impls::GeoPoint2d;
use galileo_types::geo::{Crs, GeoPoint};
use nalgebra::{Matrix4, OMatrix, Perspective3, Rotation3, Scale3, Translation3, U4};

/// Map view specifies the area of the map that should be drawn. In other words, it sets the position of "camera" that
/// looks at the map.
///
/// The main view parameters are:
/// * position - coordinates of the point in the center of the map
/// * resolution - number of map units in a single pixel at the center of the map
/// * size - size of the rendering area in pixels
/// * crs - coordinate system that the map will be rendered to. This specifies the geographic projection that the map is
///   displayed in. Note, that currently geographic CRSs are not supported, and a map with such a view will not be
///   drawn.
///
/// The view can also specify rotation along *x* (tilt) and *z* (rotation) axis.
#[derive(Debug, Clone)]
pub struct MapView {
    projected_position: Option<Point3<f64>>,
    resolution: f64,
    rotation_x: f64,
    rotation_z: f64,
    size: Size,
    crs: Crs,
}

impl MapView {
    /// Creates a new view with the given position and resolution with default CRS (web-mercator EPSG:3857).
    pub fn new(position: &impl GeoPoint<Num = f64>, resolution: f64) -> Self {
        Self::new_with_crs(position, resolution, Crs::EPSG3857)
    }

    /// Creates a new view with the given CRS.
    pub fn new_with_crs(position: &impl GeoPoint<Num = f64>, resolution: f64, crs: Crs) -> Self {
        let projected = crs
            .get_projection()
            .and_then(|projection| projection.project(&GeoPoint2d::from(position)))
            .map(|p: Point2| Point3::new(p.x(), p.y(), 0.0));
        Self {
            projected_position: projected,
            resolution,
            rotation_z: 0.0,
            rotation_x: 0.0,
            size: Default::default(),
            crs,
        }
    }

    /// Creates a new view, taking position value as projected coordinates. Default CRS is used (EPSG:3857).
    pub fn new_projected(position: &impl CartesianPoint2d<Num = f64>, resolution: f64) -> Self {
        Self::new_projected_with_crs(position, resolution, Crs::EPSG3857)
    }

    /// Creates a new view, taking position value as projected coordinates.
    pub fn new_projected_with_crs(
        position: &impl CartesianPoint2d<Num = f64>,
        resolution: f64,
        crs: Crs,
    ) -> Self {
        Self {
            projected_position: Some(Point3::new(position.x(), position.y(), 0.0)),
            resolution,
            rotation_z: 0.0,
            rotation_x: 0.0,
            size: Default::default(),
            crs,
        }
    }

    /// CRS of the view.
    pub fn crs(&self) -> &Crs {
        &self.crs
    }

    /// Position of the center point of the map (screen).
    ///
    /// If the projected position cannot be projected into geographic coordinates, `None` is returned.
    pub fn position(&self) -> Option<GeoPoint2d> {
        self.projected_position.and_then(|p| {
            self.crs
                .get_projection()
                .and_then(|proj| proj.unproject(&Point2::new(p.x(), p.y())))
        })
    }

    /// Creates a new view same as the current one but with the given position.
    pub fn with_position(&self, position: &impl GeoPoint<Num = f64>) -> Self {
        let projected_position = self
            .crs
            .get_projection()
            .and_then(|projection| projection.project(&GeoPoint2d::from(position)))
            .map(|p: Point2| Point3::new(p.x(), p.y(), 0.0));
        Self {
            projected_position,
            crs: self.crs.clone(),
            ..*self
        }
    }

    /// Resolution at the center of the map.
    pub fn resolution(&self) -> f64 {
        self.resolution
    }

    /// Creates a new view, same as the current one, but with the given resolution.
    pub fn with_resolution(&self, resolution: f64) -> Self {
        Self {
            resolution,
            crs: self.crs.clone(),
            ..*self
        }
    }

    /// Size of the view in pixels.
    pub fn size(&self) -> Size {
        self.size
    }

    /// Creates a new view, same as the current one, but with the given size.
    pub fn with_size(&self, new_size: Size) -> Self {
        Self {
            size: new_size,
            crs: self.crs.clone(),
            ..*self
        }
    }

    /// Returns bounding rectangle of the view (in projected coordinates).
    pub fn get_bbox(&self) -> Option<Rect> {
        let points = [
            Point2::new(0.0, 0.0),
            Point2::new(self.size.width(), 0.0),
            Point2::new(0.0, self.size.height()),
            Point2::new(self.size.width(), self.size.height()),
        ];

        let position = self.projected_position?;
        let max_bbox = Rect::new(
            position.x() - self.size.half_width() * self.resolution,
            position.y() - self.size.half_height() * self.resolution,
            position.x() + self.size.half_width() * self.resolution,
            position.y() + self.size.half_height() * self.resolution,
        )
        .magnify(4.0);

        if let Some(points) = points
            .into_iter()
            .map(|p| self.screen_to_map(p))
            .collect::<Option<Vec<Point2<f64>>>>()
        {
            let bbox = Rect::from_points(points.iter())?;
            Some(bbox.limit(max_bbox))
        } else {
            Some(max_bbox)
        }
    }

    fn map_to_screen_center_transform(&self) -> Option<OMatrix<f64, U4, U4>> {
        if self.size.is_zero() {
            return None;
        }

        let position = self.projected_position?;
        let x = (position.x() / self.resolution).round() * self.resolution;
        let y = (position.y() / self.resolution).round() * self.resolution;
        let z = (position.z() / self.resolution).round() * self.resolution;
        let translate = Translation3::new(-x, -y, -z).to_homogeneous();
        let rotation_x =
            Rotation3::new(nalgebra::Vector3::new(-self.rotation_x, 0.0, 0.0)).to_homogeneous();
        let rotation_z =
            Rotation3::new(nalgebra::Vector3::new(0.0, 0.0, self.rotation_z)).to_homogeneous();

        let scale = Scale3::new(
            1.0 / self.resolution,
            1.0 / self.resolution,
            1.0 / self.resolution,
        )
        .to_homogeneous();

        let translate_z = Translation3::new(0.0, 0.0, -self.size.height() / 2.0).to_homogeneous();
        let perspective = self.perspective();
        Some(perspective * translate_z * scale * rotation_x * rotation_z * translate)
    }

    fn perspective(&self) -> Matrix4<f64> {
        Perspective3::new(
            self.size.width() / self.size.height(),
            std::f64::consts::PI / 2.0,
            10.0,
            self.size.height(),
        )
        .to_homogeneous()
    }

    /// Returns transformation matrix that transforms map coordinates to scene coordinates.
    ///
    /// Scene coordinates are `[-1.0, 1.0]` coordinates of the render area with *Y* going from bottom to top.
    pub fn map_to_scene_transform(&self) -> Option<OMatrix<f64, U4, U4>> {
        let scale = Scale3::new(1.0, 1.0, 0.5).to_homogeneous();
        Some(scale * self.map_to_screen_center_transform()?)
    }

    /// Returns transformation matrix that transforms map coordinates to scene coordinates.
    ///
    /// Scene coordinates are `[-1.0, 1.0]` coordinates of the render area with *Y* going from bottom to top.
    pub fn map_to_scene_mtx(&self) -> Option<[[f32; 4]; 4]> {
        Some(self.map_to_scene_transform()?.cast::<f32>().data.0)
    }

    /// Rotation angle around *X* axis in radians (tilt).
    pub fn rotation_x(&self) -> f64 {
        self.rotation_x
    }

    /// Rotation angle around *Z* axis in radians.
    pub fn rotation_z(&self) -> f64 {
        self.rotation_z
    }

    /// Creates a new view, same as the current one, but with the given rotation x.
    pub fn with_rotation_x(&self, rotation_x: f64) -> Self {
        Self {
            rotation_x,
            crs: self.crs.clone(),
            ..*self
        }
    }

    /// Creates a new view, same as the current one, but with the given rotation z.
    pub fn with_rotation_z(&self, rotation_z: f64) -> Self {
        Self {
            rotation_z,
            crs: self.crs.clone(),
            ..*self
        }
    }

    /// Creates a new view, same as the current one, but with the given rotation values.
    pub fn with_rotation(&self, rotation_x: f64, rotation_z: f64) -> Self {
        Self {
            rotation_x,
            rotation_z,
            crs: self.crs.clone(),
            ..*self
        }
    }

    /// Projects the given screen point into map coordinates at the 0 elevation.
    ///
    /// Returns `None` if the point is outside of map (this can be possible, if the map is tilted and the point is
    /// above the horizon, or if the point is outside the projection bounds).
    pub fn screen_to_map(&self, px_position: Point2) -> Option<Point2> {
        // todo: this must be calculated with matrices somehow but I'm not bright enough
        // to figure out how to do it...
        let x = px_position.x();
        let y = px_position.y();
        let a = (self.size.half_height() - y) * std::f64::consts::FRAC_PI_4.tan()
            / self.size.half_height();

        let s = 1.0 / ((std::f64::consts::FRAC_PI_2 - self.rotation_x).tan() / a - 1.0) + 1.0;

        let x0 = (x - self.size.half_width()) * self.resolution;
        let y0 = (self.size.half_height() - y) * self.resolution;

        if s.is_infinite() || s.is_nan() || s <= 0.0 {
            return None;
        }

        let y0_ang = y0 / self.rotation_x.cos();

        let x0_scaled = x0 * s;
        let y0_scaled = y0_ang * s;

        let rotation_z = Rotation3::new(nalgebra::Vector3::new(0.0, 0.0, -self.rotation_z));
        let position = self.projected_position?;
        let translation = Translation3::new(position.x(), position.y(), position.z());

        let p = nalgebra::Point3::new(x0_scaled, y0_scaled, 0.0);
        let transformed = translation * rotation_z * p;

        Some(Point2::new(transformed.x, transformed.y))
    }

    /// Projects the given screen point into map coordinates at the 0 elevation, and then projects them into
    /// geographic coordinates.
    ///
    /// Returns `None` if the point is outside of map (this can be possible, if the map is tilted and the point is
    /// above the horizon, or if the point is outside the projection bounds).
    pub fn screen_to_map_geo(&self, px_position: Point2) -> Option<GeoPoint2d> {
        self.screen_to_map(px_position).and_then(|p| {
            self.crs
                .get_projection()
                .and_then(|proj| proj.unproject(&Point2::new(p.x(), p.y())))
        })
    }

    /// Creates a new view, same as the current one, but translated so that point `from` on the current view becomes
    /// the point `to` in the new view.
    pub fn translate_by_pixels(&self, from: Point2, to: Point2) -> Self {
        let Some(from_projected) = self.screen_to_map(from) else {
            return self.clone();
        };
        let Some(to_projected) = self.screen_to_map(to) else {
            return self.clone();
        };

        const MAX_TRANSLATE: f64 = 100.0;
        let max_translate = MAX_TRANSLATE * self.resolution;
        let mut delta = to_projected - from_projected;
        if delta.dx().abs() > max_translate {
            delta.set_dx(max_translate * delta.dx().signum());
        }
        if delta.dy().abs() > max_translate {
            delta.set_dy(max_translate * delta.dy().signum());
        }

        self.translate(delta)
    }

    /// Move the view by the given projected coordinates delta.
    pub fn translate(&self, delta: Vector2<f64>) -> Self {
        match self.projected_position {
            Some(v) => {
                let projected_position = v - Vector3::new(delta.dx(), delta.dy(), 0.0);
                Self {
                    projected_position: Some(projected_position),
                    crs: self.crs.clone(),
                    ..*self
                }
            }
            None => Self {
                crs: self.crs.clone(),
                ..*self
            },
        }
    }

    pub fn zoom(&self, zoom: f64, base_point: Point2) -> Self {
        let base_point = self.screen_to_map(base_point);
        let resolution = self.resolution * zoom;

        let new_position = base_point.and_then(|base_point| {
            self.projected_position.map(|position| {
                let position2d = Point2::new(position.x(), position.y());
                let result = base_point.add((position2d - base_point) * zoom);
                Point3::new(result.x(), result.y(), position.z())
            })
        });

        Self {
            projected_position: new_position,
            resolution,
            crs: self.crs.clone(),
            ..*self
        }
    }

    pub fn interpolate(&self, target: &MapView, k: f64) -> Self {
        let Some(source_position) = self.projected_position else {
            return self.clone();
        };
        let Some(target_position) = target.projected_position else {
            return self.clone();
        };

        let projected_position = source_position + (target_position - source_position) * k;
        Self {
            projected_position: Some(projected_position),
            resolution: self.resolution + (target.resolution - self.resolution) * k,
            crs: self.crs.clone(),
            ..*self
        }
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;

    use super::*;

    fn test_view() -> MapView {
        MapView::new_projected(&Point2::new(0.0, 0.0), 1.0)
    }

    #[test]
    fn screen_to_map_size() {
        let view = test_view().with_size(Size::new(100.0, 100.0));

        assert_abs_diff_eq!(
            view.screen_to_map(Point2::new(0.0, 0.0)).unwrap(),
            Point2::new(-50.0, 50.0),
            epsilon = 0.0001,
        );
        assert_abs_diff_eq!(
            view.screen_to_map(Point2::new(50.0, 50.0)).unwrap(),
            Point2::new(0.0, 0.0),
            epsilon = 0.0001,
        );

        let view = test_view().with_size(Size::new(200.0, 50.0));

        assert_abs_diff_eq!(
            view.screen_to_map(Point2::new(0.0, 0.0)).unwrap(),
            Point2::new(-100.0, 25.0),
            epsilon = 0.0001,
        );
        assert_abs_diff_eq!(
            view.screen_to_map(Point2::new(25.0, 49.0)).unwrap(),
            Point2::new(-75.0, -24.0),
            epsilon = 0.0001,
        );
    }

    #[test]
    fn screen_to_map_zero_size() {
        let view = test_view().with_size(Size::new(0.0, 0.0));
        let projected = view.screen_to_map(Point2::new(0.0, 0.0));
        assert!(projected.is_none());
    }

    #[test]
    fn screen_to_map_position() {
        let view = MapView::new_projected(&Point2::new(-100.0, -100.0), 1.0)
            .with_size(Size::new(100.0, 100.0));

        assert_abs_diff_eq!(
            view.screen_to_map(Point2::new(0.0, 0.0)).unwrap(),
            Point2::new(-150.0, -50.0),
            epsilon = 0.0001,
        );
        assert_abs_diff_eq!(
            view.screen_to_map(Point2::new(50.0, 50.0)).unwrap(),
            Point2::new(-100.0, -100.0),
            epsilon = 0.0001,
        );
        assert_abs_diff_eq!(
            view.screen_to_map(Point2::new(100.0, 100.0)).unwrap(),
            Point2::new(-50.0, -150.0),
            epsilon = 0.0001,
        );
    }

    #[test]
    fn screen_to_map_resolution() {
        let view = test_view()
            .with_resolution(2.0)
            .with_size(Size::new(100.0, 100.0));

        assert_abs_diff_eq!(
            view.screen_to_map(Point2::new(0.0, 0.0)).unwrap(),
            Point2::new(-100.0, 100.0),
            epsilon = 0.0001,
        );
        assert_abs_diff_eq!(
            view.screen_to_map(Point2::new(100.0, 100.0)).unwrap(),
            Point2::new(100.0, -100.0),
            epsilon = 0.0001,
        );
    }

    #[test]
    fn screen_to_map_rotation_x() {
        let view = test_view()
            .with_rotation_x(std::f64::consts::PI / 4.0)
            .with_size(Size::new(100.0, 100.0));

        assert_abs_diff_eq!(
            view.screen_to_map(Point2::new(50.0, 50.0)).unwrap(),
            Point2::new(0.0, 0.0),
            epsilon = 0.0001,
        );

        let projected = view.screen_to_map(Point2::new(0.0, 0.0));
        assert!(projected.is_none());

        assert_abs_diff_eq!(
            view.screen_to_map(Point2::new(100.0, 100.0)).unwrap(),
            Point2::new(25.0, -35.35),
            epsilon = 0.1,
        );
    }

    #[test]
    fn map_to_scene() {
        let view = test_view().with_size(Size::new(100.0, 100.0));
        let point = nalgebra::Point3::new(-50.0, 50.0, 0.0).to_homogeneous();
        let transform = view.map_to_scene_transform().unwrap();
        let projected = transform * point;

        assert_abs_diff_eq!(
            projected.unscale(projected.w),
            nalgebra::Point3::new(-1.0, 1.0, 0.388).to_homogeneous(),
            epsilon = 0.01
        );

        let point = nalgebra::Point3::new(0.0, 0.0, 0.0).to_homogeneous();
        let transform = view.map_to_scene_transform().unwrap();
        let projected = transform * point;

        assert_abs_diff_eq!(
            projected.unscale(projected.w),
            nalgebra::Point3::new(0.0, 0.0, 0.3888).to_homogeneous(),
            epsilon = 0.01
        );
    }
}
