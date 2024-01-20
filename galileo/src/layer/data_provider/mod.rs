pub mod file_cache;
pub mod url_data_provider;
pub mod url_image_provider;

use crate::error::GalileoError;
use maybe_sync::MaybeSend;
use std::future::Future;

pub trait DataProvider<Key, Data> {
    fn load(&self, key: &Key) -> impl Future<Output = Result<Data, GalileoError>> + MaybeSend;
}

pub trait DataDecoder {
    type Input;
    type Output;

    fn decode(&self, input: Self::Input) -> Result<Self::Output, GalileoError>;
}

pub trait PersistentCacheController<Key: ?Sized, Data> {
    fn get(&self, key: &Key) -> Option<Data>;
    fn insert(&self, key: &Key, data: &Data) -> Result<(), GalileoError>;
}
