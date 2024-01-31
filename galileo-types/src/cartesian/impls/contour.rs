use crate::geo::traits::projection::Projection;
use crate::geometry_type::{ContourGeometryType, GeometryType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contour<Point> {
    pub points: Vec<Point>,
    pub is_closed: bool,
}

impl<Point> Contour<Point> {
    pub fn new(points: Vec<Point>, is_closed: bool) -> Self {
        Self { points, is_closed }
    }

    pub fn open(points: Vec<Point>) -> Self {
        Self {
            points,
            is_closed: false,
        }
    }

    pub fn closed(points: Vec<Point>) -> Self {
        Self {
            points,
            is_closed: true,
        }
    }

    pub fn into_closed(self) -> Option<ClosedContour<Point>> {
        if self.is_closed {
            Some(ClosedContour {
                points: self.points,
            })
        } else {
            None
        }
    }

    pub fn project_points<P, Proj>(&self, projection: &Proj) -> Option<Contour<P>>
    where
        Proj: Projection<InPoint = Point, OutPoint = P>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClosedContour<Point> {
    pub points: Vec<Point>,
}

impl<Point> ClosedContour<Point> {
    pub fn new(points: Vec<Point>) -> Self {
        Self { points }
    }

    pub fn project_points<P, Proj>(&self, projection: &Proj) -> Option<ClosedContour<P>>
    where
        Proj: Projection<InPoint = Point, OutPoint = P>,
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

// impl<'a, P> crate::traits::contour::ClosedContour<'a> for ClosedContour<P> {
//     fn iter_points(&'a self) -> Self::PointIterator {
//         todo!()
//     }
// }

impl<P> crate::contour::Contour for Contour<P> {
    type Point = P;

    fn is_closed(&self) -> bool {
        self.is_closed
    }

    fn iter_points(&self) -> impl Iterator<Item = &P> {
        self.points.iter()
    }
}

// impl<'a, Num: Float, P: CartesianPoint2d<Num = Num> + 'a> CartesianContour<'a, Num, P>
//     for Contour<P>
// {
// }

impl<P: GeometryType> GeometryType for Contour<P> {
    type Type = ContourGeometryType;
    type Space = P::Space;
}

impl<P: GeometryType> GeometryType for ClosedContour<P> {
    type Type = ContourGeometryType;
    type Space = P::Space;
}
