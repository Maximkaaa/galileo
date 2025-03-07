use galileo_types::cartesian::{Point2, Point3};
use galileo_types::geo::impls::GeoPoint2d;
use galileo_types::geometry::Geometry;
use galileo_types::geometry_type::GeometryType;
use galileo_types::impls::{Contour, MultiContour, MultiPolygon, Polygon};
use galileo_types::Disambig;

/// A feature is an arbitrary geographic object.
pub trait Feature {
    /// Type of the geometry the feature returns.
    type Geom: Geometry;
    /// Returns the geometry of the feature.
    fn geometry(&self) -> &Self::Geom;
}

macro_rules! impl_feature {
    ($geom:ident) => {
        impl Feature for $geom {
            type Geom = Self;
            fn geometry(&self) -> &Self::Geom {
                self
            }
        }
    };

    ($geom:ident, $generic:ident) => {
        impl<$generic: ::galileo_types::geometry_type::GeometryType> Feature for $geom<$generic> {
            type Geom = Self;
            fn geometry(&self) -> &Self::Geom {
                self
            }
        }
    };
}

impl_feature!(Point2);
impl_feature!(Point3);
impl_feature!(GeoPoint2d);
impl_feature!(Contour, Point);
impl_feature!(MultiContour, Point);
impl_feature!(Polygon, Point);
impl_feature!(MultiPolygon, Point);

impl<T: GeometryType, Space> Feature for Disambig<T, Space>
where
    Disambig<T, Space>: Geometry,
{
    type Geom = Self;

    fn geometry(&self) -> &Self::Geom {
        self
    }
}

#[cfg(feature = "geojson")]
mod geojson;
