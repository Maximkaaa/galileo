#[cfg(feature = "tokio")]
use maybe_sync::MaybeSend;
use std::future::Future;

#[cfg(feature = "tokio")]
pub fn spawn<T>(future: T)
where
    T: Future + MaybeSend + 'static,
    T::Output: MaybeSend + 'static,
{
    tokio::spawn(future);
}

#[cfg(feature = "web")]
pub fn spawn<T>(future: T)
where
    T: Future + 'static,
    T::Output: 'static,
{
    wasm_bindgen_futures::spawn_local(async {
        future.await;
    });
}
