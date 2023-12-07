use galileo_types::{ClosedContour, Point2d};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct BoundingBox {
    x_min: f64,
    y_min: f64,
    x_max: f64,
    y_max: f64,
}

impl BoundingBox {
    pub fn new(x_min: f64, y_min: f64, x_max: f64, y_max: f64) -> Self {
        Self {
            x_min,
            y_min,
            x_max,
            y_max,
        }
    }

    pub fn x_min(&self) -> f64 {
        self.x_min
    }

    pub fn x_max(&self) -> f64 {
        self.x_max
    }

    pub fn y_min(&self) -> f64 {
        self.y_min
    }

    pub fn y_max(&self) -> f64 {
        self.y_max
    }

    pub fn width(&self) -> f64 {
        self.x_max - self.x_min
    }

    pub fn height(&self) -> f64 {
        self.y_max - self.y_min
    }

    pub fn p1(&self) -> Point2d {
        Point2d::new(self.x_min, self.y_min)
    }

    pub fn intersect(&self, other: BoundingBox) -> Self {
        Self::new(
            self.x_min.max(other.x_min),
            self.y_min.max(other.y_min),
            self.x_max.min(other.x_max),
            self.y_max.min(other.y_max),
        )
    }

    pub fn into_contour(&self) -> ClosedContour<Point2d> {
        ClosedContour::new(vec![
            Point2d::new(self.x_min, self.y_min),
            Point2d::new(self.x_min, self.y_max),
            Point2d::new(self.x_max, self.y_max),
            Point2d::new(self.x_max, self.y_min),
        ])
    }

    pub fn shrink(&self, amount: f64) -> BoundingBox {
        Self {
            x_min: self.x_min + amount,
            x_max: self.x_max - amount,
            y_min: self.y_min + amount,
            y_max: self.y_max - amount,
        }
    }
}
