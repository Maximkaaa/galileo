use crate::bounding_rect::BoundingRect;
use crate::CartesianPoint2d;

pub trait Geometry {
    type Num: num_traits::Num + Copy + PartialOrd;
    fn bounding_rect(&self) -> BoundingRect<Self::Num>;
    fn is_point_inside<P>(&self, point: &P, tolerance: Self::Num) -> bool
    where
        P: CartesianPoint2d<Num = Self::Num>;
}

pub trait GeometryMarker {
    type Marker;
}

pub trait GeometryHelper<Marker>: GeometryMarker {
    type Num: num_traits::Num + Copy + PartialOrd;
    fn __bounding_rect(&self) -> BoundingRect<Self::Num>;
    fn __contains_point<P>(&self, point: &P, tolerance: Self::Num) -> bool
    where
        P: CartesianPoint2d<Num = Self::Num>;
}

impl<T: GeometryHelper<<Self as GeometryMarker>::Marker>> Geometry for T {
    type Num = T::Num;

    fn bounding_rect(&self) -> BoundingRect<Self::Num> {
        T::__bounding_rect(&self)
    }

    fn is_point_inside<P>(&self, point: &P, tolerance: Self::Num) -> bool
    where
        P: CartesianPoint2d<Num = Self::Num>,
    {
        T::__contains_point(&self, point, tolerance)
    }
}
