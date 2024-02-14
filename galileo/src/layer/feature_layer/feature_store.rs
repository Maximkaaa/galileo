use std::sync::{Arc, Mutex};

/// Feature storage of a [FeatureLayer](super::FeatureLayer).
///
/// All access operations in the storage return [FeatureContainer] or [FeatureContainerMut] structs. These containers
/// then allow access to references to the features themselves. When a feature is modified through
/// [AsMut::as_mut] or [FeatureContainerMut::edit_style], the `FeatureLayer` containing them
/// is automatically notified of the change, and the layer can update rendering of the given features without redrawing
/// the whole feature set.
#[derive(Default)]
pub struct FeatureStore<F> {
    features: Vec<FeatureEntry<F>>,
    pending_updates: Arc<Mutex<Vec<FeatureUpdate>>>,
}

/// Immutable container for a feature in a [FeatureLayer](super::FeatureLayer).
///
/// Reference to the container can be converted into a reference to the feature using [AsRef] trait.
pub struct FeatureContainer<'a, F> {
    feature: &'a F,
    feature_index: usize,
}

impl<'a, F> FeatureContainer<'a, F> {
    /// Index of the feature in the layer.
    pub fn index(&self) -> usize {
        self.feature_index
    }
}

impl<'a, F> AsRef<F> for FeatureContainer<'a, F> {
    fn as_ref(&self) -> &F {
        self.feature
    }
}

/// Mutable container for a feature in a [FeatureLayer](super::FeatureLayer).
///
/// Reference to the container can be converted into a reference to the feature using [AsRef] and [AsMut] traits.
pub struct FeatureContainerMut<'a, F> {
    entry: &'a mut FeatureEntry<F>,
    feature_index: usize,
    is_updated: bool,
    pending_updates: Arc<Mutex<Vec<FeatureUpdate>>>,
}

impl<'a, F> FeatureContainerMut<'a, F> {
    /// Index of the feature in the layer.
    pub fn index(&self) -> usize {
        self.feature_index
    }

    /// Returns true if the feature is hidden.
    ///
    /// Hidden features keep their place in the layer, but are not displayed on the map.
    pub fn is_hidden(&self) -> bool {
        self.entry.is_hidden
    }

    /// Notifies the layer that after the feature is modified, the geometry will not be changed and only the style
    /// is to be updated. If geometry might change, use [container.as_mut()](AsMut::as_mut) instead.
    pub fn edit_style(self) -> &'a mut F {
        if !self.is_updated {
            self.pending_updates
                .lock()
                .expect("poisoned mutex")
                .push(FeatureUpdate::UpdateStyle {
                    feature_index: self.feature_index,
                });
        }

        &mut self.entry.feature
    }

    /// Hides the feature from the map, but leaves it in the features list.
    pub fn hide(&mut self) {
        if self.is_hidden() {
            return;
        }

        self.entry.is_hidden = true;
        let mut render_indices = self.entry.render_indices.lock().expect("mutex is poisoned");
        let to_store = (*render_indices).clone();

        for entry in &mut *render_indices {
            *entry = None;
        }

        self.pending_updates
            .lock()
            .expect("mutex is poisoned")
            .push(FeatureUpdate::Delete {
                render_indices: to_store,
            });

        self.is_updated = true;
    }

    /// Shows the previously hidden feature.
    pub fn show(&mut self) {
        if !self.is_hidden() {
            return;
        }

        self.entry.is_hidden = false;

        if !self.is_updated {
            self.pending_updates
                .lock()
                .expect("poisoned mutex")
                .push(FeatureUpdate::Update {
                    feature_index: self.feature_index,
                });
        }

        self.is_updated = true;
    }
}

impl<'a, F> AsRef<F> for FeatureContainerMut<'a, F> {
    fn as_ref(&self) -> &F {
        self.entry.feature()
    }
}

impl<'a, F> AsMut<F> for FeatureContainerMut<'a, F> {
    fn as_mut(&mut self) -> &mut F {
        if !self.is_updated {
            self.pending_updates
                .lock()
                .expect("poisoned mutex")
                .push(FeatureUpdate::Update {
                    feature_index: self.feature_index,
                });
        }

        self.is_updated = true;
        &mut self.entry.feature
    }
}

#[derive(Debug)]
pub(super) enum FeatureUpdate {
    Update { feature_index: usize },
    UpdateStyle { feature_index: usize },
    Delete { render_indices: Vec<Option<usize>> },
}

impl<F> FeatureStore<F> {
    /// Creates a new store with the given feature set.
    pub fn new(features: impl Iterator<Item = F>) -> Self {
        let features: Vec<_> = features.map(|f| FeatureEntry::new(f)).collect();
        let count = features.len();
        Self {
            features,
            pending_updates: Arc::new(Mutex::new(
                (0..count)
                    .map(|feature_index| FeatureUpdate::Update { feature_index })
                    .collect(),
            )),
        }
    }

    /// Adds a new feature to the store.
    pub fn insert(&mut self, feature: F) {
        let feature_index = self.features.len();
        self.features.push(FeatureEntry::new(feature));
        self.pending_updates
            .lock()
            .expect("poisoned mutex")
            .push(FeatureUpdate::Update { feature_index })
    }

    /// Adds a new hidden feature to the store at the end of the list.
    pub fn insert_hidden(&mut self, feature: F) {
        self.features.push(FeatureEntry::hidden(feature));
    }

    /// Returns a reference to the feature. Returns `None` if a feature with the given `index` does not exist.
    pub fn get(&self, index: usize) -> Option<&F> {
        self.features.get(index).map(|f| &f.feature)
    }

    /// Returns a mutable reference to the feature. Returns `None` if a feature with the given `index` does not exist.
    pub fn get_mut(&mut self, index: usize) -> Option<FeatureContainerMut<F>> {
        self.features.get_mut(index).map(|f| FeatureContainerMut {
            entry: f,
            feature_index: index,
            is_updated: false,
            pending_updates: self.pending_updates.clone(),
        })
    }

    /// Removes the feature with the given returning the feature.
    ///
    /// # Panics
    ///
    /// Panics if a feature with the given index does not exist.
    pub fn remove(&mut self, index: usize) -> F {
        let FeatureEntry {
            feature,
            is_hidden: _is_hidden,
            render_indices,
        } = self.features.remove(index);
        self.pending_updates
            .lock()
            .expect("mutex is poisoned")
            .push(FeatureUpdate::Delete {
                render_indices: render_indices.into_inner().expect("mutex is poisoned"),
            });

        feature
    }

    pub(super) fn get_entry(&self, index: usize) -> Option<&FeatureEntry<F>> {
        self.features.get(index)
    }

    pub(super) fn drain_updates(&self) -> Vec<FeatureUpdate> {
        let mut updates = self.pending_updates.lock().expect("poisoned mutex");
        std::mem::take(&mut *updates)
    }

    /// Iterates over immutable containers of the features.
    pub fn iter(&self) -> impl Iterator<Item = FeatureContainer<F>> {
        self.features
            .iter()
            .enumerate()
            .map(|(feature_index, f)| FeatureContainer {
                feature: &f.feature,
                feature_index,
            })
    }

    /// Iterates over mutable containers of the features.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = FeatureContainerMut<F>> {
        self.features
            .iter_mut()
            .enumerate()
            .map(|(index, f)| FeatureContainerMut {
                entry: f,
                feature_index: index,
                is_updated: false,
                pending_updates: self.pending_updates.clone(),
            })
    }
}

pub(super) struct FeatureEntry<F> {
    feature: F,
    is_hidden: bool,
    render_indices: Mutex<Vec<Option<usize>>>,
}

impl<F> FeatureEntry<F> {
    fn new(feature: F) -> Self {
        Self {
            feature,
            is_hidden: false,
            render_indices: Mutex::new(vec![]),
        }
    }

    fn hidden(feature: F) -> Self {
        Self {
            feature,
            is_hidden: true,
            render_indices: Mutex::new(vec![]),
        }
    }

    pub fn feature(&self) -> &F {
        &self.feature
    }

    pub fn render_index(&self, render_store_id: usize) -> Option<usize> {
        self.render_indices
            .lock()
            .expect("mutex is poisoned")
            .get(render_store_id)
            .copied()
            .flatten()
    }

    pub fn set_render_index(&self, render_index: usize, render_store_id: usize) {
        let mut render_indices = self.render_indices.lock().expect("mutex is poisoned");

        for _ in render_indices.len()..(render_store_id + 1) {
            render_indices.push(None)
        }

        render_indices[render_store_id] = Some(render_index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;

    #[test]
    fn feature_editing() {
        let mut store = FeatureStore::default();

        store.insert(String::from("F1"));
        let pending_updates = store.drain_updates();
        assert_eq!(pending_updates.len(), 1);
        assert_matches!(
            pending_updates[0],
            FeatureUpdate::Update { feature_index: 0 }
        );

        let mut feature = store.get_mut(0).expect("no feature");

        feature.as_mut().push('2');
        let pending_updates = store.drain_updates();
        assert_eq!(pending_updates.len(), 1);
        assert_matches!(
            pending_updates[0],
            FeatureUpdate::Update { feature_index: 0 }
        );

        assert_eq!(store.get(0).expect("no feature"), &"F12".to_string());
    }
}
