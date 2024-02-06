use crate::layer::Layer;
use std::ops::{Index, IndexMut, RangeBounds};

/// Collection of layers with some meta-information.
///
/// When a map is rendered, it draws all visible layers in the order they are stored in the
/// collection. Any layer can be temporary hidden with the [`LayerCollection::hide`] or
/// [`LayerCollection::show_by`] methods. These layers will be ignored by the renderer, but
/// retain their place in the collection.
///
/// Since a map should be able to render anything implementing the [`Layer`] trait, this
/// collection stores layers as trait objects. You can use downcasting through `Any` trait
/// to obtain a concrete layer type you work with.
///
/// ```no_run
/// use galileo::galileo_map::VectorTileProvider;
/// use galileo::layer::{RasterTileLayer, VectorTileLayer};
/// use galileo::map::layer_collection::LayerCollection;
/// use galileo::MapBuilder;
///
/// let raster_tiles = MapBuilder::create_raster_tile_layer(|index| format!("url from {index:?}"), todo!());
/// let vector_tiles = MapBuilder::create_vector_tile_layer(|index| format!("url from {index:?}"), todo!(), todo!());
///
/// let mut collection = LayerCollection::default();
/// collection.push(raster_tiles);
/// collection.push(vector_tiles);
///
/// assert!(collection[1].as_any().downcast_ref::<VectorTileLayer<VectorTileProvider>>().is_some());
/// ```
#[derive(Default)]
pub struct LayerCollection(Vec<LayerEntry>);

struct LayerEntry {
    layer: Box<dyn Layer>,
    is_hidden: bool,
}

impl LayerCollection {
    /// Shortens the collection, keeping the first `length` layers and dropping the rest. If
    /// the length of the collection is less than `length` does nothing.
    ///
    /// # Examples
    ///
    /// ```
    /// use galileo::map::layer_collection::LayerCollection;
    /// use galileo::layer::TestLayer;
    ///
    /// let mut collection = LayerCollection::from(vec![
    ///     TestLayer("Layer A"),
    ///     TestLayer("Layer B"),
    /// ]);
    ///
    /// collection.truncate(3);
    /// assert_eq!(collection.len(), 2);
    /// collection.truncate(1);
    /// assert_eq!(collection.len(), 1);
    /// assert_eq!(collection[0].as_any().downcast_ref(), Some(&TestLayer("Layer A")));
    /// ```
    pub fn truncate(&mut self, length: usize) {
        self.0.truncate(length)
    }

    /// Removes all layers from the collection.
    ///
    /// # Examples
    ///
    /// ```
    /// use galileo::map::layer_collection::LayerCollection;
    /// use galileo::layer::TestLayer;
    ///
    /// let mut collection = LayerCollection::from(vec![
    ///     TestLayer("Layer A"),
    ///     TestLayer("Layer B"),
    /// ]);
    ///
    /// collection.clear();
    /// assert_eq!(collection.len(), 0);
    /// ```
    pub fn clear(&mut self) {
        self.0.clear()
    }

    /// Removes a layer from the collection and returns it. The removed element is replaced by the
    /// last layer in the collection.
    ///
    /// # Panics
    ///
    /// Panics if `index` equals or greater then collection length.
    ///
    /// # Examples
    ///
    /// ```
    /// use galileo::map::layer_collection::LayerCollection;
    /// use galileo::layer::TestLayer;
    ///
    /// let mut collection = LayerCollection::from(vec![
    ///     TestLayer("Layer A"),
    ///     TestLayer("Layer B"),
    ///     TestLayer("Layer C"),
    /// ]);
    ///
    /// let removed = collection.swap_remove(0);
    /// assert_eq!(removed.as_any().downcast_ref(), Some(&TestLayer("Layer A")));
    /// assert_eq!(collection[0].as_any().downcast_ref(), Some(&TestLayer("Layer C")));
    /// ```
    pub fn swap_remove(&mut self, index: usize) -> Box<dyn Layer> {
        self.0.swap_remove(index).layer
    }

    /// Inserts a layer at position `index`, shifting all layers after it to the right.
    ///
    /// # Panics
    ///
    /// Panics if `index > len`
    ///
    /// # Examples
    ///
    /// ```
    /// use galileo::map::layer_collection::LayerCollection;
    /// use galileo::layer::TestLayer;
    ///
    /// let mut collection = LayerCollection::from(vec![
    ///     TestLayer("Layer A"),
    ///     TestLayer("Layer B"),
    /// ]);
    ///
    /// collection.insert(1, TestLayer("Layer C"));
    /// assert_eq!(collection.len(), 3);
    /// assert_eq!(collection[1].as_any().downcast_ref(), Some(&TestLayer("Layer C")));
    /// assert_eq!(collection[2].as_any().downcast_ref(), Some(&TestLayer("Layer B")));
    pub fn insert(&mut self, index: usize, layer: impl Layer + 'static) {
        self.0.insert(index, layer.into());
    }

    /// Removes a layer at `index`, shifting all layers after it to the left and returning the
    /// removed layer.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use galileo::map::layer_collection::LayerCollection;
    /// use galileo::layer::TestLayer;
    ///
    /// let mut collection = LayerCollection::from(vec![
    ///     TestLayer("Layer A"),
    ///     TestLayer("Layer B"),
    ///     TestLayer("Layer C"),
    /// ]);
    ///
    /// let removed = collection.remove(1);
    /// assert_eq!(removed.as_any().downcast_ref(), Some(&TestLayer("Layer B")));
    /// assert_eq!(collection.len(), 2);
    /// assert_eq!(collection[1].as_any().downcast_ref(), Some(&TestLayer("Layer C")));
    /// ```
    pub fn remove(&mut self, index: usize) -> Box<dyn Layer> {
        self.0.remove(index).layer
    }

    /// Retains only the layers specified by the predicate. In other words, remove all layers `l`
    /// for which f(&l) returns false.
    ///
    /// # Examples
    ///
    /// ```
    /// use galileo::map::layer_collection::LayerCollection;
    /// use galileo::layer::TestLayer;
    ///
    /// let mut collection = LayerCollection::from(vec![
    ///     TestLayer("Layer A"),
    ///     TestLayer("Layer B"),
    ///     TestLayer("Layer C"),
    /// ]);
    ///
    /// collection.retain(|layer| !layer.as_any().downcast_ref::<TestLayer>().is_some_and(|l| l.0.ends_with("A")));
    ///
    /// assert_eq!(collection.len(), 2);
    /// assert_eq!(collection[0].as_any().downcast_ref(), Some(&TestLayer("Layer B")));
    /// assert_eq!(collection[1].as_any().downcast_ref(), Some(&TestLayer("Layer C")));
    /// ```
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&dyn Layer) -> bool,
    {
        self.0.retain(|entry| f(&*entry.layer))
    }

    /// Adds the layer to the end of the collection.
    ///
    /// # Examples
    ///
    /// ```
    /// use galileo::map::layer_collection::LayerCollection;
    /// use galileo::layer::TestLayer;
    ///
    /// let mut collection = LayerCollection::from(vec![
    ///     TestLayer("Layer A"),
    ///     TestLayer("Layer B"),
    /// ]);
    ///
    /// collection.push(TestLayer("Layer C"));
    ///
    /// assert_eq!(collection.len(), 3);
    /// assert_eq!(collection[2].as_any().downcast_ref(), Some(&TestLayer("Layer C")));
    /// ```
    pub fn push(&mut self, layer: impl Layer + 'static) {
        self.0.push(layer.into())
    }

    /// Removes the last layer from the collection and returns it. Returns `None` if the collection
    /// is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use galileo::map::layer_collection::LayerCollection;
    /// use galileo::layer::TestLayer;
    ///
    /// let mut collection = LayerCollection::from(vec![
    ///     TestLayer("Layer A"),
    ///     TestLayer("Layer B"),
    ///     TestLayer("Layer C"),
    /// ]);
    ///
    /// let removed = collection.pop();
    ///
    /// assert_eq!(collection.len(), 2);
    /// assert_eq!(removed.unwrap().as_any().downcast_ref(), Some(&TestLayer("Layer C")));
    /// ```
    pub fn pop(&mut self) -> Option<Box<dyn Layer>> {
        self.0.pop().map(|entry| entry.layer)
    }

    /// Removes the specified range of layers from the collection in bulk, returning all removed
    /// layers in an iterator. If the iterator is dropped before being fully consumed, it drops
    /// the remaining removed layers.
    ///
    /// # Panics
    ///
    /// Panics if the starting point is greater than the end point and if the end point is
    /// greater that the length of the collection.
    ///
    /// # Examples
    ///
    /// ```
    /// use galileo::map::layer_collection::LayerCollection;
    /// use galileo::layer::TestLayer;
    ///
    /// let mut collection = LayerCollection::from(vec![
    ///     TestLayer("Layer A"),
    ///     TestLayer("Layer B"),
    ///     TestLayer("Layer C"),
    /// ]);
    ///
    /// let drained: Vec<_> = collection.drain(0..2).collect();
    /// assert_eq!(drained.len(), 2);
    /// assert_eq!(drained[1].as_any().downcast_ref(), Some(&TestLayer("Layer B")));
    ///
    /// assert_eq!(collection.len(), 1);
    /// assert_eq!(collection[0].as_any().downcast_ref(), Some(&TestLayer("Layer C")));
    /// ```
    pub fn drain<R>(&mut self, range: R) -> impl Iterator<Item = Box<dyn Layer>> + '_
    where
        R: RangeBounds<usize>,
    {
        self.0.drain(range).map(|entry| entry.layer)
    }

    /// Returns the count of layers in the collection.
    ///
    /// # Examples
    ///
    /// ```
    /// use galileo::map::layer_collection::LayerCollection;
    /// use galileo::layer::TestLayer;
    ///
    /// let collection = LayerCollection::from(vec![
    ///     TestLayer("Layer A"),
    ///     TestLayer("Layer B"),
    /// ]);
    ///
    /// assert_eq!(collection.len(), 2);
    /// ```
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the collection contains zero layers.
    ///
    /// # Examples
    ///
    /// ```
    /// use galileo::map::layer_collection::LayerCollection;
    /// use galileo::layer::TestLayer;
    ///
    /// let mut collection = LayerCollection::default();
    /// assert!(collection.is_empty());
    ///
    /// collection.push(TestLayer("Layer A"));
    /// assert!(!collection.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns a layer at `index`, or `None` if index is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use galileo::map::layer_collection::LayerCollection;
    /// use galileo::layer::TestLayer;
    ///
    /// let collection = LayerCollection::from(vec![
    ///     TestLayer("Layer A"),
    ///     TestLayer("Layer B"),
    /// ]);
    ///
    /// assert_eq!(collection.get(1).and_then(|layer| layer.as_any().downcast_ref()), Some(&TestLayer("Layer B")));
    /// assert!(collection.get(2).is_none());
    /// ```
    pub fn get(&self, index: usize) -> Option<&dyn Layer> {
        self.0.get(index).map(|entry| &*entry.layer)
    }

    /// Returns a mutable reference to a layer at `index`, or `None` if index is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use galileo::map::layer_collection::LayerCollection;
    /// use galileo::layer::TestLayer;
    ///
    /// let mut collection = LayerCollection::from(vec![
    ///     TestLayer("Layer A"),
    ///     TestLayer("Layer B"),
    /// ]);
    ///
    /// assert_eq!(collection.get_mut(1).and_then(|layer| layer.as_any_mut().downcast_ref()), Some(&TestLayer("Layer B")));
    /// assert!(collection.get(2).is_none());
    /// ```
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Box<dyn Layer>> {
        self.0.get_mut(index).map(|entry| &mut entry.layer)
    }

    /// Swaps two layers in the collection.
    ///
    /// # Panics
    ///
    /// Panics if `a` or `b` are out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use galileo::map::layer_collection::LayerCollection;
    /// use galileo::layer::TestLayer;
    ///
    /// let mut collection = LayerCollection::from(vec![
    ///     TestLayer("Layer A"),
    ///     TestLayer("Layer B"),
    ///     TestLayer("Layer C"),
    /// ]);
    ///
    /// collection.swap(1, 2);
    ///
    /// assert_eq!(collection[1].as_any().downcast_ref(), Some(&TestLayer("Layer C")));
    /// assert_eq!(collection[2].as_any().downcast_ref(), Some(&TestLayer("Layer B")));
    /// ```
    pub fn swap(&mut self, a: usize, b: usize) {
        self.0.swap(a, b)
    }

    /// Iterates over all layers in the collection.
    ///
    /// ```
    /// use galileo::map::layer_collection::LayerCollection;
    /// use galileo::layer::TestLayer;
    ///
    /// let collection = LayerCollection::from(vec![
    ///     TestLayer("Layer A"),
    ///     TestLayer("Layer B"),
    /// ]);
    ///
    /// let mut iterator = collection.iter();
    /// assert_eq!(iterator.next().and_then(|layer| layer.as_any().downcast_ref()), Some(&TestLayer("Layer A")));
    /// assert_eq!(iterator.next().and_then(|layer| layer.as_any().downcast_ref()), Some(&TestLayer("Layer B")));
    /// assert!(iterator.next().is_none());
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = &dyn Layer> + '_ {
        self.0.iter().map(|entry| &*entry.layer)
    }

    /// Iterates over mutable references to all layers in the collection.
    ///
    /// ```
    /// use galileo::map::layer_collection::LayerCollection;
    /// use galileo::layer::TestLayer;
    ///
    /// let mut collection = LayerCollection::from(vec![
    ///     TestLayer("Layer A"),
    ///     TestLayer("Layer B"),
    /// ]);
    ///
    /// let mut iterator = collection.iter_mut();
    /// assert_eq!(iterator.next().and_then(|layer| layer.as_any_mut().downcast_ref()), Some(&TestLayer("Layer A")));
    /// assert_eq!(iterator.next().and_then(|layer| layer.as_any_mut().downcast_ref()), Some(&TestLayer("Layer B")));
    /// assert!(iterator.next().is_none());
    /// ```
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Box<dyn Layer>> + '_ {
        self.0.iter_mut().map(|entry| &mut entry.layer)
    }

    /// Sets the layer at `index` as invisible. The hidden layer can be later shown with
    /// [`LayerCollection::show`].
    ///
    /// Hidden layers are stored in the layer collection, but are not rendered to a map.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use galileo::map::layer_collection::LayerCollection;
    /// use galileo::layer::TestLayer;
    ///
    /// let mut collection = LayerCollection::from(vec![
    ///     TestLayer("Layer A"),
    ///     TestLayer("Layer B"),
    /// ]);
    ///
    /// collection.hide(1);
    /// assert!(!collection.is_visible(1));
    /// ```
    pub fn hide(&mut self, index: usize) {
        self.0[index].is_hidden = true;
    }

    /// Sets the layer at `index` as visible.
    ///
    /// Hidden layers are stored in the layer collection, but are not rendered to a map.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use galileo::map::layer_collection::LayerCollection;
    /// use galileo::layer::TestLayer;
    ///
    /// let mut collection = LayerCollection::from(vec![
    ///     TestLayer("Layer A"),
    ///     TestLayer("Layer B"),
    /// ]);
    ///
    /// collection.hide(1);
    /// collection.show(1);
    /// assert!(collection.is_visible(1));
    /// ```
    pub fn show(&mut self, index: usize) {
        self.0[index].is_hidden = false;
    }

    /// Sets all layers for which the predicate returns true as visible. The rest of layers are set
    /// as hidden.
    ///
    /// # Examples
    ///
    /// ```
    /// use galileo::map::layer_collection::LayerCollection;
    /// use galileo::layer::TestLayer;
    ///
    /// let mut collection = LayerCollection::from(vec![
    ///     TestLayer("Layer A"),
    ///     TestLayer("Layer B"),
    ///     TestLayer("Layer C"),
    /// ]);
    ///
    /// collection.show_by(|layer| layer.as_any().downcast_ref::<TestLayer>().unwrap().0.ends_with("B"));
    ///
    /// assert!(!collection.is_visible(0));
    /// assert!(collection.is_visible(1));
    /// assert!(!collection.is_visible(2));
    pub fn show_by<F>(&mut self, mut f: F)
    where
        F: FnMut(&dyn Layer) -> bool,
    {
        for entry in &mut self.0 {
            entry.is_hidden = !f(&*entry.layer);
        }
    }

    /// Returns true, if the layer at `index` is not hidden.
    ///
    /// Hidden layers are stored in the layer collection, but are not rendered to a map.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use galileo::map::layer_collection::LayerCollection;
    /// use galileo::layer::TestLayer;
    ///
    /// let mut collection = LayerCollection::from(vec![
    ///     TestLayer("Layer A"),
    ///     TestLayer("Layer B"),
    /// ]);
    ///
    /// assert!(collection.is_visible(1));
    /// collection.hide(1);
    /// assert!(!collection.is_visible(1));
    /// collection.show(1);
    /// assert!(collection.is_visible(1));
    /// ```
    pub fn is_visible(&self, index: usize) -> bool {
        !self.0[index].is_hidden
    }

    /// Iterates over all visible layers in the collection.
    ///
    /// # Examples
    ///
    /// ```
    /// use galileo::map::layer_collection::LayerCollection;
    /// use galileo::layer::TestLayer;
    ///
    /// let mut collection = LayerCollection::from(vec![
    ///     TestLayer("Layer A"),
    ///     TestLayer("Layer B"),
    ///     TestLayer("Layer C"),
    /// ]);
    ///
    /// collection.hide(1);
    ///
    /// let mut iterator = collection.iter_visible();
    /// assert_eq!(iterator.next().and_then(|layer| layer.as_any().downcast_ref()), Some(&TestLayer("Layer A")));
    /// assert_eq!(iterator.next().and_then(|layer| layer.as_any().downcast_ref()), Some(&TestLayer("Layer C")));
    /// assert!(iterator.next().is_none());
    /// ```
    pub fn iter_visible(&self) -> impl Iterator<Item = &dyn Layer> + '_ {
        self.0
            .iter()
            .filter(|entry| !entry.is_hidden)
            .map(|entry| &*entry.layer)
    }
}

impl Index<usize> for LayerCollection {
    type Output = dyn Layer;

    fn index(&self, index: usize) -> &Self::Output {
        &*self.0[index].layer
    }
}

impl IndexMut<usize> for LayerCollection {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut *self.0[index].layer
    }
}

impl<L: Into<LayerEntry>, T: IntoIterator<Item = L>> From<T> for LayerCollection {
    fn from(value: T) -> Self {
        Self(value.into_iter().map(|layer| layer.into()).collect())
    }
}

impl<T: Layer + 'static> From<T> for LayerEntry {
    fn from(value: T) -> Self {
        Self {
            layer: Box::new(value),
            is_hidden: false,
        }
    }
}

impl From<Box<dyn Layer>> for LayerEntry {
    fn from(value: Box<dyn Layer>) -> Self {
        Self {
            layer: value,
            is_hidden: false,
        }
    }
}
