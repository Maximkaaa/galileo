use crate::error::GalileoMvtError;
use crate::vector_tile::tile::GeomType;
use bytes::Buf;
use galileo_types::traits::contour::{CartesianClosedContour, Winding};
use galileo_types::{CartesianPoint2d, ClosedContour, Contour, Point2d, Polygon};
use prost::Message;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

pub mod error;

mod vector_tile {
    // include!(concat!(env!("OUT_DIR"), "/vector_tile.rs"));

    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Tile {
        #[prost(message, repeated, tag = "3")]
        pub layers: ::prost::alloc::vec::Vec<tile::Layer>,
    }
    /// Nested message and enum types in `Tile`.
    pub mod tile {
        /// Variant type encoding
        /// The use of values is described in section 4.1 of the specification
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct Value {
            /// Exactly one of these values must be present in a valid message
            #[prost(string, optional, tag = "1")]
            pub string_value: ::core::option::Option<::prost::alloc::string::String>,
            #[prost(float, optional, tag = "2")]
            pub float_value: ::core::option::Option<f32>,
            #[prost(double, optional, tag = "3")]
            pub double_value: ::core::option::Option<f64>,
            #[prost(int64, optional, tag = "4")]
            pub int_value: ::core::option::Option<i64>,
            #[prost(uint64, optional, tag = "5")]
            pub uint_value: ::core::option::Option<u64>,
            #[prost(sint64, optional, tag = "6")]
            pub sint_value: ::core::option::Option<i64>,
            #[prost(bool, optional, tag = "7")]
            pub bool_value: ::core::option::Option<bool>,
        }
        /// Features are described in section 4.2 of the specification
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct Feature {
            #[prost(uint64, optional, tag = "1", default = "0")]
            pub id: ::core::option::Option<u64>,
            /// Tags of this feature are encoded as repeated pairs of
            /// integers.
            /// A detailed description of tags is located in sections
            /// 4.2 and 4.4 of the specification
            #[prost(uint32, repeated, tag = "2")]
            pub tags: ::prost::alloc::vec::Vec<u32>,
            /// The type of geometry stored in this feature.
            #[prost(enumeration = "GeomType", optional, tag = "3", default = "Unknown")]
            pub r#type: ::core::option::Option<i32>,
            /// Contains a stream of commands and parameters (vertices).
            /// A detailed description on geometry encoding is located in
            /// section 4.3 of the specification.
            #[prost(uint32, repeated, tag = "4")]
            pub geometry: ::prost::alloc::vec::Vec<u32>,
        }
        /// Layers are described in section 4.1 of the specification
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct Layer {
            /// Any compliant implementation must first read the version
            /// number encoded in this message and choose the correct
            /// implementation for this version number before proceeding to
            /// decode other parts of this message.
            #[prost(uint32, required, tag = "15", default = "1")]
            pub version: u32,
            #[prost(string, required, tag = "1")]
            pub name: ::prost::alloc::string::String,
            /// The actual features in this tile.
            #[prost(message, repeated, tag = "2")]
            pub features: ::prost::alloc::vec::Vec<Feature>,
            /// Dictionary encoding for keys
            #[prost(string, repeated, tag = "3")]
            pub keys: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
            /// Dictionary encoding for values
            #[prost(message, repeated, tag = "4")]
            pub values: ::prost::alloc::vec::Vec<Value>,
            /// Although this is an "optional" field it is required by the specification.
            /// See <https://github.com/mapbox/vector-tile-spec/issues/47>
            #[prost(uint32, optional, tag = "5", default = "4096")]
            pub extent: ::core::option::Option<u32>,
        }
        /// GeomType is described in section 4.3.4 of the specification
        #[derive(
            Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration,
        )]
        #[repr(i32)]
        pub enum GeomType {
            Unknown = 0,
            Point = 1,
            Linestring = 2,
            Polygon = 3,
        }
        impl GeomType {
            /// String value of the enum field names used in the ProtoBuf definition.
            ///
            /// The values are not transformed in any way and thus are considered stable
            /// (if the ProtoBuf definition does not change) and safe for programmatic use.
            pub fn as_str_name(&self) -> &'static str {
                match self {
                    GeomType::Unknown => "UNKNOWN",
                    GeomType::Point => "POINT",
                    GeomType::Linestring => "LINESTRING",
                    GeomType::Polygon => "POLYGON",
                }
            }
            /// Creates an enum from field names used in the ProtoBuf definition.
            pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
                match value {
                    "UNKNOWN" => Some(Self::Unknown),
                    "POINT" => Some(Self::Point),
                    "LINESTRING" => Some(Self::Linestring),
                    "POLYGON" => Some(Self::Polygon),
                    _ => None,
                }
            }
        }
    }
}

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

pub type Point = Point2d<f32>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MvtGeometry {
    Point(Vec<Point>),
    LineString(Vec<Contour<Point>>),
    Polygon(Vec<Polygon<Point>>),
}

impl From<Option<i32>> for GeomType {
    fn from(value: Option<i32>) -> Self {
        match value {
            Some(1) => GeomType::Point,
            Some(2) => GeomType::Linestring,
            Some(3) => GeomType::Polygon,
            _ => GeomType::Unknown,
        }
    }
}

impl MvtTile {
    pub fn decode<B>(buffer: B, skip_recoverable_errors: bool) -> Result<MvtTile, GalileoMvtError>
    where
        B: Buf,
    {
        let pb = vector_tile::Tile::decode(buffer)?;

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
        pb_layer: vector_tile::tile::Layer,
        skip_recoverable_errors: bool,
    ) -> Result<Self, GalileoMvtError> {
        let vector_tile::tile::Layer {
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
    fn decode(pb_value: vector_tile::tile::Value) -> Result<MvtValue, GalileoMvtError> {
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

impl MvtFeature {
    fn decode(
        pb_feature: vector_tile::tile::Feature,
        extent: u32,
        keys: &[String],
        values: &[MvtValue],
    ) -> Result<MvtFeature, GalileoMvtError> {
        let vector_tile::tile::Feature {
            id,
            tags,
            r#type,
            geometry,
        } = pb_feature;
        let pb_type = r#type.into();
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
        if tags.len() % 2 != 0 {
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
            GeomType::Linestring => MvtGeometry::LineString(Self::decode_line(commands, extent)?),
            GeomType::Polygon => MvtGeometry::Polygon(Self::decode_polygon(commands, extent)?),
        })
    }

    fn decode_point(commands: Vec<u32>, extent: u32) -> Result<Vec<Point>, GalileoMvtError> {
        let mut points = Vec::with_capacity(commands.len() / 2);
        for command in Self::decode_commands(commands, extent) {
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

    fn decode_line(
        commands: Vec<u32>,
        extent: u32,
    ) -> Result<Vec<Contour<Point>>, GalileoMvtError> {
        let mut contours = Vec::with_capacity(64);
        let mut current_contour: Option<Vec<Point>> = None;
        let mut first_point = None;

        for command in Self::decode_commands(commands, extent) {
            match command? {
                MvtGeomCommand::MoveTo(p) => {
                    if let Some(curr) = current_contour.take() {
                        if curr.len() < 2 {
                            return Err(GalileoMvtError::Generic(
                                "A line cannot have less then 1 point".into(),
                            ));
                        }

                        contours.push(Contour::open(curr));
                    }

                    first_point = Some(p);
                }
                MvtGeomCommand::LineTo(p, count) => {
                    if let Some(curr) = &mut current_contour {
                        // todo: make this less hacky
                        if curr[curr.len() - 1].taxicab_distance(&p) < 1.0 / 1024.0 {
                            continue;
                        }

                        curr.push(p);
                    } else if let Some(first) = first_point {
                        let mut curr = Vec::with_capacity(count);
                        curr.push(first);
                        curr.push(p);
                        current_contour = Some(curr);
                    } else {
                        return Err(GalileoMvtError::Generic(
                            "First command in the line cannot be MoveTo".into(),
                        ));
                    }
                }
                _ => {
                    return Err(GalileoMvtError::Generic(
                        "Linestring geometry cannot have {:?} command".into(),
                    ))
                }
            }
        }

        if let Some(contour) = current_contour {
            if contour.len() < 2 {
                return Err(GalileoMvtError::Generic(
                    "A line cannot have less then 1 point".into(),
                ));
            }

            contours.push(Contour::open(contour));
        }

        Ok(contours)
    }

    fn decode_polygon(
        commands: Vec<u32>,
        extent: u32,
    ) -> Result<Vec<Polygon<Point>>, GalileoMvtError> {
        let mut polygons = Vec::with_capacity(64);
        let mut curr_polygon = None;
        let mut curr_contour: Option<Vec<Point>> = None;
        let mut first_point = None;

        for command in Self::decode_commands(commands, extent) {
            match command? {
                MvtGeomCommand::MoveTo(p) => {
                    if curr_contour.is_some() {
                        return Err(GalileoMvtError::Generic(
                            "Polygon cannot have unclosed contours".into(),
                        ));
                    }

                    first_point = Some(p);
                }
                MvtGeomCommand::LineTo(p, count) => {
                    if let Some(curr) = &mut curr_contour {
                        curr.push(p)
                    } else if let Some(first) = first_point {
                        let mut curr = Vec::with_capacity(count);
                        curr.push(first);
                        curr.push(p);
                        curr_contour = Some(curr);
                    } else {
                        return Err(GalileoMvtError::Generic(
                            "Contour must start with move to command".into(),
                        ));
                    }
                }
                MvtGeomCommand::ClosePath => {
                    let Some(curr) = curr_contour.take() else {
                        return Err(GalileoMvtError::Generic(
                            "No opened polygon, cannot close path".into(),
                        ));
                    };

                    let curr = ClosedContour::new(curr);

                    // Since tile vectors have y axis pointing down, clockwiseness is also reversed
                    // here.
                    if let Some(mut polygon) = curr_polygon.take() {
                        match curr.winding() {
                            Winding::CounterClockwise => {
                                curr_polygon = Some(Polygon {
                                    outer_contour: curr,
                                    inner_contours: vec![],
                                });
                                polygons.push(polygon);
                            }
                            Winding::Clockwise => {
                                polygon.inner_contours.push(curr);
                                curr_polygon = Some(polygon);
                            }
                        }
                    } else {
                        if curr.winding() == Winding::Clockwise {
                            return Err(GalileoMvtError::Generic(
                                "Outer contour of polygon cannot have counterclockwise winding"
                                    .into(),
                            ));
                        }

                        curr_polygon = Some(Polygon {
                            outer_contour: curr,
                            inner_contours: vec![],
                        });
                    }
                }
            }
        }

        if let Some(polygon) = curr_polygon {
            polygons.push(polygon);
        }

        Ok(polygons)
    }

    fn decode_commands(
        commands: Vec<u32>,
        extent: u32,
    ) -> impl Iterator<Item = Result<MvtGeomCommand, GalileoMvtError>> {
        CommandIterator::new(commands.into_iter(), extent)
    }
}

struct CommandIterator<T: Iterator<Item = u32>> {
    inner: T,
    extent: u32,
    current_command: Option<(u32, u32)>,
    can_continue: bool,
    cursor: Point,
}

impl<T: Iterator<Item = u32>> CommandIterator<T> {
    fn new(inner: T, extent: u32) -> Self {
        Self {
            inner,
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

    fn read_line_to(&mut self, command_count: u32) -> Result<MvtGeomCommand, GalileoMvtError> {
        self.cursor = self.read_point()?;
        Ok(MvtGeomCommand::LineTo(
            self.cursor,
            command_count as usize + 1,
        ))
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
        for i in 0..COUNT {
            result[i] = match self.inner.next() {
                Some(v) => v,
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

impl<T: Iterator<Item = u32>> Iterator for CommandIterator<T> {
    type Item = Result<MvtGeomCommand, GalileoMvtError>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.can_continue {
            return None;
        }

        let (command_id, command_count) = match self.current_command {
            Some((id, count)) => (id, count),
            None => {
                let command_integer = self.inner.next()?;
                let command_id = command_integer & 0x7;
                let command_count = command_integer >> 3;

                (command_id, command_count)
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
            v => Some((command_id, v - 1)),
        };

        Some(match command_id {
            1 => self.read_move_to(),
            2 => self.read_line_to(command_count),
            7 => {
                if command_count != 1 {
                    self.can_continue = false;
                    Err(GalileoMvtError::Generic(format!(
                        "ClosePath command must have count 0, but has {command_count}"
                    )))
                } else {
                    Ok(MvtGeomCommand::ClosePath)
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
    LineTo(Point, usize),
    ClosePath,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

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
        let _tile = MvtTile::decode(&mut Cursor::new(&vt), false).unwrap();
    }
}
