use std::future::Future;

#[cfg(not(target_arch = "wasm32"))]
use maybe_sync::MaybeSend;

#[cfg(not(target_arch = "wasm32"))]
pub fn spawn<T>(future: T)
where
    T: Future + MaybeSend + 'static,
    T::Output: MaybeSend + 'static,
{
    tokio::spawn(future);
}

#[cfg(target_arch = "wasm32")]
pub fn spawn<T>(future: T)
where
    T: Future + 'static,
    T::Output: 'static,
{
    wasm_bindgen_futures::spawn_local(async {
        future.await;
    });
}
