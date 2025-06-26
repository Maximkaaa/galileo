//! Operations with Web Workers.

use std::cell::{LazyCell, RefCell};
use std::collections::HashMap;
use std::fmt::Display;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use bytes::Bytes;
use futures::channel::oneshot;
use futures::channel::oneshot::Sender;
use galileo_mvt::MvtTile;
use serde::{Deserialize, Serialize};
use tokio::sync::watch::Receiver;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::Worker;

use crate::layer::vector_tile_layer::style::VectorTileStyle;
use crate::layer::vector_tile_layer::tile_provider::processor::TileProcessingError;
use crate::render::render_bundle::RenderBundle;
use crate::tile_schema::TileIndex;
use crate::TileSchema;

const WORKER_URL: &str = "./vt_worker.js";
const WORKER_COUNT: usize = 4;

thread_local! {
    static INSTANCE: LazyCell<Rc<WebWorkerService>> = LazyCell::new(|| {
        Rc::new(WebWorkerService::new(WORKER_COUNT))
    });
}

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
    is_ready: Receiver<bool>,
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
    LoadFont {
        font_data: Bytes,
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
        result: Result<RenderBundle, TileProcessingError>,
    },
    LoadFont,
}

#[derive(Debug, Serialize, Deserialize)]
enum WebWorkerError {}

impl TryFrom<Result<WebWorkerResponsePayload, WebWorkerError>> for RenderBundle {
    type Error = TileProcessingError;

    fn try_from(
        value: Result<WebWorkerResponsePayload, WebWorkerError>,
    ) -> Result<Self, Self::Error> {
        match value {
            Ok(WebWorkerResponsePayload::ProcessVtTile { result }) => result,
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
            is_ready: rx,
        };

        let ready_count = Arc::new(AtomicUsize::new(0));
        for _ in 0..worker_count {
            service.spawn_worker(tx.clone(), worker_count, ready_count.clone());
        }

        service
    }

    /// Returns a static instance of the service.
    pub fn instance() -> Rc<Self> {
        INSTANCE.with(|v| (*v).clone())
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
            .request_operation(
                WebWorkerRequestPayload::ProcessVtTile {
                    tile: (*tile).clone(),
                    index,
                    style: (*style).clone(),
                    tile_schema,
                },
                self.next_worker(),
            )
            .await;

        response.try_into()
    }

    /// Loads font data to the font service in web workers.
    pub async fn load_font(&self, font_data: Arc<Vec<u8>>) {
        let mut futures = vec![];
        for worker in &self.worker_pool {
            futures.push(self.request_operation(
                WebWorkerRequestPayload::LoadFont {
                    font_data: (*font_data).clone().into(),
                },
                &worker.worker,
            ));
        }

        for result in futures::future::join_all(futures).await {
            if let Err(err) = result {
                log::error!("Failed to send font to web worker: {err:?}");
            }
        }
    }

    async fn request_operation(
        &self,
        payload: WebWorkerRequestPayload,
        worker: &Worker,
    ) -> Result<WebWorkerResponsePayload, WebWorkerError> {
        self.is_ready
            .clone()
            .wait_for(|v| *v)
            .await
            .expect("failed to read is_ready channel");

        let (sender, receiver) = oneshot::channel();
        let request_id = WebWorkerRequestId::next();
        self.add_request(request_id, sender);

        let start = web_time::Instant::now();
        self.send_request(
            &WebWorkerRequest {
                request_id,
                payload,
            },
            worker,
        );

        log::trace!(
            "Sent request {request_id} to web worker in {} ms",
            start.elapsed().as_millis()
        );

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

    fn send_request(&self, request: &WebWorkerRequest, worker: &Worker) {
        let bytes = bincode::serde::encode_to_vec(request, bincode::config::standard())
            .expect("failed to serialize ww request");
        let buf = serde_bytes::ByteBuf::from(bytes);
        worker
            .post_message(
                &serde_wasm_bindgen::to_value(&buf).expect("failed to serialize ww request"),
            )
            .expect("failed to send a message to a web worker");
    }

    fn next_worker(&self) -> &web_sys::Worker {
        let next_worker_index = self.next_worker.fetch_add(1, Ordering::Relaxed);
        &self.worker_pool[next_worker_index % self.worker_pool.len()].worker
    }

    fn spawn_worker(
        &mut self,
        is_ready_sender: tokio::sync::watch::Sender<bool>,
        worker_count: usize,
        ready_count: Arc<AtomicUsize>,
    ) {
        let worker = web_sys::Worker::new(WORKER_URL).expect("failed to create web worker");
        let worker_state = Rc::new(WorkerState {
            worker,
            is_ready: AtomicBool::new(false),
        });
        let pending_requests = self.pending_requests.clone();

        let worker_state_clone = worker_state.clone();
        let callback: Closure<dyn FnMut(web_sys::MessageEvent)> = Closure::new(
            move |event: web_sys::MessageEvent| {
                let start = web_time::Instant::now();
                let bytes: serde_bytes::ByteBuf = match serde_wasm_bindgen::from_value(event.data())
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

                let response: WebWorkerResponse =
                    match bincode::serde::decode_from_slice(&bytes, bincode::config::standard()) {
                        Ok(v) => v.0,
                        Err(err) => {
                            log::error!(
                                "Failed to deserialize message ({:?}) from web worker: {err:?}",
                                event.data()
                            );
                            return;
                        }
                    };

                log::trace!(
                    "Received response for request {} from a web worker in {} ms",
                    response.request_id,
                    start.elapsed().as_millis(),
                );

                match response.payload {
                    WebWorkerResponsePayload::Ready => {
                        let ready_count = ready_count.fetch_add(1, Ordering::Relaxed) + 1;
                        worker_state_clone.is_ready.store(true, Ordering::Relaxed);

                        log::debug!("Initialized {ready_count} out of {worker_count} workers");

                        if ready_count == worker_count {
                            log::debug!("WebWorkerService is ready to roll");
                            is_ready_sender
                                .send(true)
                                .expect("failed to send ready state through channel");
                        }
                    }
                    v => {
                        let channel = pending_requests.borrow_mut().remove(&response.request_id);
                        if let Some(channel) = channel {
                            if let Err(err) = channel.send(Ok(v)) {
                                log::error!("Failed to send result of web worker execution through channel: {err:?}");
                            }

                            log::trace!(
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
    use std::sync::Arc;

    use bytes::Bytes;
    use galileo_mvt::MvtTile;
    use serde_bytes::ByteBuf;
    use wasm_bindgen::prelude::wasm_bindgen;
    use wasm_bindgen::{JsCast, JsValue};

    use super::{WebWorkerRequest, WebWorkerRequestId, WebWorkerRequestPayload, WebWorkerResponse};
    use crate::layer::vector_tile_layer::style::VectorTileStyle;
    use crate::layer::vector_tile_layer::tile_provider::processor::TileProcessingError;
    use crate::layer::vector_tile_layer::tile_provider::VtProcessor;
    use crate::platform::web::web_workers::WebWorkerResponsePayload;
    use crate::render::render_bundle::RenderBundle;
    use crate::render::text::{RustybuzzRasterizer, TextService};
    use crate::tile_schema::TileIndex;
    use crate::TileSchema;

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
        let bytes = bincode::serde::encode_to_vec(&response, bincode::config::standard())
            .expect("failed to serialize ww response");
        let buf = serde_bytes::ByteBuf::from(bytes);
        let js_value =
            serde_wasm_bindgen::to_value(&buf).expect("failed to convert response to JsValue");

        js_sys::global()
            .dyn_into::<web_sys::DedicatedWorkerGlobalScope>()
            .expect("failed to get global web worker object")
            .post_message(&js_value)
            .expect("failed to send web worker response");

        log::trace!(
            "Web woker sent response ({js_value:?}) for request {}",
            response.request_id
        );
    }

    #[wasm_bindgen]
    pub fn process_message(msg: JsValue) -> JsValue {
        log::debug!("Web worker received a message");

        let start = web_time::Instant::now();
        let buf: ByteBuf = serde_wasm_bindgen::from_value(msg)
            .expect("failed to decode JsValue into web worker request");
        let bytes = buf.into_vec();
        let (request, _): (WebWorkerRequest, _) =
            bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
                .expect("failed to decode bytes into request");

        let WebWorkerRequest {
            request_id,
            payload: request_payload,
        } = request;

        log::trace!(
            "Web worker processing request {request_id}. Decoded in {} ms",
            start.elapsed().as_millis()
        );

        let start = web_time::Instant::now();
        let payload = process_request(request_payload);
        let response = WebWorkerResponse {
            request_id,
            payload,
        };
        log::trace!(
            "Processed request {request_id} in {} ms",
            start.elapsed().as_millis()
        );

        let start = web_time::Instant::now();
        let bytes = bincode::serde::encode_to_vec(&response, bincode::config::standard())
            .expect("failed to serialize ww response");
        let buf = serde_bytes::ByteBuf::from(bytes);
        let result =
            serde_wasm_bindgen::to_value(&buf).expect("failed to convert response to JsValue");

        log::debug!(
            "Web worker encoded request {request_id} in {} ms",
            start.elapsed().as_millis()
        );

        result
    }

    fn process_request(request: WebWorkerRequestPayload) -> WebWorkerResponsePayload {
        match request {
            WebWorkerRequestPayload::ProcessVtTile {
                tile,
                index,
                style,
                tile_schema,
            } => process_vt_tile(tile, index, style, tile_schema),
            WebWorkerRequestPayload::LoadFont { font_data } => load_font(font_data),
        }
    }

    fn load_font(font_data: Bytes) -> WebWorkerResponsePayload {
        log::trace!("Loading font data in web workder");

        if TextService::instance().is_none() {
            let provider = RustybuzzRasterizer::default();
            TextService::initialize(provider);
        }

        if let Some(instance) = TextService::instance() {
            let font_data = Arc::new(font_data.to_vec());
            instance.load_font_internal(font_data, false);
        }

        WebWorkerResponsePayload::LoadFont
    }

    fn process_vt_tile(
        tile: MvtTile,
        index: TileIndex,
        style: VectorTileStyle,
        tile_schema: TileSchema,
    ) -> WebWorkerResponsePayload {
        let mut bundle = RenderBundle::default();
        let result = match VtProcessor::prepare(&tile, &mut bundle, index, &style, &tile_schema) {
            Ok(()) => Ok(bundle),
            Err(_) => Err(TileProcessingError::Rendering),
        };

        WebWorkerResponsePayload::ProcessVtTile { result }
    }
}
