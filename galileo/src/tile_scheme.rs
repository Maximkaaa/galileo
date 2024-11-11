//! [`TileSchema`] is used by tile layers to calculate [tile indices](TileIndex) needed for a given ['MapView'].

use galileo_types::cartesian::{CartesianPoint2d, Point2d, Rect};
use galileo_types::geo::Crs;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[cfg(target_arch = "wasm32")]
use js_sys::wasm_bindgen::prelude::wasm_bindgen;

use crate::lod::Lod;
use crate::view::MapView;

const RESOLUTION_TOLERANCE: f64 = 0.01;

/// Direction of the Y index of tiles.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum VerticalDirection {
    /// Tiles with `Y == 0` are at the top of the map.
    TopToBottom,
    /// Tiles with `Y == 0` are at the bottom of the map.
    BottomToTop,
}

/// Index of a tile.
#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct TileIndex {
    /// Z index.
    pub z: u32,
    /// X index.
    pub x: i32,
    /// Y index.
    pub y: i32,
    pub(crate) display_x: i32,
}

impl TileIndex {
    /// Create a new index instance.
    pub fn new(x: i32, y: i32, z: u32) -> Self {
        Self {
            x,
            y,
            z,
            display_x: x,
        }
    }
}

/// Tile schema specifies how tile indices are calculated based on the map position and resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileSchema {
    /// Position where all tiles have `X == 0, Y == 0` indices.
    pub origin: Point2d,
    /// Rectangle that contains all tiles of the tile scheme.
    pub bounds: Rect,
    /// Sorted set of levels of detail that specify resolutions for each z-level.
    pub lods: BTreeSet<Lod>,
    /// Width of a single tile in pixels.
    pub tile_width: u32,
    /// Height of a single tile in pixels.
    pub tile_height: u32,
    /// Direction of the Y-axis.
    pub y_direction: VerticalDirection,
    /// Crs of the scheme.
    pub crs: Crs,
}

impl TileSchema {
    /// Resolution of the given z-level, if exists.
    pub fn lod_resolution(&self, z: u32) -> Option<f64> {
        for lod in &self.lods {
            if lod.z_index() == z {
                return Some(lod.resolution());
            }
        }

        None
    }

    /// Width of a single tile.
    pub fn tile_width(&self) -> u32 {
        self.tile_width
    }

    /// Height of a single tile.
    pub fn tile_height(&self) -> u32 {
        self.tile_height
    }

    /// Select a level of detail for the given resolution.
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

        Some(*prev_lod)
    }

    /// Iterate over tile indices that should be displayed for the given map view.
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

        let x_min = (self.x_adj(bounding_box.x_min()) / tile_w) as i32;
        let x_min = x_min.max(self.min_x_index(lod.resolution()));

        let x_max_adj = self.x_adj(bounding_box.x_max());
        let x_add_one = if (x_max_adj % tile_w) < 0.001 { -1 } else { 0 };

        let x_max = (x_max_adj / tile_w) as i32 + x_add_one;
        let x_max = x_max.min(self.max_x_index(lod.resolution()));

        let (top, bottom) = if self.y_direction == VerticalDirection::TopToBottom {
            (bounding_box.y_min(), bounding_box.y_max())
        } else {
            (bounding_box.y_max(), bounding_box.y_min())
        };

        let y_min = (self.y_adj(bottom) / tile_h) as i32;
        let y_min = y_min.max(self.min_y_index(lod.resolution()));

        let y_max_adj = self.y_adj(top);
        let y_add_one = if (y_max_adj % tile_h) < 0.001 { -1 } else { 0 };

        let y_max = (y_max_adj / tile_h) as i32 + y_add_one;
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

    pub(crate) fn get_substitutes(
        &self,
        index: TileIndex,
    ) -> Option<impl Iterator<Item = TileIndex>> {
        let lod = self.lod_over(index.z)?;
        // todo: we don't really need shrink here, but .iter_tiles_over_bbox return extra tiles
        // when borders of tiles are exactly on bbox border.
        self.iter_tiles_over_bbox(
            lod.resolution(),
            self.tile_bbox(index)?.shrink(lod.resolution()),
        )
    }

    /// Returns lod one z-level over the given.
    fn lod_over(&self, z: u32) -> Option<&Lod> {
        let mut lod_iter = self.lods.iter();
        for lod in lod_iter.by_ref() {
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

    /// Standard Web Mercator based tile scheme (used, for example, by OSM and Google maps).
    pub fn web(lods_count: u32) -> Self {
        const ORIGIN: Point2d = Point2d::new(-20037508.342787, 20037508.342787);
        const TOP_RESOLUTION: f64 = 156543.03392800014;

        let mut lods = vec![Lod::new(TOP_RESOLUTION, 0).expect("invalid const parameters")];
        for i in 1..lods_count {
            lods.push(
                Lod::new(lods[(i - 1) as usize].resolution() / 2.0, i)
                    .expect("invalid const parameters"),
            );
        }

        TileSchema {
            origin: ORIGIN,
            bounds: Rect::new(
                -20037508.342787,
                -20037508.342787,
                20037508.342787,
                20037508.342787,
            ),
            lods: lods.into_iter().collect(),
            tile_width: 256,
            tile_height: 256,
            y_direction: VerticalDirection::TopToBottom,
            crs: Crs::EPSG3857,
        }
    }

    pub(crate) fn tile_bbox(&self, index: TileIndex) -> Option<Rect> {
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

    fn min_x_index(&self, resolution: f64) -> i32 {
        ((self.bounds.x_min() - self.origin.x()) / resolution / self.tile_width as f64).floor()
            as i32
    }

    fn max_x_index(&self, resolution: f64) -> i32 {
        let pix_bound = (self.bounds.x_max() - self.origin.x()) / resolution;
        let floored = pix_bound.floor();
        if (pix_bound - floored).abs() < 0.1 {
            (floored / self.tile_width as f64) as i32 - 1
        } else {
            (floored / self.tile_width as f64) as i32
        }
    }

    fn min_y_index(&self, resolution: f64) -> i32 {
        match self.y_direction {
            VerticalDirection::TopToBottom => {
                ((self.bounds.y_min() + self.origin.y()) / resolution / self.tile_height as f64)
                    .floor() as i32
            }
            VerticalDirection::BottomToTop => {
                ((self.bounds.y_min() - self.origin.y()) / resolution / self.tile_height as f64)
                    .floor() as i32
            }
        }
    }

    fn max_y_index(&self, resolution: f64) -> i32 {
        let pix_bound = match self.y_direction {
            VerticalDirection::TopToBottom => (self.bounds.y_max() + self.origin.y()) / resolution,
            VerticalDirection::BottomToTop => (self.bounds.y_max() - self.origin.y()) / resolution,
        };
        let floored = pix_bound.floor();
        if (pix_bound - floored).abs() < 0.1 {
            (floored / self.tile_height as f64) as i32 - 1
        } else {
            (floored / self.tile_height as f64) as i32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use galileo_types::cartesian::Size;

    fn simple_schema() -> TileSchema {
        TileSchema {
            origin: Point2d::default(),
            bounds: Rect::new(0.0, 0.0, 2048.0, 2048.0),
            lods: [
                Lod::new(8.0, 0).unwrap(),
                Lod::new(4.0, 1).unwrap(),
                Lod::new(2.0, 2).unwrap(),
            ]
            .into(),
            tile_width: 256,
            tile_height: 256,
            y_direction: VerticalDirection::BottomToTop,
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
