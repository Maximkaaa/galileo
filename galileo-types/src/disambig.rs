use std::marker::PhantomData;

use crate::cartesian::{CartesianPoint2d, NewCartesianPoint2d};
use crate::contour::Contour;
use crate::geo::{GeoPoint, NewGeoPoint};
use crate::geometry_type::{AmbiguousSpace, CartesianSpace2d, GeoSpace2d, GeometryType};
use crate::multi_contour::MultiContour;
use crate::multi_point::MultiPoint;
use crate::multi_polygon::MultiPolygon;
use crate::polygon::Polygon;

/// Wrapper type that disambiguates coordinate space for generic geometries.
///
/// See [`Disambiguate`] trait documentation for details.
pub struct Disambig<T, Space> {
    inner: T,
    space: PhantomData<Space>,
}

impl<T, Space> Disambig<T, Space> {
    /// Creates a new instance.
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            space: Default::default(),
        }
    }
}

impl<T: Clone, Space> Clone for Disambig<T, Space> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            space: Default::default(),
        }
    }
}

impl<T: GeometryType, Space> GeometryType for Disambig<T, Space> {
    type Type = T::Type;
    type Space = Space;
}

impl<T: CartesianPoint2d> CartesianPoint2d for Disambig<T, CartesianSpace2d> {
    type Num = T::Num;

    fn x(&self) -> Self::Num {
        self.inner.x()
    }

    fn y(&self) -> Self::Num {
        self.inner.y()
    }
}

impl<T: GeoPoint> GeoPoint for Disambig<T, GeoSpace2d> {
    type Num = T::Num;

    fn lat(&self) -> Self::Num {
        self.inner.lat()
    }

    fn lon(&self) -> Self::Num {
        self.inner.lon()
    }
}

impl<T: NewCartesianPoint2d> NewCartesianPoint2d for Disambig<T, CartesianSpace2d> {
    fn new(x: f64, y: f64) -> Self {
        Self::new(T::new(x, y))
    }
}

impl<T: NewGeoPoint> NewGeoPoint for Disambig<T, GeoSpace2d> {
    fn latlon(lat: f64, lon: f64) -> Self {
        Self::new(T::latlon(lat, lon))
    }
}

impl<T: Contour, Space> Contour for Disambig<T, Space> {
    type Point = T::Point;

    fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }

    fn iter_points(&self) -> impl Iterator<Item = Self::Point> {
        self.inner.iter_points()
    }
}

impl<T: Polygon, Space> Polygon for Disambig<T, Space> {
    type Contour = T::Contour;

    fn outer_contour(&self) -> &Self::Contour {
        self.inner.outer_contour()
    }

    fn inner_contours(&self) -> impl Iterator<Item = &'_ Self::Contour> {
        self.inner.inner_contours()
    }
}

impl<T: MultiPoint, Space> MultiPoint for Disambig<T, Space> {
    type Point = T::Point;

    fn iter_points(&self) -> impl Iterator<Item = Self::Point> {
        self.inner.iter_points()
    }
}

impl<T: MultiContour, Space> MultiContour for Disambig<T, Space> {
    type Contour = T::Contour;

    fn contours(&self) -> impl Iterator<Item = &Self::Contour> {
        self.inner.contours()
    }
}

impl<T: MultiPolygon, Space> MultiPolygon for Disambig<T, Space> {
    type Polygon = T::Polygon;

    fn polygons(&self) -> impl Iterator<Item = &Self::Polygon> {
        self.inner.polygons()
    }
}

/// A trait used to convert a geometry with no specified coordinate space into one of the specific coordinate spaces.
/// This trait is auto-implemented for all types, that implement `GeometryType<Space = AmbiguousSpace>` trait.
pub trait Disambiguate {
    /// Specifies that the geometry is in geographic coordinates.
    fn to_geo2d(self) -> Disambig<Self, GeoSpace2d>
    where
        Self: Sized,
    {
        Disambig::new(self)
    }

    /// Specifies that the geometry is in cartesian coordinates.
    fn to_cartesian2d(self) -> Disambig<Self, CartesianSpace2d>
    where
        Self: Sized,
    {
        Disambig::new(self)
    }
}

impl<T: GeometryType<Space = AmbiguousSpace>> Disambiguate for T {}
