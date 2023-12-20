use crate::rect::Rect;
use crate::CartesianPoint2d;

pub trait Geometry {
    type Point: Point;
}

pub enum GeometryType {
    Point(),
    Line(),
}

pub trait CartesianGeometry {
    type Point: Point<Type = CartesianPointType>;

    fn bounding_rect(&self) -> Rect<<Self::Point as Point>::Num>;
    fn is_point_inside<P>(&self, point: &P, tolerance: <Self::Point as Point>::Num) -> bool
    where
        P: CartesianPoint2d<Num = <Self::Point as Point>::Num>;
}

impl<T: CartesianGeometry> Geometry for T {
    type Point = T::Point;
}

pub trait GeometryMarker {
    type Marker;
}

pub trait Point {
    type Type: PointType;
    type Num;
    const DIMENSIONS: usize;
}

pub trait PointType {}

pub struct CartesianPointType;
impl PointType for CartesianPointType {}

pub struct GeoPointType;
impl PointType for GeoPointType {}

pub trait PointHelper<T>: Point {}

pub trait GeometryHelper<Marker>: GeometryMarker {
    type Point: Point<Type = CartesianPointType>;

    fn __bounding_rect(&self) -> Rect<<Self::Point as Point>::Num>;
    fn __contains_point<P>(&self, point: &P, tolerance: <Self::Point as Point>::Num) -> bool
    where
        P: CartesianPoint2d<Num = <Self::Point as Point>::Num>;
}

impl<T: GeometryHelper<<Self as GeometryMarker>::Marker>> CartesianGeometry for T {
    type Point = T::Point;

    fn bounding_rect(&self) -> Rect<<<Self as Geometry>::Point as Point>::Num> {
        T::__bounding_rect(self)
    }

    fn is_point_inside<P>(
        &self,
        point: &P,
        tolerance: <<Self as Geometry>::Point as Point>::Num,
    ) -> bool
    where
        P: CartesianPoint2d<Num = <<Self as Geometry>::Point as Point>::Num>,
    {
        T::__contains_point(self, point, tolerance)
    }
}
