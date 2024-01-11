use crate::error::GalileoError;
use crate::layer::tile_provider::TileSource;
use crate::layer::vector_tile_layer::style::VectorTileStyle;
use crate::layer::vector_tile_layer::tile_provider::{
    LockedTileStore, TileState, VectorTile, VectorTileProvider,
};
use crate::messenger::Messenger;
use crate::platform::{PlatformService, PlatformServiceImpl};
use crate::render::wgpu::WgpuRenderBundle;
use crate::render::{RenderBundle, Renderer};
use crate::tile_scheme::{TileIndex, TileScheme};
use galileo_mvt::MvtTile;
use lyon::lyon_tessellation::VertexBuffers;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use wasm_bindgen::prelude::*;

const WORKER_URL: &str = "./vt_worker.js";
const DEFAULT_WORKER_COUNT: usize = 4;
const READY_MESSAGE: f64 = 42.0;

pub struct WebWorkerVectorTileProvider {
    worker_pool: Vec<Rc<RefCell<WorkerState>>>,
    next_worker: AtomicUsize,
    tiles: Arc<RwLock<HashMap<TileIndex, TileState>>>,
    messenger: Arc<RwLock<Option<Box<dyn Messenger>>>>,
    tile_source: Box<dyn TileSource>,
    tile_scheme: TileScheme,
}

struct WorkerState {
    worker: web_sys::Worker,
    is_ready: bool,
}

impl VectorTileProvider for WebWorkerVectorTileProvider {
    fn create(
        messenger: Option<Box<dyn Messenger>>,
        tile_source: impl TileSource + 'static,
        tile_scheme: TileScheme,
    ) -> Self {
        Self::new(
            DEFAULT_WORKER_COUNT,
            messenger,
            Box::new(tile_source),
            tile_scheme,
        )
    }

    fn supports(&self, _renderer: &RwLock<dyn Renderer>) -> bool {
        // todo
        true
    }

    fn load_tile(
        &self,
        index: TileIndex,
        style: &VectorTileStyle,
        renderer: &Arc<RwLock<dyn Renderer>>,
    ) {
        self.load(index, style.clone(), renderer)
    }

    fn update_style(&self) {
        let mut tiles = self.tiles.write().unwrap();
        for (_, tile_state) in tiles.iter_mut() {
            if matches!(tile_state, TileState::Loaded(_)) {
                let TileState::Loaded(tile) = std::mem::replace(tile_state, TileState::Error)
                else {
                    panic!("Type of value changed unexpectingly");
                };
                *tile_state = TileState::Outdated(tile);
            }
        }

        if let Some(messenger) = &(*self.messenger.read().unwrap()) {
            messenger.request_redraw();
        }
    }

    fn read(&self) -> LockedTileStore {
        LockedTileStore {
            guard: self.tiles.read().unwrap(),
        }
    }

    fn set_messenger(&self, messenger: Box<dyn Messenger>) {
        *self.messenger.write().unwrap() = Some(messenger.into())
    }
}

impl WebWorkerVectorTileProvider {
    pub fn new(
        pool_size: usize,
        messenger: Option<Box<dyn Messenger>>,
        tile_source: Box<dyn TileSource>,
        tile_scheme: TileScheme,
    ) -> Self {
        let mut provider = Self {
            worker_pool: Vec::with_capacity(pool_size),
            next_worker: Default::default(),
            tiles: Arc::new(RwLock::new(HashMap::new())),
            messenger: Arc::new(RwLock::new(messenger)),
            tile_source,
            tile_scheme,
        };

        for _ in 0..pool_size {
            provider.spawn_worker();
        }

        provider
    }

    fn load(&self, index: TileIndex, style: VectorTileStyle, renderer: &Arc<RwLock<dyn Renderer>>) {
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

        {
            let mut tiles = self.tiles.write().unwrap();
            match tiles.get_mut(&index) {
                None => {
                    tiles.insert(index, TileState::Loading(renderer.clone()));
                }
                Some(state @ TileState::Outdated(..)) => {
                    let TileState::Outdated(tile) = std::mem::replace(state, TileState::Error)
                    else {
                        panic!("Type of value changed unexpectingly");
                    };
                    *state = TileState::Updating(tile, renderer.clone());
                }
                _ => {
                    return;
                }
            }
        }

        let url = (*self.tile_source)(index);
        loop {
            let worker_index =
                self.next_worker.fetch_add(1, Ordering::Relaxed) % self.worker_pool.len();
            let state = self.worker_pool[worker_index].borrow();
            if !state.is_ready {
                continue;
            }

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

                let result: Result<WorkerOutput, (TileIndex, String)> =
                    serde_wasm_bindgen::from_value(event.data()).unwrap();
                let mut store = tiles_store.write().unwrap();
                match result {
                    Ok(WorkerOutput {
                        index,
                        mvt_tile,
                        vertices,
                        indices,
                    }) => match store.get(&index) {
                        Some(TileState::Loading(renderer) | TileState::Updating(_, renderer)) => {
                            let mut bundle = WgpuRenderBundle::new();

                            let vertices_cast = bytemuck::cast_slice(&vertices).into();
                            let indices_cast = bytemuck::cast_slice(&indices).into();
                            bundle.vertex_buffers.vertices = vertices_cast;
                            bundle.vertex_buffers.indices = indices_cast;
                            let mvt_tile = bincode::deserialize(&mvt_tile).unwrap();

                            let packed = renderer.read().unwrap().pack_bundle(Box::new(bundle));
                            store.insert(
                                index,
                                TileState::Loaded(VectorTile {
                                    bundle: packed,
                                    mvt_tile,
                                }),
                            );

                            log::info!("Tile {:?} is stored", index);
                            if let Some(messenger) = &(*messenger.read().unwrap()) {
                                messenger.request_redraw();
                            }
                        }
                        _ => {
                            store.insert(index, TileState::Error);

                            log::info!("Tile {:?} loaded, but is not needed anymore", index);
                        }
                    },
                    Err((index, message)) => {
                        log::info!("Failed to load tile {index:?}: {message}");
                        store.insert(index, TileState::Error);
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

#[derive(Debug, Serialize, Deserialize)]
struct WorkerOutput {
    index: TileIndex,
    #[serde(with = "serde_bytes")]
    mvt_tile: Vec<u8>,
    #[serde(with = "serde_bytes")]
    vertices: Vec<u8>,
    #[serde(with = "serde_bytes")]
    indices: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
#[wasm_bindgen]
pub struct LoadTilePayload {
    index: TileIndex,
    url: String,
    style: VectorTileStyle,
    tile_scheme: TileScheme,
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

#[wasm_bindgen]
pub async fn load_tile(data: JsValue) -> JsValue {
    let payload: LoadTilePayload = serde_wasm_bindgen::from_value(data).unwrap();
    let index = payload.index;
    let result = try_load_tile(payload)
        .await
        .map_err(|err| (index, format!("{err:?}")));

    serde_wasm_bindgen::to_value(&result).unwrap()
}

async fn try_load_tile(payload: LoadTilePayload) -> Result<WorkerOutput, GalileoError> {
    let mvt_tile = download_tile(payload.index, &payload.url).await?;
    let mut bundle: Box<dyn RenderBundle> = Box::new(WgpuRenderBundle::new());
    VectorTile::prepare(
        &mvt_tile,
        &mut bundle,
        payload.index,
        &payload.style,
        &payload.tile_scheme,
    )?;

    let bundle: Box<WgpuRenderBundle> = bundle.into_any().downcast().unwrap();
    let VertexBuffers { vertices, indices } = bundle.vertex_buffers;
    let vertices_bytes: Vec<u8> = bytemuck::cast_slice(&vertices[..]).into();
    let index_bytes: Vec<u8> = bytemuck::cast_slice(&indices[..]).into();

    let tile_bytes = bincode::serialize(&mvt_tile).unwrap();

    Ok(WorkerOutput {
        index: payload.index,
        vertices: vertices_bytes,
        indices: index_bytes,
        mvt_tile: tile_bytes,
    })
}

async fn download_tile(index: TileIndex, url: &str) -> Result<MvtTile, GalileoError> {
    let platform_service = PlatformServiceImpl::new();
    match platform_service.load_bytes_from_url(url).await {
        Ok(bytes) => {
            let mvt_tile = match MvtTile::decode(bytes, false) {
                Ok(v) => v,
                Err(e) => {
                    log::info!("Failed to decode tile {index:?}: {e:?}");
                    return Err(GalileoError::IO);
                }
            };

            Ok(mvt_tile)
        }
        Err(e) => {
            log::info!("Failed to load tile {index:?}");
            Err(e)
        }
    }
}
