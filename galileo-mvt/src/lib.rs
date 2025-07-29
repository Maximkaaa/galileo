use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::iter::Enumerate;

use bytes::Buf;
use contour::MvtMultiPolygon;
pub use contour::{MvtContours, MvtPolygon};
use galileo_types::cartesian::{CartesianPoint2d, Point2};
use geozero::mvt::tile::GeomType;
use geozero::mvt::{Message as GeozeroMessage, Tile};
use serde::{Deserialize, Serialize};
use strfmt::DisplayStr;

use crate::error::GalileoMvtError;

mod contour;
pub mod error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MvtTile {
    pub layers: Vec<MvtLayer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MvtLayer {
    pub name: String,
    pub features: Vec<MvtFeature>,
    pub properties: Vec<String>,
    pub size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MvtFeature {
    pub id: Option<u64>,
    pub properties: HashMap<String, MvtValue>,
    pub geometry: MvtGeometry,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MvtValue {
    String(String),
    Float(f32),
    Double(f64),
    // For both Int and Sint variants of protobuf values
    Int64(i64),
    Uint64(u64),
    Bool(bool),
    Unknown,
}

impl Display for MvtValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MvtValue::String(v) => write!(f, "{v}"),
            MvtValue::Float(v) => write!(f, "{v}"),
            MvtValue::Double(v) => write!(f, "{v}"),
            MvtValue::Int64(v) => write!(f, "{v}"),
            MvtValue::Uint64(v) => write!(f, "{v}"),
            MvtValue::Bool(v) => write!(f, "{v}"),
            MvtValue::Unknown => write!(f, "<NONE>"),
        }
    }
}

impl DisplayStr for MvtValue {
    fn display_str(&self, f: &mut strfmt::Formatter) -> strfmt::Result<()> {
        f.str(&self.to_string())?;
        Ok(())
    }
}

impl MvtValue {
    pub fn eq_str(&self, str_value: &str) -> bool {
        match &self {
            MvtValue::String(s) => s == str_value,
            MvtValue::Float(v) => str_value.parse::<f32>() == Ok(*v),
            MvtValue::Double(v) => str_value.parse::<f64>() == Ok(*v),
            MvtValue::Int64(v) => str_value.parse::<i64>() == Ok(*v),
            MvtValue::Uint64(v) => str_value.parse::<u64>() == Ok(*v),
            MvtValue::Bool(v) => str_value.parse::<bool>() == Ok(*v),
            MvtValue::Unknown => false,
        }
    }
}

pub type Point = Point2<f32>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MvtGeometry {
    Point(Vec<Point>),
    LineString(MvtContours),
    Polygon(MvtMultiPolygon),
}

impl MvtTile {
    pub fn decode<B>(buffer: B, skip_recoverable_errors: bool) -> Result<MvtTile, GalileoMvtError>
    where
        B: Buf,
    {
        let pb = Tile::decode(buffer);

        if let Err(e) = pb {
            return Err(GalileoMvtError::Proto(e.to_string()));
        }

        let pb = pb.unwrap();

        let mut layers = vec![];
        for layer in pb.layers.into_iter() {
            match MvtLayer::decode(layer, skip_recoverable_errors) {
                Ok(v) => layers.push(v),
                Err(e) => {
                    if skip_recoverable_errors {
                        log::warn!("{e:?}");
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        let tile = MvtTile { layers };

        if tile.layers.is_empty() {
            return Err(GalileoMvtError::Generic(
                "Tile does not contain any valid layers".into(),
            ));
        }

        Ok(tile)
    }
}

impl MvtLayer {
    fn decode(
        pb_layer: geozero::mvt::tile::Layer,
        skip_recoverable_errors: bool,
    ) -> Result<Self, GalileoMvtError> {
        let geozero::mvt::tile::Layer {
            name,
            keys,
            values,
            features,
            version,
            extent,
        } = pb_layer;
        if version != 2 {
            return Err(GalileoMvtError::Generic(format!(
                "Invalid version: {version}"
            )));
        }

        let mut mvt_values = Vec::with_capacity(values.len());
        for value in values {
            match MvtValue::decode(value) {
                Ok(v) => mvt_values.push(v),
                Err(e) => {
                    if skip_recoverable_errors {
                        log::warn!("{e:?}");
                        mvt_values.push(MvtValue::Unknown);
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        let mut mvt_features = Vec::with_capacity(features.len());
        for feature in features {
            match MvtFeature::decode(feature, extent.unwrap_or(4096), &keys, &mvt_values) {
                Ok(v) => mvt_features.push(v),
                Err(e) => {
                    if skip_recoverable_errors {
                        log::warn!("{e:?}");
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Ok(MvtLayer {
            name,
            properties: keys,
            features: mvt_features,
            size: pb_layer.extent.unwrap_or(4096),
        })
    }
}

impl MvtValue {
    fn decode(pb_value: geozero::mvt::tile::Value) -> Result<MvtValue, GalileoMvtError> {
        let mut present_types = 0;
        let mut value = MvtValue::Unknown;

        if let Some(v) = pb_value.string_value {
            value = MvtValue::String(v);
            present_types += 1;
        }

        if let Some(v) = pb_value.float_value {
            value = MvtValue::Float(v);
            present_types += 1;
        }

        if let Some(v) = pb_value.double_value {
            value = MvtValue::Double(v);
            present_types += 1;
        }

        if let Some(v) = pb_value.int_value {
            value = MvtValue::Int64(v);
            present_types += 1;
        }

        if let Some(v) = pb_value.uint_value {
            value = MvtValue::Uint64(v);
            present_types += 1;
        }

        if let Some(v) = pb_value.sint_value {
            value = MvtValue::Int64(v);
            present_types += 1;
        }

        if let Some(v) = pb_value.bool_value {
            value = MvtValue::Bool(v);
            present_types += 1;
        }

        if present_types == 0 {
            Err(GalileoMvtError::Generic("No valid value present".into()))
        } else if present_types > 1 {
            Err(GalileoMvtError::Generic(
                "More than one value present".into(),
            ))
        } else {
            Ok(value)
        }
    }
}

pub fn number_to_geomtype(number: i32) -> GeomType {
    match number {
        1 => GeomType::Point,
        2 => GeomType::Linestring,
        3 => GeomType::Polygon,
        _ => GeomType::Unknown,
    }
}

pub fn opt_number_to_geomtype(number: Option<i32>) -> GeomType {
    match number {
        Some(number) => number_to_geomtype(number),
        None => GeomType::Unknown,
    }
}

impl MvtFeature {
    fn decode(
        pb_feature: geozero::mvt::tile::Feature,
        extent: u32,
        keys: &[String],
        values: &[MvtValue],
    ) -> Result<MvtFeature, GalileoMvtError> {
        let geozero::mvt::tile::Feature {
            id,
            tags,
            r#type,
            geometry,
        } = pb_feature;
        let pb_type = opt_number_to_geomtype(r#type);
        let properties = Self::decode_properties(tags, keys, values)?;
        let geometry = Self::decode_geometry(pb_type, geometry, extent)?;

        Ok(MvtFeature {
            id,
            properties,
            geometry,
        })
    }
    fn decode_properties(
        tags: Vec<u32>,
        keys: &[String],
        values: &[MvtValue],
    ) -> Result<HashMap<String, MvtValue>, GalileoMvtError> {
        let mut properties = HashMap::new();
        if !tags.len().is_multiple_of(2) {
            return Err(GalileoMvtError::Generic(
                "Invalid number of tags in feature".into(),
            ));
        }

        for tag_pair in tags.chunks(2) {
            let key = keys
                .get(tag_pair[0] as usize)
                .ok_or(GalileoMvtError::Generic("Invalid tag key".into()))?;
            let value = values
                .get(tag_pair[1] as usize)
                .ok_or(GalileoMvtError::Generic("Invalid tag value".into()))?;

            properties.insert(key.clone(), value.clone());
        }

        Ok(properties)
    }

    fn decode_geometry(
        geom_type: GeomType,
        commands: Vec<u32>,
        extent: u32,
    ) -> Result<MvtGeometry, GalileoMvtError> {
        Ok(match geom_type {
            GeomType::Unknown => {
                return Err(GalileoMvtError::Generic("Unknown geometry type".into()))
            }
            GeomType::Point => MvtGeometry::Point(Self::decode_point(commands, extent)?),
            GeomType::Linestring => MvtGeometry::LineString(MvtContours::new(commands, extent)?),
            GeomType::Polygon => MvtGeometry::Polygon(MvtMultiPolygon::new(commands, extent)?),
        })
    }

    fn decode_point(commands: Vec<u32>, extent: u32) -> Result<Vec<Point>, GalileoMvtError> {
        let mut points = Vec::with_capacity(commands.len() / 2);
        for command in Self::decode_commands(&commands, extent) {
            match command? {
                MvtGeomCommand::MoveTo(p) => points.push(p),
                _ => {
                    return Err(GalileoMvtError::Generic(
                        "Point geometry cannot have {:?} command".into(),
                    ))
                }
            }
        }

        Ok(points)
    }

    fn decode_commands(
        commands: &[u32],
        extent: u32,
    ) -> impl Iterator<Item = Result<MvtGeomCommand, GalileoMvtError>> + use<'_> {
        CommandIterator::new(commands.iter(), extent).map(|res| res.map(|(command, _)| command))
    }
}

struct CommandIterator<'a, T: Iterator<Item = &'a u32>> {
    inner: Enumerate<T>,
    extent: u32,
    current_command: Option<(u32, u32, usize)>,
    can_continue: bool,
    cursor: Point,
}

impl<'a, T: Iterator<Item = &'a u32>> CommandIterator<'a, T> {
    fn new(inner: T, extent: u32) -> Self {
        Self {
            inner: inner.enumerate(),
            extent,
            current_command: None,
            can_continue: true,
            cursor: Point::default(),
        }
    }

    fn read_move_to(&mut self) -> Result<MvtGeomCommand, GalileoMvtError> {
        self.cursor = self.read_point()?;
        Ok(MvtGeomCommand::MoveTo(self.cursor))
    }

    fn read_line_to(&mut self) -> Result<MvtGeomCommand, GalileoMvtError> {
        self.cursor = self.read_point()?;
        Ok(MvtGeomCommand::LineTo(self.cursor))
    }

    fn read_point(&mut self) -> Result<Point, GalileoMvtError> {
        let vals = self.read_vals::<2>()?;
        Ok(Point::new(
            self.decode_sint_coord(vals[0]) + self.cursor.x(),
            self.decode_sint_coord(vals[1]) + self.cursor.y(),
        ))
    }

    fn decode_sint_coord(&mut self, val: u32) -> f32 {
        sint_to_int(val) as f32 / self.extent as f32
    }

    fn read_vals<const COUNT: usize>(&mut self) -> Result<[u32; COUNT], GalileoMvtError> {
        let mut result = [0; COUNT];
        for val in result.iter_mut() {
            *val = match self.inner.next() {
                Some((_, v)) => *v,
                None => {
                    return Err(GalileoMvtError::Generic(
                        "Expected value to be present, but found end of data".into(),
                    ));
                }
            };
        }

        Ok(result)
    }
}

fn sint_to_int(sint: u32) -> i32 {
    if sint == u32::MAX {
        // Edge case. Operation below will overflow with this value.
        return i32::MIN;
    }

    match sint & 1 {
        0 => (sint >> 1) as i32,
        1 => -(((sint >> 1) + 1) as i32),
        _ => unreachable!(),
    }
}

impl<'a, T: Iterator<Item = &'a u32>> Iterator for CommandIterator<'a, T> {
    type Item = Result<(MvtGeomCommand, usize), GalileoMvtError>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.can_continue {
            return None;
        }

        let (command_id, command_count, index) = match self.current_command {
            Some((id, count, index)) => (id, count, index),
            None => {
                let (index, command_integer) = self.inner.next()?;
                let command_id = command_integer & 0x7;
                let command_count = command_integer >> 3;

                (command_id, command_count, index)
            }
        };

        self.current_command = match command_count {
            0 => {
                self.can_continue = false;
                return Some(Err(GalileoMvtError::Generic(
                    "Command count cannot be 0".into(),
                )));
            }
            1 => None,
            v => Some((command_id, v - 1, index)),
        };

        Some(match command_id {
            1 => self.read_move_to().map(|command| (command, index)),
            2 => self.read_line_to().map(|command| (command, index)),
            7 => {
                if command_count != 1 {
                    self.can_continue = false;
                    Err(GalileoMvtError::Generic(format!(
                        "ClosePath command must have count 0, but has {command_count}"
                    )))
                } else {
                    Ok((MvtGeomCommand::ClosePath, index))
                }
            }
            _ => {
                self.can_continue = false;
                Err(GalileoMvtError::Generic(format!(
                    "Unknown command id {command_id}"
                )))
            }
        })
    }
}

enum MvtGeomCommand {
    MoveTo(Point),
    LineTo(Point),
    ClosePath,
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use galileo_types::{Contour, MultiContour, MultiPolygon, Polygon};

    use super::*;

    #[test]
    fn sint_to_int_test() {
        assert_eq!(sint_to_int(0), 0);
        assert_eq!(sint_to_int(1), -1);
        assert_eq!(sint_to_int(2), 1);
        assert_eq!(sint_to_int(3), -2);
        assert_eq!(sint_to_int(0xfffffffe), 0x7fffffff);
        assert_eq!(sint_to_int(0xffffffff), i32::MIN);
    }

    #[test]
    fn test_protobuf() {
        let vt = include_bytes!("../test-data/vt.mvt");
        let tile = MvtTile::decode(&mut Cursor::new(&vt), false).unwrap();

        let layer = tile.layers.iter().find(|l| l.name == "boundary").unwrap();

        let feature205 = layer.features.iter().find(|f| f.id == Some(205)).unwrap();
        let MvtGeometry::LineString(contours) = &feature205.geometry else {
            panic!("invalid geometry type");
        };
        assert_eq!(contours.contours().count(), 1);
        assert_eq!(contours.contours().next().unwrap().iter_points().count(), 2);

        let feature681247437 = layer
            .features
            .iter()
            .find(|f| f.id == Some(681247437))
            .unwrap();
        let MvtGeometry::LineString(contours) = &feature681247437.geometry else {
            panic!("invalid geometry type");
        };
        assert_eq!(contours.contours().count(), 461);
        let points = contours
            .contours()
            .fold(0, |acc, c| acc + c.iter_points().count());
        assert_eq!(points, 6608);

        let layer = tile.layers.iter().find(|l| l.name == "water").unwrap();
        let feature342914 = layer
            .features
            .iter()
            .find(|f| f.id == Some(342914))
            .unwrap();
        let MvtGeometry::Polygon(polygons) = &feature342914.geometry else {
            panic!("invalid geometry type");
        };
        assert_eq!(polygons.polygons().count(), 4);
        let points = polygons
            .polygons()
            .flat_map(|p| p.iter_contours())
            .fold((0, 0), |acc, c| {
                (acc.0 + 1, acc.1 + c.iter_points_closing().count())
            });
        assert_eq!(points, (37, 1092));
    }
}
