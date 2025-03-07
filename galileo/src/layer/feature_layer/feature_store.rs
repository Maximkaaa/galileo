use std::sync::atomic::{AtomicU64, Ordering};

use ahash::{HashMap, HashMapExt};
use maybe_sync::{MaybeSend, MaybeSync};

/// Unique identifier of a feature in a feature layer.
///
/// This type is opaque on purpose. Application code should not make any assumption about insides
/// of this type, as it may change in future. The only important property of the `FeatureId` that
/// must be observed is that an implementation of `FeatureStore` must return a unique value of
/// `FeatureId` for each unique feature in a feature layer, and this id must not change during
/// lifetime of the layer.
///
/// This rule allows however for the id to change between runs of the application. And same ids can
/// be used for different features in different layers.
///
/// To get a unique value of a `FeatureId` use [`FeatureId::next()`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct FeatureId(u64);

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

impl FeatureId {
    /// Returns application-wise unique `FeatureId`.
    pub fn next() -> Self {
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

/// Collection of features for a feature layer.
pub trait FeatureStore<F>: MaybeSend + MaybeSync {
    /// Returns an iterator over all features with their ids.
    fn iter(&self) -> Box<dyn Iterator<Item = (FeatureId, &F)> + '_>;

    /// Returns a mutable iterator over all features with their ids.
    fn iter_mut(&mut self) -> Box<dyn Iterator<Item = (FeatureId, &mut F)> + '_>;

    /// Returns a shared reference to the feature with the given id, or `None` if it does not exist.
    fn get(&self, id: FeatureId) -> Option<&F>;

    /// Returns an exclusive reference to the feature with the given id, or `None` if it does not
    /// exist.
    fn get_mut(&mut self, id: FeatureId) -> Option<&mut F>;

    /// Adds the `feature` to the store and returns its id.
    fn add(&mut self, feature: F) -> FeatureId;

    /// Removes the feature from the store returning the feature itself.
    ///
    /// If the feature with the given id is not in the store, returns `None`.
    fn remove(&mut self, id: FeatureId) -> Option<F>;
}

pub(super) struct VecFeatureStore<F> {
    features: Vec<(FeatureId, F)>,
    ids: HashMap<FeatureId, usize>,
}

impl<F> VecFeatureStore<F> {
    pub(super) fn new(feature_iter: impl IntoIterator<Item = F>) -> Self {
        let mut features = vec![];
        let mut ids = HashMap::new();
        for (index, feature) in feature_iter.into_iter().enumerate() {
            let id = FeatureId::next();
            features.push((id, feature));
            ids.insert(id, index);
        }

        Self { features, ids }
    }
}

impl<F> FeatureStore<F> for VecFeatureStore<F>
where
    F: MaybeSync + MaybeSend,
{
    fn iter(&self) -> Box<dyn Iterator<Item = (FeatureId, &F)> + '_> {
        Box::new(self.features.iter().map(|(id, f)| (*id, f)))
    }

    fn iter_mut(&mut self) -> Box<dyn Iterator<Item = (FeatureId, &mut F)> + '_> {
        Box::new(self.features.iter_mut().map(|(id, f)| (*id, f)))
    }

    fn get(&self, id: FeatureId) -> Option<&F> {
        self.ids
            .get(&id)
            .and_then(|index| self.features.get(*index))
            .map(|(_, f)| f)
    }

    fn get_mut(&mut self, id: FeatureId) -> Option<&mut F> {
        self.ids
            .get(&id)
            .and_then(|index| self.features.get_mut(*index))
            .map(|(_, f)| f)
    }

    fn add(&mut self, feature: F) -> FeatureId {
        let id = FeatureId::next();
        let index = self.features.len();
        self.features.push((id, feature));
        self.ids.insert(id, index);

        id
    }

    fn remove(&mut self, id: FeatureId) -> Option<F> {
        let index = self.ids.remove(&id)?;
        let (_, feature) = self.features.remove(index);

        for (to_adjust_id, _) in self.features.iter().skip(index) {
            if let Some(index) = self.ids.get_mut(to_adjust_id) {
                *index -= 1;
            }
        }

        Some(feature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vec_store_add_remove_features_preserves_indices() {
        let mut store = VecFeatureStore::new([]);

        let ids: Vec<_> = (0..5).map(|i| store.add(i)).collect();
        store.remove(ids[2]);

        assert_eq!(store.get(ids[2]), None);

        assert_eq!(store.get(ids[0]), Some(&0));
        assert_eq!(store.get(ids[1]), Some(&1));
        assert_eq!(store.get(ids[3]), Some(&3));
        assert_eq!(store.get(ids[4]), Some(&4));
    }
}
