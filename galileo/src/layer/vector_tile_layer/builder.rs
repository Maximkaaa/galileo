use std::path::{Path, PathBuf};
use std::sync::Arc;

use bytes::Bytes;

use super::style::{
    VectorTileDefaultSymbol, VectorTileLineSymbol, VectorTilePolygonSymbol, VectorTileStyle,
};
use super::tile_provider::loader::WebVtLoader;
use super::tile_provider::processor::VectorTileProcessor;
use super::tile_provider::VectorTileProvider;
use super::VectorTileLayer;
use crate::error::GalileoError;
use crate::layer::data_provider::{FileCacheController, PersistentCacheController, UrlSource};
use crate::layer::Layer;
use crate::tile_schema::TileIndex;
use crate::{Color, Messenger, TileSchema};

/// Constructor for a [`VectorTileLayer`].
///
/// ```
/// use galileo::layer::vector_tile_layer::VectorTileLayerBuilder;
///
/// # fn load_tile_schema() -> galileo::TileSchema { galileo::TileSchema::web(10) }
/// # fn load_style() -> galileo::layer::vector_tile_layer::style::VectorTileStyle {
/// #     galileo::layer::vector_tile_layer::style::VectorTileStyle::default() }
///
/// let tile_schema = load_tile_schema();
/// let style = load_style();
///
/// let layer = VectorTileLayerBuilder::new_rest(
///     |index| {
///         format!(
///             "https://vector_tiles.example.com/{}/{}/{}.png",
///             index.z, index.x, index.y
///         )
///     })
///     .with_file_cache("target")
///     .with_tile_schema(tile_schema)
///     .with_style(style)
///     .build()?;
/// # Ok::<(), galileo::error::GalileoError>(())
/// ```
pub struct VectorTileLayerBuilder {
    provider_type: ProviderType,
    style: Option<VectorTileStyle>,
    tile_schema: Option<TileSchema>,
    messenger: Option<Box<dyn Messenger>>,
    cache: CacheType,
    offline_mode: bool,
}

enum ProviderType {
    Rest(Box<dyn UrlSource<TileIndex>>),
    Custom(VectorTileProvider),
}

enum CacheType {
    None,
    File(PathBuf),
    Custom(Box<dyn PersistentCacheController<str, Bytes>>),
}

impl VectorTileLayerBuilder {
    /// Initializes a builder for a layer that requests tiles from the given url source.
    ///
    /// ```
    /// use galileo::layer::vector_tile_layer::VectorTileLayerBuilder;
    ///
    /// let layer = VectorTileLayerBuilder::new_rest(
    ///     |index| {
    ///         format!(
    ///             "https://vector_tiles.example.com/{}/{}/{}.png",
    ///             index.z, index.x, index.y
    ///         )
    ///     }).build()?;
    /// # Ok::<(), galileo::error::GalileoError>(())
    /// ```
    pub fn new_rest(tile_source: impl UrlSource<TileIndex> + 'static) -> Self {
        Self {
            provider_type: ProviderType::Rest(Box::new(tile_source)),
            style: None,
            tile_schema: None,
            messenger: None,
            cache: CacheType::None,
            offline_mode: false,
        }
    }

    /// Initializes a builder for a lyer with the given tile provider.
    ///
    /// ```
    /// use galileo::layer::vector_tile_layer::VectorTileLayerBuilder;
    /// use galileo::layer::vector_tile_layer::tile_provider::VectorTileProvider;
    ///
    /// # fn get_tile_provider() -> VectorTileProvider {
    /// #     VectorTileLayerBuilder::new_rest(|_|
    /// #         unimplemented!()).build().unwrap().provider().clone()
    /// # }
    ///
    /// let provider = get_tile_provider();
    ///
    /// let layer = VectorTileLayerBuilder::new_with_provider(provider).build()?;
    /// # Ok::<(), galileo::error::GalileoError>(())
    /// ```
    pub fn new_with_provider(provider: VectorTileProvider) -> Self {
        Self {
            provider_type: ProviderType::Custom(provider),
            style: None,
            tile_schema: None,
            messenger: None,
            cache: CacheType::None,
            offline_mode: false,
        }
    }

    /// Adds a file cache for the tiles in the given folder.
    ///
    /// The file cache controller will create folders under the given path based on the url of the
    /// layer, so different layers can use the same `path` for the tile cache.
    ///
    /// If the `path` folder doesn't exist it will be creating. In case the creation of the folder
    /// fails, building the tile layer will return an error.
    ///
    /// Cannot be used with custom tile provider given by
    /// [`VectorTileLayerBuilder::new_with_provider()`] method as the provider must have already be
    /// created with the cache configured. So in this case building will also return an error.
    ///
    /// Replaces the value set by the [`VectorTileLayerBuilder::with_cache_controller()`] method.
    ///
    /// # Platforms
    ///
    /// When compiling for the `wasm32` architecture, file system operations are not available, so
    /// using a file cache will result in a runtime error. If you want to use the same code to
    /// create a layer for all platforms and not worry about cache availability, you can use
    /// [`VectorTileLayerBuilder::with_file_cache_checked()`] method.
    ///
    /// ```
    /// use galileo::layer::vector_tile_layer::VectorTileLayerBuilder;
    ///
    /// let layer = VectorTileLayerBuilder::new_rest(
    ///     |index| {
    ///         format!(
    ///             "https://vector_tiles.example.com/{}/{}/{}.png",
    ///             index.z, index.x, index.y
    ///         )
    ///     })
    ///     .with_file_cache("./target")
    ///     .build()?;
    /// # Ok::<(), galileo::error::GalileoError>(())
    /// ```
    pub fn with_file_cache(mut self, path: impl AsRef<Path>) -> Self {
        // You would think that we don't need `with_file_cache_checked` method and can move its
        // logic here instead. But actually not all `wasm32` platforms don't have access to the FS,
        // and there is no simple way to detect if there is for the current target. So I'd rather
        // have both methods for future, when we want to add support for more platforms or have a
        // better way to check if the FS operations are available on the current target.
        self.cache = CacheType::File(path.as_ref().into());
        self
    }

    /// Sets the file cache if available on the target platform, or skips it otherwise.
    ///
    /// Currently it only checks if the target architecture is "wasm32".
    ///
    /// ```
    /// use galileo::layer::vector_tile_layer::VectorTileLayerBuilder;
    ///
    /// let layer = VectorTileLayerBuilder::new_rest(
    ///     |index| {
    ///         format!(
    ///             "https://vector_tiles.example.com/{}/{}/{}.png",
    ///             index.z, index.x, index.y
    ///         )
    ///     })
    ///     .with_file_cache_checked("./target")
    ///     .build()?;
    /// # Ok::<(), galileo::error::GalileoError>(())
    /// ```
    pub fn with_file_cache_checked(self, _path: impl AsRef<Path>) -> Self {
        #[allow(unused_mut)]
        let mut this = self;
        #[cfg(not(target_arch = "wasm32"))]
        {
            this = this.with_file_cache(_path);
        }
        this
    }

    /// Adds the given persistent cache for the tiles.
    ///
    /// Cannot be used with custom tile provider given by
    /// [`VectorTileLayerBuilder::new_with_provider()`] method as the provider must have already be
    /// created with the cache configured. So in this case building will also return an error.
    ///
    /// Replaces the value set by the [`VectorTileLayerBuilder::with_file_cache()`] method.
    ///
    /// ```
    /// use galileo::layer::vector_tile_layer::VectorTileLayerBuilder;
    /// use galileo::layer::data_provider::FileCacheController;
    ///
    /// let cache_controller = FileCacheController::new("target")?;
    /// let layer = VectorTileLayerBuilder::new_rest(
    ///     |index| {
    ///         format!(
    ///             "https://vector_tiles.example.com/{}/{}/{}.png",
    ///             index.z, index.x, index.y
    ///         )
    ///     })
    ///     .with_cache_controller(cache_controller)
    ///     .build()?;
    /// # Ok::<(), galileo::error::GalileoError>(())
    /// ```
    pub fn with_cache_controller(
        mut self,
        cache: impl PersistentCacheController<str, Bytes> + 'static,
    ) -> Self {
        self.cache = CacheType::Custom(Box::new(cache));
        self
    }

    /// Sets the layer to only use cached tiles without requesting from the url source.
    ///
    /// Note that even in offline mode url source must be configured correctly as it will be used
    /// to identify tiles in the cache.
    ///
    /// Cannot be used with custom tile provider given by
    /// [`VectorTileLayerBuilder::new_with_provider()`] method as the provider must have already be
    /// created with the offline mode. So in this case building will also return an error.
    ///
    /// If the layer is set to offline mode but there is no cache configured, building it will
    /// return a configuration error.
    ///
    /// ```
    /// use galileo::layer::vector_tile_layer::VectorTileLayerBuilder;
    ///
    /// let layer = VectorTileLayerBuilder::new_rest(
    ///     |index| {
    ///         format!(
    ///             "https://vector_tiles.example.com/{}/{}/{}.png",
    ///             index.z, index.x, index.y
    ///         )
    ///     })
    ///     .with_file_cache("./target")
    ///     .with_offline_mode()
    ///     .build()?;
    /// # Ok::<(), galileo::error::GalileoError>(())
    /// ```
    pub fn with_offline_mode(mut self) -> Self {
        self.offline_mode = true;
        self
    }

    /// Sets the layer's tile schema.
    ///
    /// Defaults to `TileSchema::web(18)`. Note that for vector tiles you usually don't want to use
    /// the default schema as vector tiles usually are larger than 256 px.
    ///
    /// ```
    /// use galileo::layer::Layer;
    /// use galileo::layer::vector_tile_layer::VectorTileLayerBuilder;
    /// use galileo::TileSchema;
    ///
    /// let layer = VectorTileLayerBuilder::new_rest(
    ///     |index| {
    ///         format!(
    ///             "https://vector_tiles.example.com/{}/{}/{}.png",
    ///             index.z, index.x, index.y
    ///         )
    ///     })
    ///     .with_tile_schema(TileSchema::web(10))
    ///     .build()?;
    ///
    /// assert_eq!(*layer.tile_schema().as_ref().unwrap(), TileSchema::web(10));
    /// # Ok::<(), galileo::error::GalileoError>(())
    /// ```
    pub fn with_tile_schema(mut self, tile_schema: TileSchema) -> Self {
        self.tile_schema = Some(tile_schema);
        self
    }

    /// Sets the layer's messenger.
    ///
    /// Vector tile layer uses the messenger to notify application when a new tile is loaded and
    /// ready to be drawn. This is required since the tiles are loaded asynchronously.
    ///
    /// If the messenger is not set, after call to
    /// [`Layer::prepare()`](crate::layer::Layer::prepare()) drawing the layer will
    /// still not draw anything, since the tiles are not loaded yet.
    ///
    /// Setting the messenger separately for the layer is not required if the map is created with
    /// the [`MapBuilder`](crate::map::MapBuilder) and the messenger is set for the map.
    pub fn with_messenger(mut self, messenger: impl Messenger + 'static) -> Self {
        self.messenger = Some(Box::new(messenger));
        self
    }

    /// Sets the layer's style.
    ///
    /// ```
    /// use galileo::layer::vector_tile_layer::VectorTileLayerBuilder;
    /// use galileo::layer::vector_tile_layer::style::VectorTileStyle;
    ///
    /// # fn load_style() -> VectorTileStyle { VectorTileStyle::default() }
    ///
    /// let style = load_style();
    ///
    /// let layer = VectorTileLayerBuilder::new_rest(
    ///     |index| {
    ///         format!(
    ///             "https://vector_tiles.example.com/{}/{}/{}.png",
    ///             index.z, index.x, index.y
    ///         )
    ///     })
    ///     .with_style(style)
    ///     .build()?;
    /// # Ok::<(), galileo::error::GalileoError>(())
    /// ```
    pub fn with_style(mut self, style: VectorTileStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Consumes the builder and constructs the vector tile layer.
    ///
    /// Will return an error if the layer is configured incorrectly or if the cache controller
    /// fails to initialize.
    pub fn build(self) -> Result<VectorTileLayer, GalileoError> {
        let Self {
            provider_type,
            style,
            tile_schema,
            messenger,
            cache,
            offline_mode,
        } = self;

        let tile_schema = tile_schema.unwrap_or_else(|| TileSchema::web(18));

        let cache_controller: Option<Box<dyn PersistentCacheController<str, Bytes>>> = match cache {
            CacheType::None => None,
            CacheType::File(path_buf) => Some(Box::new(FileCacheController::new(&path_buf)?)),
            CacheType::Custom(persistent_cache_controller) => Some(persistent_cache_controller),
        };

        if cache_controller.is_none() && offline_mode {
            return Err(GalileoError::Configuration(
                "offline mode cannot be used without cache".into(),
            ));
        }

        let processor = Self::create_processor(tile_schema.clone());

        let provider = match provider_type {
            ProviderType::Rest(url_source) => {
                let loader = WebVtLoader::new(cache_controller, url_source, offline_mode);

                VectorTileProvider::new(Arc::new(loader), Arc::new(processor))
            }
            ProviderType::Custom(raster_tile_provider) => {
                if cache_controller.is_some() {
                    return Err(GalileoError::Configuration(
                        "custom tile provider cannot be used together with a cache controller"
                            .into(),
                    ));
                }

                raster_tile_provider
            }
        };

        let style = style.unwrap_or_else(Self::default_style);

        let mut layer = VectorTileLayer::new(provider, style, tile_schema);
        if let Some(messenger) = messenger {
            layer.set_messenger(messenger);
        }

        Ok(layer)
    }

    fn create_processor(tile_schema: TileSchema) -> impl VectorTileProcessor {
        #[cfg(target_arch = "wasm32")]
        {
            crate::platform::web::vt_processor::WebWorkerVtProcessor::new(
                tile_schema.clone(),
                crate::platform::web::web_workers::WebWorkerService::instance(),
            )
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            crate::platform::native::vt_processor::ThreadVtProcessor::new(tile_schema.clone())
        }
    }

    fn default_style() -> VectorTileStyle {
        VectorTileStyle {
            rules: vec![],
            default_symbol: VectorTileDefaultSymbol {
                point: None,
                line: Some(VectorTileLineSymbol {
                    width: 1.0,
                    stroke_color: Color::BLACK,
                }),
                polygon: Some(VectorTilePolygonSymbol {
                    fill_color: Color::GRAY,
                }),
                label: None,
            },
            background: Color::WHITE,
        }
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_compact_debug_snapshot;

    use super::*;

    fn custom_provider() -> VectorTileProvider {
        VectorTileLayerBuilder::new_rest(|_| unimplemented!())
            .build()
            .unwrap()
            .provider()
            .clone()
    }

    #[test]
    fn with_file_cache_replaces_cache_controller() {
        let cache = FileCacheController::new("target").unwrap();
        let builder = VectorTileLayerBuilder::new_rest(|_| unimplemented!())
            .with_cache_controller(cache)
            .with_file_cache("target");

        assert!(matches!(builder.cache, CacheType::File(_)));
    }

    #[test]
    fn with_file_cache_fails_build_if_cannot_init_folder() {
        let result = VectorTileLayerBuilder::new_rest(|_| unimplemented!())
            .with_file_cache("Cargo.toml")
            .build();

        assert!(result.is_err());
        assert_compact_debug_snapshot!(result, @r#"Err(FsIo("failed to initialize file cache folder \"Cargo.toml\": File exists (os error 17)"))"#);
    }

    #[test]
    fn with_file_cache_fails_build_if_custom_provider() {
        let provider = custom_provider();
        let result = VectorTileLayerBuilder::new_with_provider(provider)
            .with_file_cache("target")
            .build();

        assert!(result.is_err());
        assert_compact_debug_snapshot!(result, @r#"Err(Configuration("custom tile provider cannot be used together with a cache controller"))"#);
    }

    #[test]
    fn with_cache_controller_replaces_file_cache() {
        let cache = FileCacheController::new("target").unwrap();
        let builder = VectorTileLayerBuilder::new_rest(|_| unimplemented!())
            .with_file_cache("target")
            .with_cache_controller(cache);

        assert!(matches!(builder.cache, CacheType::Custom(_)));
    }

    #[test]
    fn with_cache_controller_fails_build_if_custom_provider() {
        let provider = custom_provider();
        let cache = FileCacheController::new("target").unwrap();
        let result = VectorTileLayerBuilder::new_with_provider(provider)
            .with_cache_controller(cache)
            .build();

        assert!(result.is_err());
        assert_compact_debug_snapshot!(result, @r#"Err(Configuration("custom tile provider cannot be used together with a cache controller"))"#);
    }

    #[test]
    fn with_offline_mode_incompatible_with_custom_provider() {
        let provider = custom_provider();
        let result = VectorTileLayerBuilder::new_with_provider(provider)
            .with_file_cache("target")
            .with_offline_mode()
            .build();

        assert!(result.is_err());
        assert_compact_debug_snapshot!(result, @r#"Err(Configuration("custom tile provider cannot be used together with a cache controller"))"#);
    }

    #[test]
    fn with_offline_mode_does_not_work_without_cache() {
        let result = VectorTileLayerBuilder::new_rest(|_| unimplemented!())
            .with_offline_mode()
            .build();

        assert!(result.is_err());
        assert_compact_debug_snapshot!(result, @r#"Err(Configuration("offline mode cannot be used without cache"))"#);
    }

    #[test]
    fn default_tile_schema() {
        let layer = VectorTileLayerBuilder::new_rest(|_| unimplemented!())
            .build()
            .unwrap();

        assert_eq!(*layer.tile_schema().as_ref().unwrap(), TileSchema::web(18));
    }
}
