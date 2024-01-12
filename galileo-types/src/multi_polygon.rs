use crate::cartesian::impls::polygon::Polygon;
use crate::geo::traits::projection::Projection;
use crate::geometry::{Geom, Geometry, GeometrySpecialization};
use crate::geometry_type::{GeometryType, MultiPolygonGeometryType};

pub trait MultiPolygon {
    type Polygon;

    fn polygons(&self) -> impl Iterator<Item = &Self::Polygon>;
}

impl<Poly, Space> GeometrySpecialization<MultiPolygonGeometryType, Space> for Poly
where
    Poly: MultiPolygon + GeometryType<Type = MultiPolygonGeometryType, Space = Space>,
    Poly::Polygon: Geometry,
{
    type Point = <Poly::Polygon as Geometry>::Point;

    fn project<Proj>(&self, projection: &Proj) -> Option<Geom<Proj::OutPoint>>
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
