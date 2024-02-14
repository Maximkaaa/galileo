use crate::error::GalileoError;
use crate::layer::data_provider::UrlDataProvider;
use crate::layer::data_provider::{DataProvider, UrlSource};
use crate::layer::vector_tile_layer::style::VectorTileStyle;
use crate::layer::vector_tile_layer::tile_provider::vt_processor::{
    VectorTileDecodeContext, VtProcessor,
};
use crate::layer::vector_tile_layer::tile_provider::{
    LockedTileStore, TileState, UnpackedVectorTile, VectorTileProvider,
};
use crate::messenger::Messenger;
use crate::render::render_bundle::tessellating::serialization::TessellatingRenderBundleBytes;
use crate::render::render_bundle::tessellating::TessellatingRenderBundle;
use crate::render::render_bundle::{RenderBundle, RenderBundleType};
use crate::tile_scheme::{TileIndex, TileSchema};
use galileo_mvt::MvtTile;
use quick_cache::unsync::Cache;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use wasm_bindgen::prelude::*;

const WORKER_URL: &str = "./vt_worker.js";
const READY_MESSAGE: f64 = 42.0;

/// Vector tile provider that uses web workers to download and decode the tiles.
pub struct WebWorkerVectorTileProvider {
    worker_pool: Vec<Rc<RefCell<WorkerState>>>,
    next_worker: AtomicUsize,
    tiles: Arc<Mutex<Cache<TileIndex, TileState>>>,
    messenger: Arc<RwLock<Option<Box<dyn Messenger>>>>,
    tile_source: Box<dyn UrlSource<TileIndex>>,
    tile_scheme: TileSchema,
}

struct WorkerState {
    worker: web_sys::Worker,
    is_ready: bool,
}

impl VectorTileProvider for WebWorkerVectorTileProvider {
    fn load_tile(&self, index: TileIndex, style: &VectorTileStyle) {
        self.load(index, style.clone())
    }

    fn update_style(&self) {
        let mut tiles = self.tiles.lock().expect("tile store mutex is poisoned");
        let indices: Vec<_> = tiles.iter().map(|(index, _)| index.clone()).collect();

        for index in indices {
            let Some(mut entry) = tiles.get_mut(&index) else {
                continue;
            };
            let tile_state = &mut *entry;
            if matches!(*tile_state, TileState::Packed(_)) {
                let TileState::Packed(tile) = std::mem::replace(tile_state, TileState::Error)
                else {
                    log::error!("Type of value changed unexpectedly during updating style.");
                    continue;
                };

                *tile_state = TileState::Outdated(tile);
            }
        }

        if let Some(messenger) = &(*self.messenger.read().unwrap()) {
            messenger.request_redraw();
        }
    }

    fn read(&self) -> LockedTileStore {
        let guard = self.tiles.lock().expect("tile store mutex is poisoned");
        LockedTileStore { guard }
    }

    fn set_messenger(&self, messenger: Box<dyn Messenger>) {
        *self.messenger.write().unwrap() = Some(messenger.into())
    }
}

impl WebWorkerVectorTileProvider {
    /// Creates a new provider with `pool_size` workers.
    pub fn new(
        pool_size: usize,
        messenger: Option<Box<dyn Messenger>>,
        source: impl UrlSource<TileIndex> + 'static,
        tile_scheme: TileSchema,
    ) -> Self {
        let mut provider = Self {
            worker_pool: Vec::with_capacity(pool_size),
            next_worker: Default::default(),
            tiles: Arc::new(Mutex::new(Cache::new(1000))),
            messenger: Arc::new(RwLock::new(messenger)),
            tile_source: Box::new(source),
            tile_scheme,
        };

        for _ in 0..pool_size {
            provider.spawn_worker();
        }

        provider
    }

    fn load(&self, index: TileIndex, style: VectorTileStyle) {
        if self
            .worker_pool
            .iter()
            .filter(|v| v.borrow().is_ready)
            .count()
            == 0
        {
            log::warn!("Web worker pool is not initialized!");
            return;
        }

        if !self.set_loading_state(index) {
            return;
        }

        let url = (*self.tile_source)(&index);
        loop {
            let worker_index =
                self.next_worker.fetch_add(1, Ordering::Relaxed) % self.worker_pool.len();
            let state = self.worker_pool[worker_index].borrow();
            if !state.is_ready {
                continue;
            }

            log::info!("Requesting loading a tile");
            let payload = LoadTilePayload {
                index,
                url,
                style,
                tile_scheme: self.tile_scheme.clone(),
            };

            state
                .worker
                .post_message(&serde_wasm_bindgen::to_value(&payload).unwrap())
                .unwrap();
            return;
        }
    }

    fn set_loading_state(&self, index: TileIndex) -> bool {
        let mut tiles = self.tiles.lock().expect("tile store mutex is poisoned");
        let has_entry = tiles.peek(&index).is_some();
        if has_entry {
            if let Some(mut entry) = tiles.get_mut(&index) {
                let value = &mut *entry;
                if !matches!(value, TileState::Outdated(..)) {
                    return false;
                }

                let TileState::Outdated(tile) = std::mem::replace(value, TileState::Error) else {
                    log::error!("Type of value changed unexpectedly during loading.");
                    return false;
                };

                *value = TileState::Updating(tile);
            }
        } else {
            tiles.insert(index, TileState::Loading);
        }

        true
    }

    fn spawn_worker(&mut self) {
        let worker = web_sys::Worker::new(WORKER_URL).unwrap();
        let worker_state = Rc::new(RefCell::new(WorkerState {
            worker,
            is_ready: false,
        }));
        let worker_clone = worker_state.clone();

        let tiles_store = self.tiles.clone();
        let messenger = self.messenger.clone();
        let callback: Closure<dyn FnMut(web_sys::MessageEvent)> =
            Closure::new(move |event: web_sys::MessageEvent| {
                if let Some(message) = event.data().as_f64() {
                    if message == READY_MESSAGE {
                        worker_clone.borrow_mut().is_ready = true;
                        if let Some(messenger) = &(*messenger.read().unwrap()) {
                            messenger.request_redraw();
                        }

                        log::info!("Received worker ready message");
                    }

                    return;
                }

                let worker_output = match serde_wasm_bindgen::from_value(event.data()) {
                    Ok(v) => v,
                    Err(err) => {
                        log::warn!("Failed to deserialize worker message: {err:?}");
                        return;
                    }
                };

                match worker_output {
                    WorkerOutput::VectorTile(result) => {
                        let mut store = tiles_store.lock().expect("tiles mutex is poisoned");
                        match result {
                            Ok(decoded_vector_tile) => {
                                store_vector_tile(decoded_vector_tile, &mut store, &messenger)
                            }
                            Err((index, message)) => {
                                log::info!("Failed to load tile {index:?}: {message}");
                                store.insert(index, TileState::Error);
                            }
                        }
                    }
                }
            });

        worker_state
            .borrow_mut()
            .worker
            .set_onmessage(Some(callback.as_ref().unchecked_ref()));
        self.worker_pool.push(worker_state);

        log::info!("Vt worker started");
        callback.forget();
    }
}

fn store_vector_tile(
    decoded_vector_tile: DecodedVectorTile,
    store: &mut Cache<TileIndex, TileState>,
    messenger: &Arc<RwLock<Option<Box<dyn Messenger>>>>,
) {
    log::info!("Storing tile");
    let DecodedVectorTile {
        index,
        mvt_tile,
        bundle_bytes,
    } = decoded_vector_tile;
    match store.get(&index) {
        Some(TileState::Loading | TileState::Updating(_)) => {
            let converted: TessellatingRenderBundleBytes =
                bincode::deserialize(&bundle_bytes).unwrap();
            let bundle = RenderBundle(RenderBundleType::Tessellating(
                TessellatingRenderBundle::from_bytes_unchecked(converted),
            ));
            let mvt_tile = MvtTile::decode(bytes::Bytes::from(mvt_tile), true).unwrap();
            store.insert(
                index,
                TileState::Loaded(Box::new(UnpackedVectorTile { bundle, mvt_tile })),
            );

            if let Some(messenger) = &(*messenger.read().unwrap()) {
                messenger.request_redraw();
            }
        }
        _ => {}
    }
}

#[derive(Debug, Serialize, Deserialize)]
enum WorkerOutput {
    VectorTile(Result<DecodedVectorTile, (TileIndex, String)>),
}

#[derive(Debug, Serialize, Deserialize)]
struct DecodedVectorTile {
    index: TileIndex,
    #[serde(with = "serde_bytes")]
    mvt_tile: Vec<u8>,
    #[serde(with = "serde_bytes")]
    bundle_bytes: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoadTilePayload {
    index: TileIndex,
    url: String,
    style: VectorTileStyle,
    tile_scheme: TileSchema,
}

#[wasm_bindgen]
pub fn init_vt_worker() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Info).expect("Couldn't init logger");

    log::info!("Vt worker is initialized");

    js_sys::global()
        .dyn_into::<web_sys::DedicatedWorkerGlobalScope>()
        .unwrap()
        .post_message(&JsValue::from_f64(READY_MESSAGE))
        .unwrap();
}

fn vt_data_provider() -> UrlDataProvider<str, VtProcessor> {
    UrlDataProvider::new(|v: &str| v.to_string(), VtProcessor {})
}

#[wasm_bindgen]
pub async fn load_tile(data: JsValue) -> JsValue {
    let payload: LoadTilePayload = serde_wasm_bindgen::from_value(data).unwrap();
    let index = payload.index;
    let result = try_load_tile(payload)
        .await
        .map_err(|err| (index, format!("{err:?}")));

    let output = WorkerOutput::VectorTile(result);
    serde_wasm_bindgen::to_value(&output).unwrap()
}

async fn try_load_tile(payload: LoadTilePayload) -> Result<DecodedVectorTile, GalileoError> {
    let data_provider = vt_data_provider();
    let context = VectorTileDecodeContext {
        index: payload.index,
        style: payload.style,
        tile_schema: payload.tile_scheme,
        bundle: RenderBundle(RenderBundleType::Tessellating(
            TessellatingRenderBundle::new(),
        )),
    };

    let bytes = data_provider.load_raw(&payload.url).await?;
    let (bundle, _) = data_provider.decode(bytes.clone(), context)?;
    let RenderBundleType::Tessellating(bundle) = bundle.0;

    let serialized = bincode::serialize(&bundle.into_bytes()).unwrap();

    Ok(DecodedVectorTile {
        index: payload.index,
        mvt_tile: bytes.to_vec(),
        bundle_bytes: serialized,
    })
}
