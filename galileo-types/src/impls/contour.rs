use serde::{Deserialize, Serialize};
use std::ops::{Index, IndexMut};

use crate::geo::Projection;
use crate::geometry_type::{ContourGeometryType, GeometryType};

/// Simple [`crate::Contour`] implementation.
#[derive(Debug, Default, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Deserialize, Serialize)]
pub struct Contour<Point> {
    points: Vec<Point>,
    is_closed: bool,
}

impl<Point> std::ops::Deref for Contour<Point> {
    type Target = Vec<Point>;

    fn deref(&self) -> &Self::Target {
        &self.points
    }
}
impl<Point> std::ops::DerefMut for Contour<Point> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.points
    }
}

impl<Point> Contour<Point> {
    /// Creates a new contour.
    pub fn new(points: Vec<Point>, is_closed: bool) -> Self {
        Self { points, is_closed }
    }

    /// Creates a new open contour.
    pub fn open(points: Vec<Point>) -> Self {
        Self {
            points,
            is_closed: false,
        }
    }

    /// Creates a new closed contour.
    pub fn closed(points: Vec<Point>) -> Self {
        Self {
            points,
            is_closed: true,
        }
    }

    /// Converts self into a `ClosedContour` instance if the contour is closed, or returns `None` if the contour is
    /// open.
    pub fn into_closed(self) -> Option<ClosedContour<Point>> {
        if self.is_closed {
            Some(ClosedContour {
                points: self.points,
            })
        } else {
            None
        }
    }

    /// Projects all the points of the contour with the given projection.
    pub fn project_points<P, Proj>(&self, projection: &Proj) -> Option<Contour<P>>
    where
        Proj: Projection<InPoint = Point, OutPoint = P> + ?Sized,
    {
        let points = self
            .points
            .iter()
            .map(|p| projection.project(p))
            .collect::<Option<Vec<P>>>()?;
        Some(Contour {
            points,
            is_closed: self.is_closed,
        })
    }
}

/// Closed contour implementation.
#[derive(Debug, Clone, Default, PartialEq, PartialOrd, Eq, Ord, Hash, Deserialize, Serialize)]
pub struct ClosedContour<Point> {
    /// Points of the contour.
    pub points: Vec<Point>,
}

impl<Point> ClosedContour<Point> {
    /// Creates a new closed contour.
    pub fn new(points: Vec<Point>) -> Self {
        Self { points }
    }

    /// Projects all the points of the contour with the given projection.
    pub fn project_points<P, Proj>(&self, projection: &Proj) -> Option<ClosedContour<P>>
    where
        Proj: Projection<InPoint = Point, OutPoint = P> + ?Sized,
    {
        let points = self
            .points
            .iter()
            .map(|p| projection.project(p))
            .collect::<Option<Vec<P>>>()?;
        Some(ClosedContour { points })
    }
}

impl<P> From<ClosedContour<P>> for Contour<P> {
    fn from(value: ClosedContour<P>) -> Self {
        Self {
            points: value.points,
            is_closed: true,
        }
    }
}

impl<P> crate::contour::ClosedContour for ClosedContour<P> {
    type Point = P;

    fn iter_points(&self) -> impl Iterator<Item = &'_ P> {
        self.points.iter()
    }
}

impl<P> crate::contour::Contour for Contour<P> {
    type Point = P;

    fn is_closed(&self) -> bool {
        self.is_closed
    }

    fn iter_points(&self) -> impl Iterator<Item = &P> {
        self.points.iter()
    }
}

impl<P: GeometryType> GeometryType for Contour<P> {
    type Type = ContourGeometryType;
    type Space = P::Space;
}

impl<P: GeometryType> GeometryType for ClosedContour<P> {
    type Type = ContourGeometryType;
    type Space = P::Space;
}
