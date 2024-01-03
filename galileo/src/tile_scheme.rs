use galileo_types::cartesian::impls::point::Point2d;
use galileo_types::cartesian::rect::Rect;
use galileo_types::cartesian::traits::cartesian_point::CartesianPoint2d;
use galileo_types::geo::crs::Crs;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::bounding_box::BoundingBox;
use crate::lod::Lod;
use crate::view::MapView;

const RESOLUTION_TOLERANCE: f64 = 0.01;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum VerticalDirection {
    TopToBottom,
    BottomToTop,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash, Serialize, Deserialize)]
pub struct TileIndex {
    pub z: u32,
    pub x: i64,
    pub y: i64,
    pub display_x: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileScheme {
    pub origin: Point2d,
    pub bounds: BoundingBox,
    pub lods: BTreeSet<Lod>,
    pub tile_width: u32,
    pub tile_height: u32,
    pub y_direction: VerticalDirection,
    pub max_tile_scale: f64,
    pub cycle_x: bool,
    pub crs: Crs,
}

impl TileScheme {
    pub fn lod_resolution(&self, z: u32) -> Option<f64> {
        for lod in &self.lods {
            if lod.z_index() == z {
                return Some(lod.resolution());
            }
        }

        None
    }

    pub fn tile_width(&self) -> u32 {
        self.tile_width
    }

    pub fn tile_height(&self) -> u32 {
        self.tile_height
    }

    pub fn select_lod(&self, resolution: f64) -> Option<Lod> {
        if !resolution.is_finite() {
            return None;
        }

        let mut prev_lod = self.lods.iter().next()?;

        for lod in self.lods.iter().skip(1) {
            if lod.resolution() * (1.0 - RESOLUTION_TOLERANCE) > resolution {
                break;
            }

            prev_lod = lod;
        }

        if prev_lod.resolution() / resolution > self.max_tile_scale
            || resolution / prev_lod.resolution() > self.max_tile_scale
        {
            None
        } else {
            Some(*prev_lod)
        }
    }

    pub fn iter_tiles(&self, view: &MapView) -> Option<impl Iterator<Item = TileIndex>> {
        if *view.crs() != self.crs {
            return None;
        }

        let resolution = view.resolution();
        let bounding_box = view.get_bbox()?;
        self.iter_tiles_over_bbox(resolution, bounding_box)
    }

    fn iter_tiles_over_bbox(
        &self,
        resolution: f64,
        bounding_box: Rect,
    ) -> Option<impl Iterator<Item = TileIndex>> {
        let lod = self.select_lod(resolution)?;

        let tile_w = lod.resolution() * self.tile_width as f64;
        let tile_h = lod.resolution() * self.tile_height as f64;

        let x_min = (self.x_adj(bounding_box.x_min()) / tile_w) as i64;
        let x_min = x_min.max(self.min_x_index(lod.resolution()));

        let x_max_adj = self.x_adj(bounding_box.x_max());
        let x_add_one = if (x_max_adj % tile_w) < 0.001 { -1 } else { 0 };

        let x_max = (x_max_adj / tile_w) as i64 + x_add_one;
        let x_max = x_max.min(self.max_x_index(lod.resolution()));

        let (top, bottom) = if self.y_direction == VerticalDirection::TopToBottom {
            (bounding_box.y_min(), bounding_box.y_max())
        } else {
            (bounding_box.y_max(), bounding_box.y_min())
        };

        let y_min = (self.y_adj(bottom) / tile_h) as i64;
        let y_min = y_min.max(self.min_y_index(lod.resolution()));

        let y_max_adj = self.y_adj(top);
        let y_add_one = if (y_max_adj % tile_h) < 0.001 { -1 } else { 0 };

        let y_max = (y_max_adj / tile_h) as i64 + y_add_one;
        let y_max = y_max.min(self.max_y_index(lod.resolution()));

        Some((x_min..=x_max).flat_map(move |x| {
            (y_min..=y_max).map(move |y| TileIndex {
                x,
                y,
                z: lod.z_index(),
                display_x: x,
            })
        }))
    }

    pub fn get_substitutes(&self, index: TileIndex) -> Option<impl Iterator<Item = TileIndex>> {
        let lod = self.lod_over(index.z)?;
        self.iter_tiles_over_bbox(lod.resolution(), self.tile_bbox(index)?)
    }

    /// Returns lod one z-level over the given.
    fn lod_over(&self, z: u32) -> Option<&Lod> {
        let mut lod_iter = self.lods.iter();
        while let Some(lod) = lod_iter.next() {
            if lod.z_index() == z {
                break;
            }
        }

        lod_iter.next()
    }

    fn x_adj(&self, x: f64) -> f64 {
        x - self.origin.x()
    }

    fn y_adj(&self, y: f64) -> f64 {
        match self.y_direction {
            VerticalDirection::TopToBottom => self.origin.y() - y,
            VerticalDirection::BottomToTop => y - self.origin.y(),
        }
    }

    pub fn web(lods_count: u32) -> Self {
        const ORIGIN: Point2d = Point2d::new(-20037508.342787, 20037508.342787);
        const TOP_RESOLUTION: f64 = 156543.03392800014;

        let mut lods = vec![Lod::new(TOP_RESOLUTION, 0).unwrap()];
        for i in 1..lods_count {
            lods.push(Lod::new(lods[(i - 1) as usize].resolution() / 2.0, i).unwrap());
        }

        TileScheme {
            origin: ORIGIN,
            bounds: BoundingBox::new(
                -20037508.342787,
                -20037508.342787,
                20037508.342787,
                20037508.342787,
            ),
            lods: lods.into_iter().collect(),
            tile_width: 256,
            tile_height: 256,
            y_direction: VerticalDirection::TopToBottom,
            max_tile_scale: 1024.0,
            cycle_x: true,
            crs: Crs::EPSG3857,
        }
    }

    pub fn tile_bbox(&self, index: TileIndex) -> Option<Rect> {
        let resolution = self
            .lods
            .iter()
            .find(|lod| lod.z_index() == index.z)?
            .resolution();
        let x_min = self.origin.x() + (index.x as f64) * self.tile_width as f64 * resolution;
        let y_min = match self.y_direction {
            VerticalDirection::TopToBottom => {
                self.origin.y() - (index.y + 1) as f64 * self.tile_height as f64 * resolution
            }
            VerticalDirection::BottomToTop => {
                self.origin.y() + (index.y as f64) * self.tile_height as f64 * resolution
            }
        };

        Some(Rect::new(
            x_min,
            y_min,
            x_min + self.tile_width as f64 * resolution,
            y_min + self.tile_height as f64 * resolution,
        ))
    }

    fn min_x_index(&self, resolution: f64) -> i64 {
        ((self.bounds.x_min() - self.origin.x()) / resolution / self.tile_width as f64).floor()
            as i64
    }

    fn max_x_index(&self, resolution: f64) -> i64 {
        let pix_bound = (self.bounds.x_max() - self.origin.x()) / resolution;
        let floored = pix_bound.floor();
        if (pix_bound - floored).abs() < 0.1 {
            (floored / self.tile_width as f64) as i64 - 1
        } else {
            (floored / self.tile_width as f64) as i64
        }
    }

    fn min_y_index(&self, resolution: f64) -> i64 {
        match self.y_direction {
            VerticalDirection::TopToBottom => {
                ((self.bounds.y_min() + self.origin.y()) / resolution / self.tile_height as f64)
                    .floor() as i64
            }
            VerticalDirection::BottomToTop => {
                ((self.bounds.y_min() - self.origin.y()) / resolution / self.tile_height as f64)
                    .floor() as i64
            }
        }
    }

    fn max_y_index(&self, resolution: f64) -> i64 {
        let pix_bound = match self.y_direction {
            VerticalDirection::TopToBottom => (self.bounds.y_max() + self.origin.y()) / resolution,
            VerticalDirection::BottomToTop => (self.bounds.y_max() - self.origin.y()) / resolution,
        };
        let floored = pix_bound.floor();
        if (pix_bound - floored).abs() < 0.1 {
            (floored / self.tile_height as f64) as i64 - 1
        } else {
            (floored / self.tile_height as f64) as i64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use galileo_types::cartesian::size::Size;

    fn simple_schema() -> TileScheme {
        TileScheme {
            origin: Point2d::default(),
            bounds: BoundingBox::new(0.0, 0.0, 2048.0, 2048.0),
            lods: [
                Lod::new(8.0, 0).unwrap(),
                Lod::new(4.0, 1).unwrap(),
                Lod::new(2.0, 2).unwrap(),
            ]
            .into(),
            tile_width: 256,
            tile_height: 256,
            y_direction: VerticalDirection::BottomToTop,
            max_tile_scale: 2.0,
            cycle_x: false,
            crs: Crs::EPSG3857,
        }
    }

    fn get_view(resolution: f64, bbox: Rect) -> MapView {
        MapView::new_projected(&bbox.center(), resolution).with_size(Size::new(
            bbox.width() / resolution,
            bbox.height() / resolution,
        ))
    }

    #[test]
    fn select_lod() {
        let schema = simple_schema();
        assert_eq!(schema.select_lod(8.0).unwrap().z_index(), 0);
        assert_eq!(schema.select_lod(9.0).unwrap().z_index(), 0);
        assert_eq!(schema.select_lod(16.0).unwrap().z_index(), 0);
        assert_eq!(schema.select_lod(7.99).unwrap().z_index(), 0);
        assert_eq!(schema.select_lod(7.5).unwrap().z_index(), 1);
        assert_eq!(schema.select_lod(4.1).unwrap().z_index(), 1);
        assert_eq!(schema.select_lod(4.0).unwrap().z_index(), 1);
        assert_eq!(schema.select_lod(1.5).unwrap().z_index(), 2);
        assert_eq!(schema.select_lod(1.0).unwrap().z_index(), 2);
        assert_eq!(schema.select_lod(0.5), None);
        assert_eq!(schema.select_lod(0.0), None);
        assert_eq!(schema.select_lod(100500.0), None);
        assert_eq!(schema.select_lod(f64::INFINITY), None);
        assert_eq!(schema.select_lod(f64::NEG_INFINITY), None);
        assert_eq!(schema.select_lod(f64::NAN), None);
    }

    #[test]
    fn select_lod_considers_max_tile_scale() {
        let mut schema = simple_schema();
        assert!(schema.select_lod(16.0).is_some());
        assert!(schema.select_lod(1.0).is_some());
        assert!(schema.select_lod(17.0).is_none());
        assert!(schema.select_lod(0.9).is_none());

        schema.max_tile_scale = 1.5;
        assert!(schema.select_lod(16.0).is_none());
        assert!(schema.select_lod(1.0).is_none());
        assert!(schema.select_lod(17.0).is_none());
        assert!(schema.select_lod(0.9).is_none());

        schema.max_tile_scale = 2.5;
        assert!(schema.select_lod(16.0).is_some());
        assert!(schema.select_lod(1.0).is_some());
        assert!(schema.select_lod(17.0).is_some());
        assert!(schema.select_lod(0.9).is_some());
    }

    #[test]
    fn iter_indices_full_bbox() {
        let schema = simple_schema();
        let bbox = Rect::new(0.0, 0.0, 2048.0, 2048.0);
        let view = get_view(8.0, bbox);
        assert_eq!(schema.iter_tiles(&view).unwrap().count(), 1);
        for tile in schema.iter_tiles(&view).unwrap() {
            assert_eq!(tile.x, 0);
            assert_eq!(tile.y, 0);
            assert_eq!(tile.z, 0);
        }

        let view = get_view(4.0, bbox);
        let mut tiles: Vec<TileIndex> = schema.iter_tiles(&view).unwrap().collect();
        tiles.dedup();
        assert_eq!(tiles.len(), 4);
        for tile in tiles {
            assert!(tile.x >= 0 && tile.x <= 1);
            assert!(tile.y >= 0 && tile.y <= 1);
            assert_eq!(tile.z, 1);
        }

        let view = get_view(2.0, bbox);
        let mut tiles: Vec<TileIndex> = schema.iter_tiles(&view).unwrap().collect();
        tiles.dedup();
        assert_eq!(tiles.len(), 16);
        for tile in tiles {
            assert!(tile.x >= 0 && tile.x <= 3);
            assert!(tile.y >= 0 && tile.y <= 3);
            assert_eq!(tile.z, 2);
        }
    }

    #[test]
    fn iter_indices_part_bbox() {
        let schema = simple_schema();
        let bbox = Rect::new(200.0, 700.0, 1200.0, 1100.0);
        let view = get_view(8.0, bbox);
        assert_eq!(schema.iter_tiles(&view).unwrap().count(), 1);
        for tile in schema.iter_tiles(&view).unwrap() {
            assert_eq!(tile.x, 0);
            assert_eq!(tile.y, 0);
            assert_eq!(tile.z, 0);
        }

        let view = get_view(4.0, bbox);
        let mut tiles: Vec<TileIndex> = schema.iter_tiles(&view).unwrap().collect();
        tiles.dedup();
        assert_eq!(tiles.len(), 4);
        for tile in tiles {
            assert!(tile.x >= 0 && tile.x <= 1);
            assert!(tile.y >= 0 && tile.y <= 1);
            assert_eq!(tile.z, 1);
        }

        let view = get_view(2.0, bbox);
        let mut tiles: Vec<TileIndex> = schema.iter_tiles(&view).unwrap().collect();
        tiles.dedup();
        assert_eq!(tiles.len(), 6);
        for tile in tiles {
            assert!(tile.x >= 0 && tile.x <= 2);
            assert!(tile.y >= 1 && tile.y <= 2);
            assert_eq!(tile.z, 2);
        }
    }

    #[test]
    fn iter_tiles_outside_of_bbox() {
        let schema = simple_schema();
        let bbox = Rect::new(-100.0, -100.0, -50.0, -50.0);
        let view = get_view(8.0, bbox);
        assert_eq!(schema.iter_tiles(&view).unwrap().count(), 0);
        let view = get_view(2.0, bbox);
        assert_eq!(schema.iter_tiles(&view).unwrap().count(), 0);

        let bbox = Rect::new(2100.0, 0.0, 2500.0, 2048.0);
        let view = get_view(8.0, bbox);
        assert_eq!(schema.iter_tiles(&view).unwrap().count(), 0);
        let view = get_view(2.0, bbox);
        assert_eq!(schema.iter_tiles(&view).unwrap().count(), 0);
    }

    #[test]
    fn iter_tiles_does_not_include_tiles_outside_bbox() {
        let schema = simple_schema();
        let bbox = Rect::new(-2048.0, -2048.0, 4096.0, 4096.0);
        let view = get_view(8.0, bbox);
        for tile in schema.iter_tiles(&view).unwrap() {
            println!("{tile:?}");
        }

        assert_eq!(schema.iter_tiles(&view).unwrap().count(), 1);
        let view = get_view(2.0, bbox);
        assert_eq!(schema.iter_tiles(&view).unwrap().count(), 16);
    }

    #[test]
    fn lod_over() {
        let schema = simple_schema();
        assert_eq!(schema.lod_over(0), None);
        assert_eq!(schema.lod_over(1).unwrap().z_index(), 0);
        assert_eq!(schema.lod_over(2).unwrap().z_index(), 1);
        assert_eq!(schema.lod_over(3), None);
    }
}
