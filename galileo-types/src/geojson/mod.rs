use crate::geo::Projection;
use crate::geojson::point::GeoJsonPoint;
use crate::geometry::{Geom, Geometry};
use crate::impls::Contour;
use crate::impls::MultiContour;
use crate::impls::MultiPoint;
use crate::impls::MultiPolygon;
use crate::impls::Polygon;
use geojson::{LineStringType, PolygonType, Position, Value};

mod point;

impl Geometry for geojson::Geometry {
    type Point = GeoJsonPoint;

    fn project<Proj>(&self, projection: &Proj) -> Option<Geom<Proj::OutPoint>>
    where
        Proj: Projection<InPoint = Self::Point> + ?Sized,
    {
        match &self.value {
            Value::Point(p) => GeoJsonPoint::try_from(p.clone()).ok()?.project(projection),
            Value::MultiPoint(points) => convert_multi_point(points)?.project(projection),
            Value::LineString(points) => convert_contour(points)?.project(projection),
            Value::MultiLineString(lines) => convert_multi_contour(lines)?.project(projection),
            Value::Polygon(polygon) => convert_polygon(polygon)?.project(projection),
            Value::MultiPolygon(mp) => convert_multi_polygon(mp)?.project(projection),
            Value::GeometryCollection(_) => todo!(),
        }
    }
}

fn convert_contour(line_string: &LineStringType) -> Option<Contour<GeoJsonPoint>> {
    let is_closed = !line_string.is_empty() && line_string[0] == line_string[line_string.len() - 1];
    Some(Contour::new(
        line_string
            .iter()
            .map(|p| GeoJsonPoint::try_from(p.clone()).ok())
            .collect::<Option<Vec<_>>>()?,
        is_closed,
    ))
}

fn convert_multi_point(points: &[Position]) -> Option<MultiPoint<GeoJsonPoint>> {
    Some(MultiPoint::from(
        points
            .iter()
            .map(|p| GeoJsonPoint::try_from(p.clone()).ok())
            .collect::<Option<Vec<_>>>()?,
    ))
}

fn convert_multi_contour(lines: &[LineStringType]) -> Option<MultiContour<GeoJsonPoint>> {
    Some(MultiContour::from(
        lines
            .iter()
            .map(convert_contour)
            .collect::<Option<Vec<_>>>()?,
    ))
}

fn convert_polygon(polygon: &PolygonType) -> Option<Polygon<GeoJsonPoint>> {
    Some(Polygon::new(
        convert_contour(&polygon[0])?.into_closed()?,
        polygon[1..]
            .iter()
            .map(|p| convert_contour(p).and_then(|c| c.into_closed()))
            .collect::<Option<Vec<_>>>()?,
    ))
}

fn convert_multi_polygon(mp: &[PolygonType]) -> Option<MultiPolygon<GeoJsonPoint>> {
    Some(MultiPolygon::from(
        mp.iter().map(convert_polygon).collect::<Option<Vec<_>>>()?,
    ))
}
