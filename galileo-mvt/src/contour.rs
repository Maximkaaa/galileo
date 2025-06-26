use std::iter::Skip;
use std::slice::Iter;
use std::sync::Arc;

use galileo_types::cartesian::{CartesianClosedContour, CartesianPoint2d, Winding};
use galileo_types::geometry_type::{
    CartesianSpace2d, ContourGeometryType, GeometryType, MultiContourGeometryType,
    PolygonGeometryType,
};
use galileo_types::{ClosedContour, Contour, MultiContour, MultiPolygon, Polygon};
use serde::de::Error;
use serde::{Deserialize, Serialize};

use crate::error::GalileoMvtError;
use crate::{CommandIterator, MvtGeomCommand, Point};

#[derive(Debug, Clone, PartialEq)]
pub struct MvtMultiPolygon {
    polygons: Vec<MvtPolygon>,
}

impl MultiPolygon for MvtMultiPolygon {
    type Polygon = MvtPolygon;

    fn polygons(&self) -> impl Iterator<Item = &Self::Polygon> {
        self.polygons.iter()
    }
}

impl MvtMultiPolygon {
    pub fn new(commands: Vec<u32>, extent: u32) -> Result<MvtMultiPolygon, GalileoMvtError> {
        let polygons = MvtPolygon::new(commands, extent)?;
        Ok(Self { polygons })
    }
}

#[derive(Clone, PartialEq)]
pub struct MvtPolygon {
    commands: Arc<Vec<u32>>,
    extent: u32,
    contours: Vec<ClosedMvtContour>,
}

impl std::fmt::Debug for MvtPolygon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MvtPolygon")
            .field("extent", &self.extent)
            .field("contours", &self.contours)
            .finish()
    }
}

impl MvtPolygon {
    pub fn new(commands: Vec<u32>, extent: u32) -> Result<Vec<MvtPolygon>, GalileoMvtError> {
        Self::new_with_arc(Arc::new(commands), extent)
    }

    fn new_with_arc(
        commands: Arc<Vec<u32>>,
        extent: u32,
    ) -> Result<Vec<MvtPolygon>, GalileoMvtError> {
        let MvtContours {
            commands, contours, ..
        } = MvtContours::new_with_arc(commands, extent)?;

        if contours.iter().any(|c| !c.is_closed()) {
            return Err(GalileoMvtError::Generic(String::from(
                "polygon cannot contain open contours",
            )));
        }

        let mut polygons = vec![];
        for contour in contours {
            let contour = ClosedMvtContour { inner: contour };
            match contour.winding() {
                Winding::CounterClockwise => {
                    polygons.push(MvtPolygon {
                        commands: commands.clone(),
                        extent,
                        contours: vec![contour],
                    });
                }
                Winding::Clockwise => {
                    if !polygons.is_empty() {
                        let last_index = polygons.len() - 1;
                        polygons[last_index].contours.push(contour);
                    }
                }
            }
        }

        Ok(polygons)
    }
}

impl Polygon for MvtPolygon {
    type Contour = ClosedMvtContour;

    fn outer_contour(&self) -> &Self::Contour {
        &self.contours[0]
    }

    fn inner_contours(&self) -> impl Iterator<Item = &'_ Self::Contour> {
        self.contours.iter().skip(1)
    }
}

#[derive(Clone, PartialEq)]
pub struct MvtContours {
    commands: Arc<Vec<u32>>,
    extent: u32,
    contours: Vec<MvtContour>,
}

impl std::fmt::Debug for MvtContours {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MvtContours")
            .field("extent", &self.extent)
            .field("contours", &self.contours)
            .finish()
    }
}

#[derive(Serialize, Deserialize)]
struct MvtContoursSer {
    commands: Arc<Vec<u32>>,
    extent: u32,
}

impl MvtContours {
    pub fn new(commands: Vec<u32>, extent: u32) -> Result<Self, GalileoMvtError> {
        Self::new_with_arc(Arc::new(commands), extent)
    }

    fn new_with_arc(commands: Arc<Vec<u32>>, extent: u32) -> Result<Self, GalileoMvtError> {
        const MOVE_TO_INDEX_COUNT: usize = 3;

        let mut contours = vec![];
        let mut curr_contour = None;
        for command_res in CommandIterator::new(commands.iter(), extent) {
            match command_res? {
                (MvtGeomCommand::MoveTo(point), index) => {
                    if let Some(contour) = curr_contour.take() {
                        contours.push(contour);
                    }

                    curr_contour = Some(MvtContour {
                        commands: commands.clone(),
                        start_index: index + MOVE_TO_INDEX_COUNT,
                        start_point: point,
                        scale: extent,
                        is_closed: false,
                    });
                }
                (MvtGeomCommand::ClosePath, _) => {
                    if let Some(mut contour) = curr_contour.take() {
                        contour.is_closed = true;
                        contours.push(contour)
                    }
                }
                _ => {}
            }
        }

        if let Some(contour) = curr_contour {
            contours.push(contour);
        }

        Ok(Self {
            commands,
            extent,
            contours,
        })
    }
}

impl Serialize for MvtMultiPolygon {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let ser = MvtContoursSer {
            commands: self.polygons[0].commands.clone(),
            extent: self.polygons[0].extent,
        };
        ser.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for MvtMultiPolygon {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let MvtContoursSer { commands, extent } = MvtContoursSer::deserialize(deserializer)?;

        let polygons = MvtPolygon::new_with_arc(commands, extent)
            .map_err(|e| D::Error::custom(format!("failed to deserialize mvt contours: {e}")))?;
        Ok(Self { polygons })
    }
}

impl Serialize for MvtContours {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let ser = MvtContoursSer {
            commands: self.commands.clone(),
            extent: self.extent,
        };
        ser.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for MvtContours {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let MvtContoursSer { commands, extent } = MvtContoursSer::deserialize(deserializer)?;

        let value = Self::new_with_arc(commands, extent)
            .map_err(|e| D::Error::custom(format!("failed to deserialize mvt contours: {e}")))?;
        Ok(value)
    }
}

impl MultiContour for MvtContours {
    type Contour = MvtContour;

    fn contours(&self) -> impl Iterator<Item = &Self::Contour> {
        self.contours.iter()
    }
}

impl GeometryType for MvtContour {
    type Type = ContourGeometryType;
    type Space = CartesianSpace2d;
}

impl GeometryType for ClosedMvtContour {
    type Type = ContourGeometryType;
    type Space = CartesianSpace2d;
}

impl GeometryType for MvtContours {
    type Type = MultiContourGeometryType;
    type Space = CartesianSpace2d;
}

impl GeometryType for MvtPolygon {
    type Type = PolygonGeometryType;
    type Space = CartesianSpace2d;
}

#[derive(Clone, PartialEq)]
pub struct MvtContour {
    commands: Arc<Vec<u32>>,
    start_index: usize,
    start_point: super::Point,
    scale: u32,
    is_closed: bool,
}

impl std::fmt::Debug for MvtContour {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // todo: refactor this to use `f.debug_struct()` when `DebugStruct::field_with` is stabilized

        write!(
            f,
            "MvtContour {{ start_index: {}, start_point: {:?}, scale: {}, is_closed: {}, points: [",
            self.start_index, self.start_point, self.scale, self.is_closed
        )?;
        let mut is_first = true;
        for point in self.iter_points() {
            if !is_first {
                write!(f, ", ")?;
            }

            write!(f, "[{}, {}]", point.x(), point.y())?;
            is_first = false;
        }
        write!(f, "] }}")
    }
}

impl Contour for MvtContour {
    type Point = Point;

    fn is_closed(&self) -> bool {
        self.is_closed
    }

    fn iter_points(&self) -> impl Iterator<Item = Self::Point> {
        MvtContourIterator::new(self)
    }
}

struct MvtContourIterator<'a> {
    command_iterator: CommandIterator<'a, Skip<Iter<'a, u32>>>,
    start_point: Point,
    started: bool,
    finished: bool,
}

impl<'a> MvtContourIterator<'a> {
    fn new(contour: &'a MvtContour) -> Self {
        let inner = contour
            .commands
            .iter()
            .skip(contour.start_index)
            .enumerate();
        Self {
            command_iterator: CommandIterator {
                inner,
                extent: contour.scale,
                current_command: None,
                can_continue: true,
                cursor: contour.start_point,
            },
            start_point: contour.start_point,
            started: false,
            finished: false,
        }
    }
}

impl Iterator for MvtContourIterator<'_> {
    type Item = Point;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        if !self.started {
            self.started = true;
            return Some(self.start_point);
        }

        match self.command_iterator.next()?.ok()?.0 {
            MvtGeomCommand::LineTo(point) => Some(point),
            _ => {
                self.finished = true;
                None
            }
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct ClosedMvtContour {
    inner: MvtContour,
}

impl std::fmt::Debug for ClosedMvtContour {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClosedMvtContour")
            .field("inner", &self.inner)
            .finish()
    }
}

impl ClosedContour for ClosedMvtContour {
    type Point = Point;

    fn iter_points(&self) -> impl Iterator<Item = Self::Point> {
        self.inner.iter_points()
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::{MvtGeometry, MvtTile};

    #[test]
    fn polygon_serialization() {
        let vt = include_bytes!("../test-data/vt.mvt");
        let tile = MvtTile::decode(&mut Cursor::new(&vt), false).unwrap();

        let layer = tile.layers.iter().find(|l| l.name == "water").unwrap();
        for feature in &layer.features {
            let geometry = &feature.geometry;
            let bytes =
                bincode::serde::encode_to_vec(geometry, bincode::config::standard()).unwrap();
            let (deserialized, _): (MvtGeometry, _) =
                bincode::serde::decode_from_slice(&bytes, bincode::config::standard()).unwrap();

            assert_eq!(&deserialized, geometry);
        }
    }
}
