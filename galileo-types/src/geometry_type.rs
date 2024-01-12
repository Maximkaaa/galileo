pub trait GeometryType {
    type Type;
    type Space;
}

pub struct PointGeometryType;

pub struct MultiPointGeometryType;

pub struct ContourGeometryType;

pub struct MultiContourGeometryType;

pub struct PolygonGeometryType;

pub struct MultiPolygonGeometryType;

pub struct GeoSpace2d;

pub struct CartesianSpace2d;

pub struct CartesianSpace3d;

pub struct AmbiguousSpace;
