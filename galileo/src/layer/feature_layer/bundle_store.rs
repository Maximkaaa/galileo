use std::sync::atomic::{AtomicU64, Ordering};

use ahash::{HashMap, HashMapExt, HashSet, HashSetExt};

use super::FeatureId;
use crate::render::render_bundle::RenderBundle;
use crate::render::{Canvas, PackedBundle};

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub(super) struct BundleId(u64);

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

impl BundleId {
    fn next() -> Self {
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

pub(super) struct BundleStore {
    bundle_size_limit: usize,
    unpacked: Vec<(BundleId, RenderBundle)>,
    packed: HashMap<BundleId, Box<dyn PackedBundle>>,
    feature_to_bundle_map: HashMap<FeatureId, BundleId>,
    required_update: UpdateType,
}

#[derive(Debug, Default)]
pub(super) enum UpdateType {
    #[default]
    None,
    All,
    Selected(HashSet<FeatureId>),
}

impl UpdateType {
    fn update_all(&mut self) {
        *self = UpdateType::All;
    }

    fn update_feature(&mut self, id: FeatureId) {
        if matches!(self, UpdateType::All) {
            return;
        }

        if matches!(self, UpdateType::None) {
            *self = UpdateType::Selected(HashSet::new());
        }

        if let UpdateType::Selected(sel) = self {
            sel.insert(id);
        }
    }

    fn updated(&mut self) {
        *self = UpdateType::None
    }
}

impl BundleStore {
    pub(super) fn new(bundle_size_limit: usize) -> Self {
        Self {
            bundle_size_limit,
            unpacked: vec![],
            packed: HashMap::new(),
            feature_to_bundle_map: HashMap::new(),
            required_update: UpdateType::All,
        }
    }

    pub(super) fn pack(&mut self, canvas: &dyn Canvas) {
        for (id, bundle) in std::mem::take(&mut self.unpacked) {
            self.packed.insert(id, canvas.pack_bundle(&bundle));
        }

        self.required_update.updated();
    }

    pub(super) fn packed(&self) -> Vec<&dyn PackedBundle> {
        self.packed.values().map(|v| &**v).collect()
    }

    pub(super) fn set_bundle_size_limit(&mut self, limit: usize) {
        self.bundle_size_limit = limit;
    }

    pub(super) fn set_dpi_scale_factor(&mut self, scale: f32) {
        for (_, bundle) in &mut self.unpacked {
            bundle.set_dpi_scale_factor(scale);
        }
    }

    pub(super) fn clear(&mut self) {
        self.unpacked.clear();
        self.packed.clear();
        self.feature_to_bundle_map.clear();
        self.required_update.update_all();
    }

    pub(super) fn with_bundle(&mut self, predicate: impl FnOnce(&mut RenderBundle) -> FeatureId) {
        let (bundle_id, curr_bundle) = {
            let v = self.curr_bundle();
            (v.0, &mut v.1)
        };

        let feature_id = predicate(curr_bundle);

        self.reset_feature(feature_id);

        self.feature_to_bundle_map.insert(feature_id, bundle_id);
    }

    fn curr_bundle(&mut self) -> &mut (BundleId, RenderBundle) {
        if self.last_bundle_is_full() {
            let new_id = BundleId::next();
            self.unpacked.push((new_id, RenderBundle::default()));
        }

        let idx = self.unpacked.len() - 1;
        &mut self.unpacked[idx]
    }

    fn last_bundle_is_full(&self) -> bool {
        self.unpacked
            .iter()
            .last()
            .map(|(_, last)| last.world_set.approx_buffer_size() >= self.bundle_size_limit)
            .unwrap_or(true)
    }

    pub(super) fn reset_feature(&mut self, feature_id: FeatureId) {
        self.required_update.update_feature(feature_id);

        let Some(&bundle_id) = self.feature_to_bundle_map.get(&feature_id) else {
            return;
        };

        if self.packed.remove(&bundle_id).is_none() {
            self.unpacked.retain(|(id, _)| *id != bundle_id);
        }

        for (&feature_id, &id) in &self.feature_to_bundle_map {
            if id == bundle_id {
                self.required_update.update_feature(feature_id);
            }
        }
    }

    pub(super) fn required_update(&mut self) -> UpdateType {
        std::mem::take(&mut self.required_update)
    }
}
