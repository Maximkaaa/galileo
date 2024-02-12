pub mod url_data_provider;
pub mod url_image_provider;

#[cfg(not(target_arch = "wasm32"))]
pub mod file_cache;

use crate::error::GalileoError;
use bytes::Bytes;
use maybe_sync::{MaybeSend, MaybeSync};
use std::future::Future;

pub trait DataProvider<Key, Data, Context>: MaybeSend + MaybeSync
where
    Key: MaybeSend + MaybeSync + ?Sized,
    Context: MaybeSend + MaybeSync,
{
    fn load_raw(&self, key: &Key) -> impl Future<Output = Result<Bytes, GalileoError>> + MaybeSend;
    fn decode(&self, bytes: Bytes, context: Context) -> Result<Data, GalileoError>;
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

pub trait DataProcessor {
    type Input;
    type Output;
    type Context;

    fn process(
        &self,
        input: Self::Input,
        context: Self::Context,
    ) -> Result<Self::Output, GalileoError>;
}

pub trait PersistentCacheController<Key: ?Sized, Data> {
    fn get(&self, key: &Key) -> Option<Data>;
    fn insert(&self, key: &Key, data: &Data) -> Result<(), GalileoError>;
}

pub trait UrlSource<Key: ?Sized>: (Fn(&Key) -> String) + MaybeSend + MaybeSync {}
impl<Key: ?Sized, T: Fn(&Key) -> String> UrlSource<Key> for T where T: MaybeSend + MaybeSync {}

mod dummy {
    use crate::error::GalileoError;
    use crate::layer::data_provider::PersistentCacheController;
    use bytes::Bytes;

    #[allow(dead_code)]
    pub struct DummyCacheController {
        // Guarantees that the controller cannot be instantiated.
        private_field: u8,
    }

    impl<Key: ?Sized> PersistentCacheController<Key, Bytes> for DummyCacheController {
        fn get(&self, _key: &Key) -> Option<Bytes> {
            unreachable!()
        }

        fn insert(&self, _key: &Key, _data: &Bytes) -> Result<(), GalileoError> {
            unreachable!()
        }
    }
}
