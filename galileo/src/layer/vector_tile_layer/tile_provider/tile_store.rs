use crate::layer::vector_tile_layer::tile_provider::VtStyleId;
use crate::render::render_bundle::RenderBundle;
use crate::render::PackedBundle;
use crate::tile_scheme::TileIndex;
use galileo_mvt::MvtTile;
use quick_cache::unsync::Cache;
use quick_cache::{DefaultHashBuilder, Lifecycle, Weighter};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::{Arc, Weak};
use tokio::sync::OnceCell;

const DEFAULT_CACHE_CAPACITY: usize = 100_000_000;
const AVG_TILE_SIZE: usize = 100_000;
const EMPTY_CELL_SIZE: u32 = 1024;

#[derive(Debug, Clone)]
pub enum MvtTileState {
    Loaded(Arc<MvtTile>),
    Error(),
}

#[derive(Clone)]
pub enum PreparedTileState {
    Loading,
    Loaded(Arc<RenderBundle>),
    Packed(Arc<dyn PackedBundle>),
    Error,
}

impl Debug for PreparedTileState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PreparedTileState::Loading => write!(f, "PreparedTileState::Loading"),
            PreparedTileState::Loaded(_) => write!(f, "PreparedTileState::Loaded"),
            PreparedTileState::Packed(_) => write!(f, "PreparedTileState::Packed"),
            PreparedTileState::Error => write!(f, "PreparedTileState::Error"),
        }
    }
}

struct TileStoreEntry {
    mvt_tile: Arc<OnceCell<MvtTileState>>,
    prepared_tile: PreparedTileState,
}

pub(super) struct TileStore {
    mvt_tiles: HashMap<TileIndex, Weak<OnceCell<MvtTileState>>, ahash::RandomState>,
    processed: Cache<
        (TileIndex, VtStyleId),
        TileStoreEntry,
        TileWeighter,
        DefaultHashBuilder,
        TileStoreLc,
    >,
}

impl Default for TileStore {
    fn default() -> Self {
        Self {
            mvt_tiles: HashMap::default(),
            processed: Cache::with(
                DEFAULT_CACHE_CAPACITY / AVG_TILE_SIZE,
                DEFAULT_CACHE_CAPACITY as u64,
                TileWeighter,
                DefaultHashBuilder::default(),
                TileStoreLc,
            ),
        }
    }
}

struct TileWeighter;

impl Weighter<(TileIndex, VtStyleId), TileStoreEntry> for TileWeighter {
    fn weight(&self, _key: &(TileIndex, VtStyleId), val: &TileStoreEntry) -> u32 {
        match &val.prepared_tile {
            PreparedTileState::Loaded(v) => v.approx_buffer_size() as u32,
            _ => EMPTY_CELL_SIZE,
        }
    }
}

struct TileStoreLc;

#[derive(Debug, Default, Clone)]
struct TileStoreLcState {
    evicted: Vec<TileIndex>,
}

impl Lifecycle<(TileIndex, VtStyleId), TileStoreEntry> for TileStoreLc {
    type RequestState = TileStoreLcState;

    fn begin_request(&self) -> Self::RequestState {
        TileStoreLcState::default()
    }

    fn on_evict(
        &self,
        state: &mut Self::RequestState,
        key: (TileIndex, VtStyleId),
        _val: TileStoreEntry,
    ) {
        state.evicted.push(key.0)
    }
}

impl TileStore {
    #[allow(dead_code)]
    pub fn with_capacity(bytes_size: usize) -> Self {
        Self {
            processed: Cache::with(
                bytes_size / AVG_TILE_SIZE,
                bytes_size as u64,
                TileWeighter,
                DefaultHashBuilder::default(),
                TileStoreLc,
            ),
            ..Self::default()
        }
    }

    pub fn contains(&self, tile_index: TileIndex, style_id: VtStyleId) -> bool {
        self.processed.peek(&(tile_index, style_id)).is_some()
    }

    pub fn start_loading_tile(
        &mut self,
        index: TileIndex,
        style_id: VtStyleId,
    ) -> Arc<OnceCell<MvtTileState>> {
        let tile_cell = self
            .mvt_tiles
            .get(&index)
            .and_then(|v| v.upgrade())
            .unwrap_or_default();
        self.mvt_tiles.insert(index, Arc::downgrade(&tile_cell));

        let entry = TileStoreEntry {
            mvt_tile: tile_cell.clone(),
            prepared_tile: PreparedTileState::Loading,
        };

        self.insert_entry(index, style_id, entry);

        tile_cell
    }

    pub fn store_tile(
        &mut self,
        tile_index: TileIndex,
        style_id: VtStyleId,
        mvt_tile: Arc<OnceCell<MvtTileState>>,
        tile_state: PreparedTileState,
    ) {
        let entry = TileStoreEntry {
            mvt_tile,
            prepared_tile: tile_state,
        };

        self.insert_entry(tile_index, style_id, entry);
    }

    pub fn get_prepared(
        &self,
        index: TileIndex,
        style_id: VtStyleId,
    ) -> Option<(Arc<RenderBundle>, Arc<OnceCell<MvtTileState>>)> {
        self.processed.get(&(index, style_id)).and_then(|entry| {
            if let PreparedTileState::Loaded(tile) = &entry.prepared_tile {
                Some((tile.clone(), entry.mvt_tile.clone()))
            } else {
                None
            }
        })
    }

    pub fn get_packed(
        &self,
        index: TileIndex,
        style_id: VtStyleId,
    ) -> Option<Arc<dyn PackedBundle>> {
        self.processed.get(&(index, style_id)).and_then(|entry| {
            if let PreparedTileState::Packed(tile) = &entry.prepared_tile {
                Some(tile.clone())
            } else {
                None
            }
        })
    }

    pub fn get_mvt_tile(&self, index: TileIndex) -> Option<Arc<MvtTile>> {
        match self
            .mvt_tiles
            .get(&index)
            .and_then(|r| r.upgrade())
            .and_then(|cell| cell.get().cloned())
        {
            Some(MvtTileState::Loaded(tile)) => Some(tile),
            _ => None,
        }
    }

    fn insert_entry(&mut self, index: TileIndex, style_id: VtStyleId, entry: TileStoreEntry) {
        let lc = self
            .processed
            .insert_with_lifecycle((index, style_id), entry);

        for index in lc.evicted {
            self.on_bundle_evicted(index)
        }
    }

    fn on_bundle_evicted(&mut self, tile_index: TileIndex) {
        let Some(mvt_cell_ref) = self.mvt_tiles.get(&tile_index) else {
            return;
        };

        if mvt_cell_ref.strong_count() == 0 {
            self.mvt_tiles.remove(&tile_index);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::render_bundle::tessellating::TessellatingRenderBundle;
    use crate::render::render_bundle::RenderBundleType;

    fn render_bundle(size: usize) -> RenderBundle {
        let mut bundle = RenderBundle(RenderBundleType::Tessellating(
            TessellatingRenderBundle::new(),
        ));
        bundle.set_approx_buffer_size(size);

        bundle
    }

    fn tile_with_size(size: u64) -> PreparedTileState {
        PreparedTileState::Loaded(Arc::new(render_bundle(size as usize)))
    }

    #[test]
    fn returns_same_mvt_tile_for_different_styles() {
        let mut store = TileStore::with_capacity(1_000_000);
        let index = TileIndex::new(0, 0, 0);
        let mvt_cell = store.start_loading_tile(index, VtStyleId::next_id());
        let another_mvt_cell = store.start_loading_tile(index, VtStyleId::next_id());

        assert!(
            Arc::ptr_eq(&mvt_cell, &another_mvt_cell),
            "Mvt cells do not point to the same object"
        );
    }

    #[test]
    fn evicts_old_tiles() {
        const CAPACITY: u64 = 1_000_000;
        const ITEM_SIZE: u64 = 100_000;

        let mut store = TileStore::with_capacity(CAPACITY as usize);
        let style_id = VtStyleId::next_id();
        for i in 0..20 {
            let index = TileIndex::new(i, i, 10);
            let mvt_cell = Arc::default();
            let prepared_tile = tile_with_size(ITEM_SIZE);

            store.store_tile(index, style_id, mvt_cell, prepared_tile);
        }

        assert!(
            store.processed.weight() <= CAPACITY,
            "Cache size ({}) is larger than capacity ({})",
            store.processed.weight(),
            CAPACITY
        );
        assert!(
            store.processed.len() <= (CAPACITY / ITEM_SIZE) as usize,
            "Too many items ({}) in the cache",
            store.processed.len()
        );
        assert!(
            store.processed.len() > (CAPACITY / ITEM_SIZE) as usize - 2,
            "Too few items ({}) in the cache",
            store.processed.len()
        );
    }

    #[test]
    fn removes_mvt_tiles_after_eviction() {
        const CAPACITY: u64 = 1_000_000;
        const ITEM_SIZE: u64 = 100_000;

        let mut store = TileStore::with_capacity(CAPACITY as usize);
        let style_id = VtStyleId::next_id();
        for i in 0..20 {
            let index = TileIndex::new(i, i, 10);

            let mvt_cell = store.start_loading_tile(index, style_id);
            let prepared_tile = tile_with_size(ITEM_SIZE);

            store.store_tile(index, style_id, mvt_cell, prepared_tile);
        }

        assert!(
            store.mvt_tiles.len() <= (CAPACITY / ITEM_SIZE) as usize,
            "Too many mvt tiles ({}) in the cache",
            store.mvt_tiles.len()
        );
        assert!(
            store.mvt_tiles.len() > (CAPACITY / ITEM_SIZE) as usize - 2,
            "Too few mvt tiles ({}) in the cache",
            store.mvt_tiles.len()
        );
    }

    #[test]
    fn does_not_remove_mvt_tiles_if_multiple_styles_are_used() {
        const CAPACITY: u64 = 1_000_000;
        const ITEM_SIZE: u64 = 100_000;

        let mut store = TileStore::with_capacity(CAPACITY as usize);
        for _ in 0..3 {
            let style_id = VtStyleId::next_id();
            for i in 0..7 {
                let index = TileIndex::new(i, i, 10);

                let mvt_cell = store.start_loading_tile(index, style_id);
                let prepared_tile = tile_with_size(ITEM_SIZE);

                store.store_tile(index, style_id, mvt_cell, prepared_tile);
            }
        }

        for index in store.processed.iter().map(|((index, _), _)| index) {
            assert!(
                store.mvt_tiles.contains_key(index),
                "Mvt tiles does not contain index {index:?}"
            );
        }

        for mvt_index in store.mvt_tiles.keys() {
            assert!(
                store
                    .processed
                    .iter()
                    .any(|((index, _), _)| mvt_index == index),
                "Index {mvt_index:?} is in mvt store, but not in processed"
            );
        }
    }
}
