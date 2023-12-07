use crate::bounding_box::BoundingBox;
use crate::primitives::{Point2d, Size};
use galileo_types::vec::Vec2d;
use galileo_types::CartesianPoint2d;

#[derive(Debug, Default, Clone, Copy)]
pub struct MapView {
    pub position: Point2d,
    pub resolution: f64,
    
}

impl MapView {
    pub fn resolution(&self) -> f64 {
        self.resolution
    }

    pub fn get_bbox(&self, map_size: Size) -> BoundingBox {
        let half_width = map_size.width as f64 * self.resolution / 2.0;
        let half_height = map_size.height as f64 * self.resolution / 2.0;
        BoundingBox::new(
            self.position.x() - half_width,
            self.position.y() - half_height,
            self.position.x() + half_width,
            self.position.y() + half_height,
        )
    }

    pub fn get_transform_mtx(&self, map_size: Size) -> [[f32; 4]; 4] {
        let res = self.resolution as f32;

        let vp_sx = 2.0 / map_size.width as f32;
        let vp_sy = 2.0 / map_size.height as f32;

        let sx = vp_sx / res;
        let sy = vp_sy / res;
        let tx = -self.position.x() as f32 * sx;
        let ty = -self.position.y() as f32 * sy;

        [
            [sx, 0.0, 0.0, 0.0],
            [0.0, sy, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [tx, ty, 0.0, 1.0],
        ]
    }

    pub fn px_to_map(&self, px_position: Point2d, map_size: Size) -> Point2d {
        Point2d::new(
            self.position.x() + (px_position.x() - map_size.width as f64 / 2.0) * self.resolution,
            self.position.y() + (map_size.height as f64 / 2.0 - px_position.y()) * self.resolution,
        )
    }

    pub fn translate_by_pixels(&self, delta: Point2d) -> Self {
        let position = Point2d::new(
            self.position.x() + delta.x() * self.resolution,
            self.position.y() - delta.y() * self.resolution,
        );
        Self {
            position,
            resolution: self.resolution,
        }
    }

    pub(crate) fn translate(&self, delta: Vec2d<f64>) -> MapView {
        Self {
            position: self.position - delta,
            resolution: self.resolution,
        }
    }

    pub(crate) fn zoom(&self, zoom: f64, base_point: Point2d) -> Self {
        let resolution = self.resolution * zoom;
        let new_position = base_point.add((self.position - base_point) * zoom);
        Self {
            position: new_position,
            resolution,
        }
    }

    pub(crate) fn interpolate(&self, target: MapView, k: f64) -> Self {
        Self {
            position: self.position + (target.position - self.position) * k,
            resolution: self.resolution + (target.resolution - self.resolution) * k,
        }
    }
}
