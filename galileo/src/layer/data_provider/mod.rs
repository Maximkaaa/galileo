//! Data sources for layers.

mod file_cache;
pub use file_cache::{remove_parameters_modifier, FileCacheController, FileCachePathModifier};
use maybe_sync::{MaybeSend, MaybeSync};

use crate::error::GalileoError;

/// Persistent cache for a data of type `Data` with a key `Key`.
pub trait PersistentCacheController<Key: ?Sized, Data>: MaybeSend + MaybeSync {
    /// Loads data item from the cache.
    fn get(&self, key: &Key) -> Option<Data>;
    /// Puts data item from the cache, replacing existing value if any.
    fn insert(&self, key: &Key, data: &Data) -> Result<(), GalileoError>;
}

/// Method that constructs URL address to load a data item using the data key.
pub trait UrlSource<Key: ?Sized>: (Fn(&Key) -> String) + MaybeSend + MaybeSync {}
impl<Key: ?Sized, T: Fn(&Key) -> String> UrlSource<Key> for T where T: MaybeSend + MaybeSync {}

pub(crate) mod dummy {
    use bytes::Bytes;

    use crate::error::GalileoError;
    use crate::layer::data_provider::PersistentCacheController;

    /// Cache controller that always misses.
    pub struct DummyCacheController {}

    impl<Key: ?Sized> PersistentCacheController<Key, Bytes> for DummyCacheController {
        fn get(&self, _key: &Key) -> Option<Bytes> {
            None
        }

        fn insert(&self, _key: &Key, _data: &Bytes) -> Result<(), GalileoError> {
            Ok(())
        }
    }
}

pub use dummy::DummyCacheController;
