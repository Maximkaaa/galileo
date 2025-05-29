use std::iter::Skip;
use std::slice::Iter;
use std::sync::Arc;

use galileo_types::cartesian::{CartesianClosedContour, Winding};
use galileo_types::geometry_type::{
    CartesianSpace2d, ContourGeometryType, GeometryType, MultiContourGeometryType,
    PolygonGeometryType,
};
use galileo_types::{ClosedContour, Contour, MultiContour, Polygon};
use serde::de::Error;
use serde::{Deserialize, Serialize};

use crate::error::GalileoMvtError;
use crate::{CommandIterator, MvtGeomCommand, Point};

#[derive(Debug, Clone)]
pub struct MvtPolygon {
    commands: Arc<Vec<u32>>,
    extent: u32,
    contours: Vec<ClosedMvtContour>,
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

#[derive(Debug, Clone)]
pub struct MvtContours {
    commands: Arc<Vec<u32>>,
    extent: u32,
    contours: Vec<MvtContour>,
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

        Ok(Self {
            commands,
            extent,
            contours,
        })
    }
}

impl Serialize for MvtPolygon {
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

impl<'de> Deserialize<'de> for MvtPolygon {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let MvtContoursSer { commands, extent } = MvtContoursSer::deserialize(deserializer)?;

        let value = Self::new_with_arc(commands, extent)
            .map_err(|e| D::Error::custom(format!("failed to deserialize mvt contours: {e}")))?;
        value
            .into_iter()
            .next()
            .ok_or_else(|| D::Error::custom("cannot deserialize mvt polygon without contours"))
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

#[derive(Debug, Clone)]
pub struct MvtContour {
    commands: Arc<Vec<u32>>,
    start_index: usize,
    start_point: super::Point,
    scale: u32,
    is_closed: bool,
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
        }
    }
}

impl Iterator for MvtContourIterator<'_> {
    type Item = Point;

    fn next(&mut self) -> Option<Self::Item> {
        match self.command_iterator.next()?.ok()?.0 {
            MvtGeomCommand::LineTo(point) => Some(point),
            MvtGeomCommand::ClosePath => Some(self.start_point),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClosedMvtContour {
    inner: MvtContour,
}

impl ClosedContour for ClosedMvtContour {
    type Point = Point;

    fn iter_points(&self) -> impl Iterator<Item = Self::Point> {
        self.inner.iter_points()
    }
}
