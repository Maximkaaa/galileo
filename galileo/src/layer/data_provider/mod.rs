//! Data sources for layers.

mod url_data_provider;
mod url_image_provider;

pub use url_data_provider::UrlDataProvider;
pub use url_image_provider::UrlImageProvider;

#[cfg(not(target_arch = "wasm32"))]
mod file_cache;

#[cfg(not(target_arch = "wasm32"))]
pub use file_cache::FileCacheController;

use crate::error::GalileoError;
use bytes::Bytes;
use maybe_sync::{MaybeSend, MaybeSync};
use std::future::Future;

/// Data provider is a generic way to load and decode data for a layer.
///
/// The purpose of data providers is to encapsulate the details of where the data for a layer comes from. Data providers
/// can choose to implement in-memory or persistent caching, use background threads to decode data etc.
///
/// # Generic parameters
/// * `Key` - identification of a data item.
/// * `Data` - decoded data used by a layer.
/// * `Context` - context used to decode the raw data.
pub trait DataProvider<Key, Data, Context>: MaybeSend + MaybeSync
where
    Key: MaybeSend + MaybeSync + ?Sized,
    Context: MaybeSend + MaybeSync,
{
    /// Load the raw data for the source.
    fn load_raw(&self, key: &Key) -> impl Future<Output = Result<Bytes, GalileoError>> + MaybeSend;

    /// Decode the loaded raw data.
    fn decode(&self, bytes: Bytes, context: Context) -> Result<Data, GalileoError>;

    /// Load and decode the data.
    fn load(
        &self,
        key: &Key,
        context: Context,
    ) -> impl Future<Output = Result<Data, GalileoError>> + MaybeSend {
        async {
            let raw = self.load_raw(key).await?;
            self.decode(raw, context)
        }
    }
}

/// Data processors are used to decode raw loaded data into something useful by a layer.
pub trait DataProcessor {
    /// Raw data type.
    type Input;
    /// Decoded data type.
    type Output;
    /// Context needed to decode the data.
    type Context;

    /// Decodes the data.
    fn process(
        &self,
        input: Self::Input,
        context: Self::Context,
    ) -> Result<Self::Output, GalileoError>;
}

/// Persistent cache for a data of type `Data` with a key `Key`.
pub trait PersistentCacheController<Key: ?Sized, Data> {
    /// Loads data item from the cache.
    fn get(&self, key: &Key) -> Option<Data>;
    /// Puts data item from the cache, replacing existing value if any.
    fn insert(&self, key: &Key, data: &Data) -> Result<(), GalileoError>;
}

/// Method that constructs URL address to load a data item using the data key.
pub trait UrlSource<Key: ?Sized>: (Fn(&Key) -> String) + MaybeSend + MaybeSync {}
impl<Key: ?Sized, T: Fn(&Key) -> String> UrlSource<Key> for T where T: MaybeSend + MaybeSync {}

pub(crate) mod dummy {
    use crate::error::GalileoError;
    use crate::layer::data_provider::PersistentCacheController;
    use bytes::Bytes;

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
