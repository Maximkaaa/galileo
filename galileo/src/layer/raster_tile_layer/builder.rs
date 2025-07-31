use std::path::{Path, PathBuf};

use bytes::Bytes;

use super::{RasterTileLayer, RasterTileLoader, RestTileLoader};
use crate::error::GalileoError;
use crate::layer::attribution::Attribution;
use crate::layer::data_provider::{
    FileCacheController, FileCachePathModifier, PersistentCacheController, UrlSource,
};
use crate::tile_schema::TileIndex;
use crate::{Messenger, TileSchema};

/// Constructor for a [`RasterTileLayer`].
///
/// ```
/// use galileo::layer::raster_tile_layer::RasterTileLayerBuilder;
///
/// let layer = RasterTileLayerBuilder::new_rest(
///     |index| {
///         format!(
///             "https://tile.openstreetmap.org/{}/{}/{}.png",
///             index.z, index.x, index.y
///         )
///     })
///     .with_file_cache("target")
///     .build()?;
/// # Ok::<(), galileo::error::GalileoError>(())
/// ```
pub struct RasterTileLayerBuilder {
    loader_type: LoaderType,
    tile_schema: Option<TileSchema>,
    messenger: Option<Box<dyn Messenger>>,
    cache: CacheType,
    offline_mode: bool,
    attribution: Option<Attribution>,
}

enum LoaderType {
    Rest(Box<dyn UrlSource<TileIndex>>),
    Custom(Box<dyn RasterTileLoader>),
}

enum CacheType {
    None,
    File(PathBuf, Option<Box<FileCachePathModifier>>),
    Custom(Box<dyn PersistentCacheController<str, Bytes>>),
}

impl RasterTileLayerBuilder {
    /// Initializes a builder for a layer that requests tiles from the given url source.
    ///
    /// ```
    /// use galileo::layer::raster_tile_layer::RasterTileLayerBuilder;
    ///
    /// let layer = RasterTileLayerBuilder::new_rest(
    ///     |index| {
    ///         format!(
    ///             "https://tile.openstreetmap.org/{}/{}/{}.png",
    ///             index.z, index.x, index.y
    ///         )
    ///     }).build()?;
    /// # Ok::<(), galileo::error::GalileoError>(())
    /// ```
    pub fn new_rest(tile_source: impl UrlSource<TileIndex> + 'static) -> Self {
        Self {
            loader_type: LoaderType::Rest(Box::new(tile_source)),
            tile_schema: None,
            messenger: None,
            cache: CacheType::None,
            offline_mode: false,
            attribution: None,
        }
    }

    #[allow(rustdoc::bare_urls)]
    /// Initializes a builder for a raster tile layer with the Open Streets Map source.
    ///
    /// It uses the standard "https://tile.openstreetmap.org/z/x/y.png" URL pattern to retrieve the
    /// tiles.
    ///
    /// ```
    /// use galileo::layer::raster_tile_layer::RasterTileLayerBuilder;
    ///
    /// let layer = RasterTileLayerBuilder::new_osm().with_file_cache("target").build()?;
    /// # Ok::<(), galileo::error::GalileoError>(())
    /// ```
    pub fn new_osm() -> Self {
        Self {
            loader_type: LoaderType::Rest(Box::new(|index| {
                format!(
                    "https://tile.openstreetmap.org/{}/{}/{}.png",
                    index.z, index.x, index.y
                )
            })),
            tile_schema: None,
            messenger: None,
            cache: CacheType::None,
            offline_mode: false,
            attribution: Some(Attribution::new(
                "© OpenStreetMap contributors".to_string(),
                Some("https://www.openstreetmap.org/copyright".to_string()),
            )),
        }
    }

    /// Initializes a builder for a layer with the given tile loader.
    ///
    /// ```
    /// use galileo::layer::raster_tile_layer::{RestTileLoader, RasterTileLayerBuilder};
    ///
    /// let loader = RestTileLoader::new(
    ///     |index| {
    ///         format!(
    ///             "https://tile.openstreetmap.org/{}/{}/{}.png",
    ///             index.z, index.x, index.y
    ///         )
    ///     },
    ///     None,
    ///     false,
    /// );
    /// let layer = RasterTileLayerBuilder::new_with_loader(loader)
    ///     .build()?;
    /// # Ok::<(), galileo::error::GalileoError>(())
    /// ```
    pub fn new_with_loader(loader: impl RasterTileLoader + 'static) -> Self {
        Self {
            loader_type: LoaderType::Custom(Box::new(loader)),
            tile_schema: None,
            messenger: None,
            cache: CacheType::None,
            offline_mode: false,
            attribution: None,
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
    /// [`RasterTileLayerBuilder::new_with_loader()`] method as the provider must have already be
    /// created with the cache configured. So in this case building will also return an error.
    ///
    /// Replaces the value set by the [`RasterTileLayerBuilder::with_cache_controller()`] method.
    ///
    /// # Platforms
    ///
    /// When compiling for the `wasm32` architecture, file system operations are not available, so
    /// using a file cache will result in a runtime error. If you want to use the same code to
    /// create a layer for all platforms and not worry about cache availability, you can use
    /// [`RasterTileLayerBuilder::with_file_cache_checked()`] method.
    ///
    /// ```
    /// use galileo::layer::raster_tile_layer::RasterTileLayerBuilder;
    ///
    /// let layer = RasterTileLayerBuilder::new_rest(
    ///     |index| {
    ///         format!(
    ///             "https://tile.openstreetmap.org/{}/{}/{}.png",
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
        self.cache = CacheType::File(path.as_ref().into(), None);
        self
    }

    /// Same as [`with_file_cache`], but also modifies the file path by given `modifier` function
    ///
    /// ```
    /// use galileo::layer::raster_tile_layer::RasterTileLayerBuilder;
    ///
    /// let layer = RasterTileLayerBuilder::new_rest(
    ///     |index| {
    ///         format!(
    ///             "https://tile.openstreetmap.org/{}/{}/{}.png",
    ///             index.z, index.x, index.y
    ///         )
    ///     })
    ///     .with_file_cache_modifier(
    ///         "./target",
    ///         // modify file path to be `uppercase`
    ///         Box::new(|path| path.to_uppercase())
    ///     )
    ///     .build()?;
    /// # Ok::<(), galileo::error::GalileoError>(())
    /// ```
    pub fn with_file_cache_modifier(
        mut self,
        path: impl AsRef<Path>,
        modifier: Box<FileCachePathModifier>,
    ) -> Self {
        // You would think that we don't need `with_file_cache_modifier_checked` method and can move its
        // logic here instead. But actually not all `wasm32` platforms don't have access to the FS,
        // and there is no simple way to detect if there is for the current target. So I'd rather
        // have both methods for future, when we want to add support for more platforms or have a
        // better way to check if the FS operations are available on the current target.
        self.cache = CacheType::File(path.as_ref().into(), Some(modifier));
        self
    }

    /// Sets the file cache if available on the target platform, or skips it otherwise.
    ///
    /// Currently it only checks if the target architecture is "wasm32".
    ///
    /// ```
    /// use galileo::layer::raster_tile_layer::RasterTileLayerBuilder;
    ///
    /// let layer = RasterTileLayerBuilder::new_rest(
    ///     |index| {
    ///         format!(
    ///             "https://tile.openstreetmap.org/{}/{}/{}.png",
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

    /// Same as [`with_file_cache_checked`], but also modifies the file path by given `modifier` function
    ///
    /// ```
    /// use galileo::layer::raster_tile_layer::RasterTileLayerBuilder;
    ///
    /// let layer = RasterTileLayerBuilder::new_rest(
    ///     |index| {
    ///         format!(
    ///             "https://tile.openstreetmap.org/{}/{}/{}.png",
    ///             index.z, index.x, index.y
    ///         )
    ///     })
    ///     .with_file_cache_modifier_checked(
    ///         "./target",
    ///         // modify file path to be `uppercase`
    ///         Box::new(|path| path.to_uppercase())
    ///     )
    ///     .build()?;
    /// # Ok::<(), galileo::error::GalileoError>(())
    /// ```
    pub fn with_file_cache_modifier_checked(
        self,
        _path: impl AsRef<Path>,
        _modifier: Box<FileCachePathModifier>,
    ) -> Self {
        #[allow(unused_mut)]
        let mut this = self;
        #[cfg(not(target_arch = "wasm32"))]
        {
            this = this.with_file_cache_modifier(_path, _modifier);
        }
        this
    }

    /// Adds the given persistent cache for the tiles.
    ///
    /// Cannot be used with custom tile provider given by
    /// [`RasterTileLayerBuilder::new_with_loader()`] method as the provider must have already be
    /// created with the cache configured. So in this case building will also return an error.
    ///
    /// Replaces the value set by the [`RasterTileLayerBuilder::with_file_cache()`] method.
    ///
    /// ```
    /// use galileo::layer::raster_tile_layer::RasterTileLayerBuilder;
    /// use galileo::layer::data_provider::FileCacheController;
    ///
    /// let cache_controller = FileCacheController::new("target", None)?;
    /// let layer = RasterTileLayerBuilder::new_rest(
    ///     |index| {
    ///         format!(
    ///             "https://tile.openstreetmap.org/{}/{}/{}.png",
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
    /// [`RasterTileLayerBuilder::new_with_loader()`] method as the provider must have already be
    /// created with the offline mode. So in this case building will also return an error.
    ///
    /// If the layer is set to offline mode but there is no cache configured, building it will
    /// return a configuration error.
    ///
    /// ```
    /// use galileo::layer::raster_tile_layer::RasterTileLayerBuilder;
    ///
    /// let layer = RasterTileLayerBuilder::new_rest(
    ///     |index| {
    ///         format!(
    ///             "https://tile.openstreetmap.org/{}/{}/{}.png",
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
    /// Defaults to `TileSchema::web(18)`.
    ///
    /// ```
    /// use galileo::layer::raster_tile_layer::RasterTileLayerBuilder;
    /// use galileo::TileSchema;
    ///
    /// let layer = RasterTileLayerBuilder::new_rest(
    ///     |index| {
    ///         format!(
    ///             "https://tile.openstreetmap.org/{}/{}/{}.png",
    ///             index.z, index.x, index.y
    ///         )
    ///     })
    ///     .with_tile_schema(TileSchema::web(10))
    ///     .build()?;
    ///
    /// assert_eq!(*layer.tile_schema(), TileSchema::web(10));
    /// # Ok::<(), galileo::error::GalileoError>(())
    /// ```
    pub fn with_tile_schema(mut self, tile_schema: TileSchema) -> Self {
        self.tile_schema = Some(tile_schema);
        self
    }

    /// Sets the layer's messenger.
    ///
    /// Raster tile layer uses the messenger to notify application when a new tile is loaded and
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

    /// Sets the custom attribution with the given text and URL.
    /// The attribution consists of a text
    /// description and an optional URL where more information or the source can be found.
    pub fn with_attribution(mut self, text: String, url: String) -> Self {
        self.attribution = Some(Attribution::new(text, Some(url)));
        self
    }

    /// Consumes the builder and constructs the raster tile layer.
    ///
    /// Will return an error if the layer is configured incorrectly or if the cache controller
    /// fails to initialize.
    pub fn build(self) -> Result<RasterTileLayer, GalileoError> {
        let Self {
            loader_type: provider_type,
            tile_schema,
            messenger,
            cache,
            offline_mode,
            attribution,
        } = self;

        let tile_schema = tile_schema.unwrap_or_else(|| TileSchema::web(18));

        let cache_controller: Option<Box<dyn PersistentCacheController<str, Bytes>>> = match cache {
            CacheType::None => None,
            CacheType::File(path_buf, modifier) => {
                Some(Box::new(FileCacheController::new(&path_buf, modifier)?))
            }
            CacheType::Custom(persistent_cache_controller) => Some(persistent_cache_controller),
        };

        if cache_controller.is_none() && offline_mode {
            return Err(GalileoError::Configuration(
                "offline mode cannot be used without cache".into(),
            ));
        }

        let provider: Box<dyn RasterTileLoader> = match provider_type {
            LoaderType::Rest(url_source) => Box::new(RestTileLoader::new(
                url_source,
                cache_controller,
                offline_mode,
            )),
            LoaderType::Custom(raster_tile_provider) => {
                if cache_controller.is_some() {
                    return Err(GalileoError::Configuration(
                        "custom tile provider cannot be used together with a cache controller"
                            .into(),
                    ));
                }

                raster_tile_provider
            }
        };

        Ok(RasterTileLayer::new_raw(
            provider,
            tile_schema,
            messenger,
            attribution,
        ))
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_compact_debug_snapshot;

    use super::*;

    #[test]
    fn with_file_cache_replaces_cache_controller() {
        let cache = FileCacheController::new("target", None).unwrap();
        let builder = RasterTileLayerBuilder::new_rest(|_| unimplemented!())
            .with_cache_controller(cache)
            .with_file_cache("target");

        assert!(matches!(builder.cache, CacheType::File(_, None)));
    }

    #[test]
    fn with_file_cache_fails_build_if_cannot_init_folder() {
        let result = RasterTileLayerBuilder::new_rest(|_| unimplemented!())
            .with_file_cache("Cargo.toml")
            .build();

        assert!(result.is_err());
        assert_compact_debug_snapshot!(result, @r#"Err(FsIo("failed to initialize file cache folder \"Cargo.toml\": File exists (os error 17)"))"#);
    }

    #[test]
    fn with_file_cache_fails_build_if_custom_provider() {
        let provider = RestTileLoader::new(|_| unimplemented!(), None, false);
        let result = RasterTileLayerBuilder::new_with_loader(provider)
            .with_file_cache("target")
            .build();

        assert!(result.is_err());
        assert_compact_debug_snapshot!(result, @r#"Err(Configuration("custom tile provider cannot be used together with a cache controller"))"#);
    }

    #[test]
    fn with_cache_controller_replaces_file_cache() {
        let cache = FileCacheController::new("target", None).unwrap();
        let builder = RasterTileLayerBuilder::new_rest(|_| unimplemented!())
            .with_file_cache("target")
            .with_cache_controller(cache);

        assert!(matches!(builder.cache, CacheType::Custom(_)));
    }

    #[test]
    fn with_cache_controller_fails_build_if_custom_provider() {
        let provider = RestTileLoader::new(|_| unimplemented!(), None, false);
        let cache = FileCacheController::new("target", None).unwrap();
        let result = RasterTileLayerBuilder::new_with_loader(provider)
            .with_cache_controller(cache)
            .build();

        assert!(result.is_err());
        assert_compact_debug_snapshot!(result, @r#"Err(Configuration("custom tile provider cannot be used together with a cache controller"))"#);
    }

    #[test]
    fn with_offline_mode_incompatible_with_custom_provider() {
        let provider = RestTileLoader::new(|_| unimplemented!(), None, false);
        let result = RasterTileLayerBuilder::new_with_loader(provider)
            .with_file_cache("target")
            .with_offline_mode()
            .build();

        assert!(result.is_err());
        assert_compact_debug_snapshot!(result, @r#"Err(Configuration("custom tile provider cannot be used together with a cache controller"))"#);
    }

    #[test]
    fn with_offline_mode_does_not_work_without_cache() {
        let result = RasterTileLayerBuilder::new_rest(|_| unimplemented!())
            .with_offline_mode()
            .build();

        assert!(result.is_err());
        assert_compact_debug_snapshot!(result, @r#"Err(Configuration("offline mode cannot be used without cache"))"#);
    }

    #[test]
    fn default_tile_schema() {
        let layer = RasterTileLayerBuilder::new_rest(|_| unimplemented!())
            .build()
            .unwrap();

        assert_eq!(*layer.tile_schema(), TileSchema::web(18));
    }
}
