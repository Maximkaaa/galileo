//! Operations with Web Workers.

use crate::layer::vector_tile_layer::style::VectorTileStyle;
use crate::layer::vector_tile_layer::tile_provider::processor::TileProcessingError;
use crate::render::render_bundle::tessellating::serialization::TessellatingRenderBundleBytes;
use crate::render::render_bundle::tessellating::TessellatingRenderBundle;
use crate::render::render_bundle::{RenderBundle, RenderBundleType};
use crate::tile_scheme::TileIndex;
use crate::TileSchema;
use futures::channel::oneshot;
use futures::channel::oneshot::Sender;
use galileo_mvt::MvtTile;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Display;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::watch::Receiver;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

const WORKER_URL: &str = "./vt_worker.js";

struct WorkerState {
    worker: web_sys::Worker,
    is_ready: AtomicBool,
}

type WwSender = Sender<Result<WebWorkerResponsePayload, WebWorkerError>>;

/// Service for communicating with Web Workers.
pub struct WebWorkerService {
    worker_pool: Vec<Rc<WorkerState>>,
    next_worker: AtomicUsize,
    pending_requests: Rc<RefCell<HashMap<WebWorkerRequestId, WwSender>>>,
    is_ready: RefCell<Receiver<bool>>,
}

const INVALID_ID: u64 = 0;
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Hash)]
struct WebWorkerRequestId(u64);

impl Display for WebWorkerRequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl WebWorkerRequestId {
    fn next() -> Self {
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }

    fn empty() -> Self {
        Self(INVALID_ID)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct WebWorkerRequest {
    request_id: WebWorkerRequestId,
    payload: WebWorkerRequestPayload,
}

#[derive(Debug, Serialize, Deserialize)]
enum WebWorkerRequestPayload {
    ProcessVtTile {
        tile: MvtTile,
        index: TileIndex,
        style: VectorTileStyle,
        tile_schema: TileSchema,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct WebWorkerResponse {
    request_id: WebWorkerRequestId,
    payload: WebWorkerResponsePayload,
}

#[derive(Debug, Serialize, Deserialize)]
enum WebWorkerResponsePayload {
    Ready,
    ProcessVtTile {
        result: Result<Vec<u8>, TileProcessingError>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
enum WebWorkerError {}

impl TryFrom<Result<WebWorkerResponsePayload, WebWorkerError>> for RenderBundle {
    type Error = TileProcessingError;

    fn try_from(
        value: Result<WebWorkerResponsePayload, WebWorkerError>,
    ) -> Result<Self, Self::Error> {
        match value {
            Ok(WebWorkerResponsePayload::ProcessVtTile { result }) => result.map(|bytes| {
                let converted: TessellatingRenderBundleBytes = bincode::deserialize(&bytes)
                    .expect("Failed to deserialize render bundle bytes");
                RenderBundle(RenderBundleType::Tessellating(
                    TessellatingRenderBundle::from_bytes_unchecked(converted),
                ))
            }),
            _ => {
                log::error!("Unexpected response type for tile processing request: {value:?}");
                Err(TileProcessingError::Internal)
            }
        }
    }
}

impl WebWorkerService {
    /// Create new instance of the service.
    pub fn new(worker_count: usize) -> Self {
        let (tx, rx) = tokio::sync::watch::channel(false);
        let mut service = Self {
            worker_pool: vec![],
            next_worker: Default::default(),
            pending_requests: Rc::new(RefCell::new(Default::default())),
            is_ready: RefCell::new(rx),
        };
        for _ in 0..worker_count {
            service.spawn_worker(tx.clone());
        }

        service
    }

    /// Pre-render vector tile.
    pub async fn process_vt_tile(
        &self,
        tile: Arc<MvtTile>,
        index: TileIndex,
        style: Arc<VectorTileStyle>,
        tile_schema: TileSchema,
    ) -> Result<RenderBundle, TileProcessingError> {
        let response = self
            .request_operation(WebWorkerRequestPayload::ProcessVtTile {
                tile: (*tile).clone(),
                index,
                style: (*style).clone(),
                tile_schema,
            })
            .await;

        response.try_into()
    }

    async fn request_operation(
        &self,
        payload: WebWorkerRequestPayload,
    ) -> Result<WebWorkerResponsePayload, WebWorkerError> {
        self.is_ready
            .borrow_mut()
            .wait_for(|v| *v)
            .await
            .expect("failed to read is_ready channel");

        let (sender, receiver) = oneshot::channel();
        let request_id = WebWorkerRequestId::next();
        self.add_request(request_id, sender);
        self.send_request(&WebWorkerRequest {
            request_id,
            payload,
        });

        log::debug!("Sent request {request_id} to web worker");

        receiver.await.expect("failed to read ww result channel")
    }

    fn add_request(
        &self,
        request_id: WebWorkerRequestId,
        result_channel: Sender<Result<WebWorkerResponsePayload, WebWorkerError>>,
    ) {
        self.pending_requests
            .borrow_mut()
            .insert(request_id, result_channel);
    }

    fn send_request(&self, request: &WebWorkerRequest) {
        let worker = self.next_worker();
        worker
            .post_message(
                &serde_wasm_bindgen::to_value(request).expect("failed to serialize ww request"),
            )
            .expect("failed to send a message to a web worker");
    }

    fn next_worker(&self) -> &web_sys::Worker {
        let next_worker_index = self.next_worker.fetch_add(1, Ordering::Relaxed);
        &self.worker_pool[next_worker_index % self.worker_pool.len()].worker
    }

    fn spawn_worker(&mut self, is_ready_sender: tokio::sync::watch::Sender<bool>) {
        let worker = web_sys::Worker::new(WORKER_URL).expect("failed to create web worker");
        let worker_state = Rc::new(WorkerState {
            worker,
            is_ready: AtomicBool::new(false),
        });
        let pending_requests = self.pending_requests.clone();

        let worker_state_clone = worker_state.clone();
        let callback: Closure<dyn FnMut(web_sys::MessageEvent)> = Closure::new(
            move |event: web_sys::MessageEvent| {
                let response: WebWorkerResponse = match serde_wasm_bindgen::from_value(event.data())
                {
                    Ok(v) => v,
                    Err(err) => {
                        log::error!(
                            "Failed to deserialize message ({:?}) from web worker: {err:?}",
                            event.data()
                        );
                        return;
                    }
                };

                log::info!(
                    "Recieved response for request {} from a web worker",
                    response.request_id
                );

                match response.payload {
                    WebWorkerResponsePayload::Ready => {
                        is_ready_sender
                            .send(true)
                            .expect("failed to send ready state through channel");
                        worker_state_clone.is_ready.store(true, Ordering::Relaxed)
                    }
                    v => {
                        let channel = pending_requests.borrow_mut().remove(&response.request_id);
                        if let Some(channel) = channel {
                            if let Err(err) = channel.send(Ok(v)) {
                                log::error!("Failed to send result of web worker execution through channel: {err:?}");
                            }

                            log::debug!(
                                "Response for request {} is sent to the caller",
                                response.request_id
                            );
                        }
                    }
                }
            },
        );

        worker_state
            .worker
            .set_onmessage(Some(callback.as_ref().unchecked_ref()));
        self.worker_pool.push(worker_state);

        callback.forget();
    }
}

mod worker {
    use galileo_mvt::MvtTile;
    use wasm_bindgen::{prelude::wasm_bindgen, JsCast, JsValue};

    use crate::{
        layer::vector_tile_layer::{
            style::VectorTileStyle,
            tile_provider::{processor::TileProcessingError, VtProcessor},
        },
        platform::web::web_workers::WebWorkerResponsePayload,
        render::render_bundle::RenderBundle,
        tile_scheme::TileIndex,
        TileSchema,
    };

    use super::{
        RenderBundleType, TessellatingRenderBundle, WebWorkerRequest, WebWorkerRequestId,
        WebWorkerRequestPayload, WebWorkerResponse,
    };

    #[wasm_bindgen]
    pub fn init_vt_worker() {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init_with_level(log::Level::Info).expect("Couldn't init logger");

        log::debug!("Vt worker is initialized");

        send_response(WebWorkerResponse {
            request_id: WebWorkerRequestId::empty(),
            payload: WebWorkerResponsePayload::Ready,
        });
    }

    fn send_response(response: WebWorkerResponse) {
        let js_value =
            serde_wasm_bindgen::to_value(&response).expect("failed to convert response to JsValue");

        js_sys::global()
            .dyn_into::<web_sys::DedicatedWorkerGlobalScope>()
            .expect("failed to get global web worker object")
            .post_message(&js_value)
            .expect("failed to send web worker response");

        log::debug!(
            "Web woker sent response ({js_value:?}) for request {}",
            response.request_id
        );
    }

    #[wasm_bindgen]
    pub fn process_message(msg: JsValue) -> JsValue {
        log::debug!("Web worker received a message");

        let request: WebWorkerRequest = serde_wasm_bindgen::from_value(msg)
            .expect("failed to decode JsValue into web worker request");
        let WebWorkerRequest {
            request_id,
            payload: request_payload,
        } = request;

        log::debug!("Web worker processing request {request_id}");

        let payload = process_request(request_payload);
        let response = WebWorkerResponse {
            request_id,
            payload,
        };

        log::debug!("Web worker processed request {request_id}");

        serde_wasm_bindgen::to_value(&response).expect("failed to convert response to JsValue")
    }

    fn process_request(request: WebWorkerRequestPayload) -> WebWorkerResponsePayload {
        match request {
            WebWorkerRequestPayload::ProcessVtTile {
                tile,
                index,
                style,
                tile_schema,
            } => process_vt_tile(tile, index, style, tile_schema),
        }
    }

    fn process_vt_tile(
        tile: MvtTile,
        index: TileIndex,
        style: VectorTileStyle,
        tile_schema: TileSchema,
    ) -> WebWorkerResponsePayload {
        let mut bundle = RenderBundle(RenderBundleType::Tessellating(
            TessellatingRenderBundle::new(),
        ));
        let result = match VtProcessor::prepare(&tile, &mut bundle, index, &style, &tile_schema) {
            Ok(()) => {
                let RenderBundle(RenderBundleType::Tessellating(tessellating)) = bundle;

                let bytes = tessellating.into_bytes();
                let serialized =
                    bincode::serialize(&bytes).expect("failed to serialize render bundle");
                Ok(serialized)
            }
            Err(_) => Err(TileProcessingError::Rendering),
        };

        WebWorkerResponsePayload::ProcessVtTile { result }
    }
}
