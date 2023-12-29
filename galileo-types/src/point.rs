pub trait Point {
    type Type: PointType;
    type Num;
    const DIMENSIONS: usize;
}

pub trait PointType {}

pub struct CartesianPointType;
impl PointType for CartesianPointType {}

pub struct GeoPointType;
impl PointType for GeoPointType {}

pub trait PointHelper<T>: Point {}
