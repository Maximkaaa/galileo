use crate::rect::Rect;
use crate::CartesianPoint2d;

pub trait Geometry {
    type Point: Point;
}

pub trait CartesianGeometry {
    type Num: num_traits::Num + Copy + PartialOrd;
    fn bounding_rect(&self) -> Rect<Self::Num>;
    fn is_point_inside<P>(&self, point: &P, tolerance: Self::Num) -> bool
    where
        P: CartesianPoint2d<Num = Self::Num>;
}

pub trait GeometryMarker {
    type Marker;
}

pub trait Point {
    type Type: PointType;
    type Num;
}

pub trait PointType {}

pub struct CartesianPointType;
impl PointType for CartesianPointType {}

pub struct GeoPointType;
impl PointType for GeoPointType {}

pub trait GeometryHelper<Marker>: GeometryMarker {
    type Num: num_traits::Num + Copy + PartialOrd;
    fn __bounding_rect(&self) -> Rect<Self::Num>;
    fn __contains_point<P>(&self, point: &P, tolerance: Self::Num) -> bool
    where
        P: CartesianPoint2d<Num = Self::Num>;
}

impl<T: GeometryHelper<<Self as GeometryMarker>::Marker>> CartesianGeometry for T {
    type Num = T::Num;

    fn bounding_rect(&self) -> Rect<Self::Num> {
        T::__bounding_rect(self)
    }

    fn is_point_inside<P>(&self, point: &P, tolerance: Self::Num) -> bool
    where
        P: CartesianPoint2d<Num = Self::Num>,
    {
        T::__contains_point(self, point, tolerance)
    }
}
