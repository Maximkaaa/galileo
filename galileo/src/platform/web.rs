//! Platform specific stuff for WASM32 (web) targets.

use crate::decoded_image::DecodedImage;
use crate::error::GalileoError;
use crate::platform::PlatformService;
use async_trait::async_trait;
use js_sys::Uint8Array;
use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    CanvasRenderingContext2d, HtmlCanvasElement, HtmlImageElement, Request, RequestInit,
    RequestMode, Response, WorkerGlobalScope,
};

pub mod map_builder;
pub mod vt_processor;
pub mod web_workers;

/// Platform service for Web target.
pub struct WebPlatformService {}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl PlatformService for WebPlatformService {
    fn new() -> Self {
        Self {}
    }

    async fn load_image_url(&self, url: &str) -> Result<DecodedImage, GalileoError> {
        let image = ImageFuture::new(url).await?;

        let canvas: HtmlCanvasElement = web_sys::window()
            .ok_or(GalileoError::Wasm(Some(
                "Window element is not available".into(),
            )))?
            .document()
            .ok_or(GalileoError::Wasm(Some(
                "Document element is not available".into(),
            )))?
            .create_element("canvas")?
            .dyn_into::<HtmlCanvasElement>()?;

        canvas.set_width(image.natural_width());
        canvas.set_height(image.natural_height());

        let context = canvas
            .get_context("2d")?
            .ok_or(GalileoError::Wasm(Some(
                "Cannot get 2d canvas context".into(),
            )))?
            .dyn_into::<CanvasRenderingContext2d>()?;

        context.draw_image_with_html_image_element(&image, 0.0, 0.0)?;
        let image_data = context.get_image_data(
            0.0,
            0.0,
            image.natural_width() as f64,
            image.natural_height() as f64,
        )?;

        let dimensions = (image_data.width(), image_data.height());

        Ok(DecodedImage {
            bytes: image_data.data().to_vec(),
            dimensions,
        })
    }

    async fn load_bytes_from_url(&self, url: &str) -> Result<bytes::Bytes, GalileoError> {
        let mut opts = RequestInit::new();
        opts.method("GET");
        opts.mode(RequestMode::Cors);

        let request =
            Request::new_with_str_and_init(url, &opts).expect("failed to create a request object");
        request
            .headers()
            .set("Accept", "application/vnd.mapbox-vector-tile")?;

        use wasm_bindgen::JsCast;
        let resp_value = {
            if let Some(window) = web_sys::window() {
                JsFuture::from(window.fetch_with_request(&request)).await?
            } else if let Ok(global) = js_sys::global().dyn_into::<WorkerGlobalScope>() {
                JsFuture::from(global.fetch_with_request(&request)).await?
            } else {
                return Err(GalileoError::Wasm(Some(
                    "Global object is not available".into(),
                )));
            }
        };

        assert!(resp_value.is_instance_of::<Response>());
        let resp: Response = resp_value.dyn_into()?;

        let bytes_val = JsFuture::from(resp.array_buffer()?).await?;
        let array = Uint8Array::new(&bytes_val);
        Ok(array.to_vec().into())
    }
}

/// Future for getting image with browser API
pub struct ImageFuture {
    image: Option<HtmlImageElement>,
    load_failed: Rc<Cell<bool>>,
}

impl ImageFuture {
    /// Create a new instance.
    pub fn new(path: &str) -> Self {
        let image = HtmlImageElement::new().expect("Cannot create HTMLImage Element");
        image.set_cross_origin(Some("anonymous"));
        image.set_src(path);
        ImageFuture {
            image: Some(image),
            load_failed: Rc::new(Cell::new(false)),
        }
    }
}

impl Future for ImageFuture {
    type Output = Result<HtmlImageElement, GalileoError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match &self.image {
            Some(image) if image.complete() => {
                let image = self.image.take().expect("Image future is in invalid state");
                let failed = self.load_failed.get();

                if failed {
                    Poll::Ready(Err(GalileoError::IO))
                } else {
                    Poll::Ready(Ok(image))
                }
            }
            Some(image) => {
                let waker = cx.waker().clone();
                let on_load_closure = Closure::wrap(Box::new(move || {
                    waker.wake_by_ref();
                }) as Box<dyn FnMut()>);
                image.set_onload(Some(on_load_closure.as_ref().unchecked_ref()));
                on_load_closure.forget();

                let waker = cx.waker().clone();
                let failed_flag = self.load_failed.clone();
                let on_error_closure = Closure::wrap(Box::new(move || {
                    failed_flag.set(true);
                    waker.wake_by_ref();
                }) as Box<dyn FnMut()>);
                image.set_onerror(Some(on_error_closure.as_ref().unchecked_ref()));
                on_error_closure.forget();

                Poll::Pending
            }
            _ => Poll::Ready(Err(GalileoError::IO)),
        }
    }
}
