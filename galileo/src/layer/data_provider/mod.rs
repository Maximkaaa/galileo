pub mod file_cache;
pub mod url_data_provider;
pub mod url_image_provider;

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

pub struct EmptyCache {}
impl<Key: ?Sized, Data> PersistentCacheController<Key, Data> for EmptyCache {
    fn get(&self, _key: &Key) -> Option<Data> {
        None
    }

    fn insert(&self, _key: &Key, _data: &Data) -> Result<(), GalileoError> {
        Ok(())
    }
}
