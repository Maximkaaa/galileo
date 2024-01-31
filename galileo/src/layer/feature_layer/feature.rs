use galileo_types::cartesian::impls::contour::Contour;
use galileo_types::cartesian::impls::multipolygon::MultiPolygon;
use galileo_types::cartesian::impls::point::{Point2d, Point3d};
use galileo_types::cartesian::impls::polygon::Polygon;
use galileo_types::disambig::Disambig;
use galileo_types::geo::impls::point::GeoPoint2d;
use galileo_types::geometry::Geometry;
use galileo_types::geometry_type::GeometryType;
use galileo_types::impls::multi_contour::MultiContour;

pub trait Feature {
    type Geom: Geometry;
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

impl_feature!(Point2d);
impl_feature!(Point3d);
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
