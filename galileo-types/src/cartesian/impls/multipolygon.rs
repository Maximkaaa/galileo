use crate::cartesian::impls::polygon::Polygon;
use crate::cartesian::rect::Rect;
use crate::cartesian::traits::cartesian_point::CartesianPoint2d;
use crate::geo::traits::projection::Projection;
use crate::geometry::{CartesianGeometry2d, Geom, Geometry};
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

impl<P> Geometry for MultiPolygon<P> {
    type Point = P;

    fn project<Proj: Projection<InPoint = Self::Point> + ?Sized>(
        &self,
        projection: &Proj,
    ) -> Option<Geom<Proj::OutPoint>> {
        Some(Geom::MultiPolygon(MultiPolygon {
            parts: self
                .parts
                .iter()
                .map(|p| {
                    p.project(projection).and_then(|p| match p {
                        Geom::Polygon(v) => Some(v),
                        _ => None,
                    })
                })
                .collect::<Option<Vec<Polygon<Proj::OutPoint>>>>()?,
        }))
    }
}

impl<P> CartesianGeometry2d<P> for MultiPolygon<P>
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
