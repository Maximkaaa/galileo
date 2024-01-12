use crate::cartesian::impls::polygon::Polygon;
use crate::cartesian::rect::Rect;
use crate::cartesian::traits::cartesian_point::CartesianPoint2d;
use crate::geometry::CartesianGeometry2d;
use crate::geometry_type::{GeometryType, MultiPolygonGeometryType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MultiPolygon<P> {
    pub parts: Vec<Polygon<P>>,
}

impl<P> From<Vec<Polygon<P>>> for MultiPolygon<P> {
    fn from(parts: Vec<Polygon<P>>) -> Self {
        Self { parts }
    }
}

impl<P> MultiPolygon<P> {
    pub fn parts(&self) -> &[Polygon<P>] {
        &self.parts
    }
}

impl<P> crate::multi_polygon::MultiPolygon for MultiPolygon<P> {
    type Polygon = Polygon<P>;

    fn polygons(&self) -> impl Iterator<Item = &Self::Polygon> {
        self.parts.iter()
    }
}

impl<P: GeometryType> GeometryType for MultiPolygon<P> {
    type Type = MultiPolygonGeometryType;
    type Space = P::Space;
}

impl<P: GeometryType> CartesianGeometry2d<P> for MultiPolygon<P>
where
    P: CartesianPoint2d,
{
    fn is_point_inside<Other: CartesianPoint2d<Num = P::Num>>(
        &self,
        point: &Other,
        tolerance: P::Num,
    ) -> bool {
        self.parts
            .iter()
            .any(|p| p.is_point_inside(point, tolerance))
    }

    fn bounding_rectangle(&self) -> Rect<P::Num> {
        todo!()
    }
}
