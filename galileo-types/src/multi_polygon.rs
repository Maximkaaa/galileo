use crate::cartesian::CartesianPoint2d;
use crate::cartesian::Rect;
use crate::contour::Contour;
use crate::geo::Projection;
use crate::geometry::{
    CartesianGeometry2d, CartesianGeometry2dSpecialization, Geom, Geometry, GeometrySpecialization,
};
use crate::geometry_type::{CartesianSpace2d, GeometryType, MultiPolygonGeometryType};
use crate::impls::Polygon;

/// Geometry consisting of several polygons.
pub trait MultiPolygon {
    /// Polygon type.
    type Polygon: crate::polygon::Polygon;

    /// Iterates over polygons.
    fn polygons(&self) -> impl Iterator<Item = &Self::Polygon>;
}

impl<Poly, Space> GeometrySpecialization<MultiPolygonGeometryType, Space> for Poly
where
    Poly: MultiPolygon + GeometryType<Type = MultiPolygonGeometryType, Space = Space>,
    Poly::Polygon: Geometry,
{
    type Point = <Poly::Polygon as Geometry>::Point;

    fn project_spec<Proj>(&self, projection: &Proj) -> Option<Geom<Proj::OutPoint>>
    where
        Proj: Projection<InPoint = Self::Point> + ?Sized,
    {
        let polygons = self
            .polygons()
            .map(|c| {
                c.project(projection).and_then(|c| match c {
                    Geom::Polygon(polygon) => Some(polygon),
                    _ => None,
                })
            })
            .collect::<Option<Vec<Polygon<Proj::OutPoint>>>>()?;
        Some(Geom::MultiPolygon(polygons.into()))
    }
}

impl<P, Poly> CartesianGeometry2dSpecialization<P, MultiPolygonGeometryType> for Poly
where
    P: CartesianPoint2d,
    Poly: MultiPolygon
        + GeometryType<Type = MultiPolygonGeometryType, Space = CartesianSpace2d>
        + Geometry<Point = P>,
    Poly::Polygon: CartesianGeometry2d<P>,
    <Poly::Polygon as crate::polygon::Polygon>::Contour: Contour<Point = P>,
{
    fn is_point_inside_spec<Other: CartesianPoint2d<Num = P::Num>>(
        &self,
        point: &Other,
        tolerance: P::Num,
    ) -> bool {
        self.polygons().any(|p| p.is_point_inside(point, tolerance))
    }

    fn bounding_rectangle_spec(&self) -> Option<Rect<P::Num>> {
        self.polygons()
            .filter_map(|p| p.bounding_rectangle())
            .collect()
    }
}
