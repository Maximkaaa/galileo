use crate::cartesian::impls::contour::Contour;
use crate::cartesian::impls::multipolygon::MultiPolygon;
use crate::cartesian::impls::polygon::Polygon;
use crate::cartesian::rect::Rect;
use crate::cartesian::traits::cartesian_point::CartesianPoint2d;
use crate::geo::traits::projection::Projection;

pub enum Geom<P> {
    Point(P),
    Line(Contour<P>),
    Polygon(Polygon<P>),
    MultiPolygon(MultiPolygon<P>),
}

pub trait Geometry {
    type Point;
    fn project<P: Projection<InPoint = Self::Point> + ?Sized>(
        &self,
        projection: &P,
    ) -> Option<Geom<P::OutPoint>>;
}

pub trait CartesianGeometry2d<P: CartesianPoint2d>: Geometry<Point = P> {
    fn is_point_inside<Other: CartesianPoint2d<Num = P::Num>>(
        &self,
        point: &Other,
        tolerance: P::Num,
    ) -> bool;
    fn bounding_rectangle(&self) -> Rect<P::Num>;
}

impl<P> From<P> for Geom<P> {
    fn from(value: P) -> Self {
        Self::Point(value)
    }
}

impl<P> From<Contour<P>> for Geom<P> {
    fn from(value: Contour<P>) -> Self {
        Self::Line(value)
    }
}

impl<P> From<Polygon<P>> for Geom<P> {
    fn from(value: Polygon<P>) -> Self {
        Self::Polygon(value)
    }
}

impl<P> From<MultiPolygon<P>> for Geom<P> {
    fn from(value: MultiPolygon<P>) -> Self {
        Self::MultiPolygon(value)
    }
}
