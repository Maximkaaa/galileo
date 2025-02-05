use galileo_types::cartesian::{CartesianPoint2d, Point2d};
use galileo_types::geo::impls::GeoPoint2d;
use galileo_types::geo::{Crs, GeoPoint};
use galileo_types::latlon;

use super::Map;
use crate::layer::Layer;
use crate::{MapView, Messenger};

// z-level 4 on the standard web tile scheme
const DEFAULT_RESOLUTION: f64 = 156543.03392800014 / 16.0;

/// Convenience type to initialize a [Map].
///
/// ```
/// use galileo::MapBuilder;
/// use galileo::galileo_types::latlon;
/// # use approx::assert_relative_eq;
/// #
/// # use galileo::layer::raster_tile_layer::RasterTileLayerBuilder;
/// # let tile_layer = RasterTileLayerBuilder::new_rest(|_| unimplemented!()).build().unwrap();
///
/// let map = MapBuilder::default()
///     .with_position(latlon!(55.0, 37.0))
///     .with_z_level(12)
///     .with_layer(tile_layer)
///     .build();
/// ```
#[derive(Default)]
pub struct MapBuilder {
    position: Option<GeoPoint2d>,
    projected_position: Option<Point2d>,
    resolution: Option<f64>,
    z_level: Option<u32>,
    crs: Option<Crs>,
    layers: Vec<Box<dyn Layer>>,
    messenger: Option<Box<dyn Messenger>>,
}

impl MapBuilder {
    /// Sets the center point of the map to the given geographical point.
    ///
    /// If the given geographical point cannot be projected to the map [Crs], the map will not be
    /// renderred. To set position of a map using CRS that cannot be projected into from
    /// geographical coordinates, use [`MapBuilder::with_projected_position()`] instead.
    ///
    /// Replaces the values set by the [`MapBuilder::with_projected_position()`] and
    /// [`MapBuilder::with_latlon()`] methods.
    ///
    /// Defaults to [0, 0].
    ///
    /// ```
    /// use galileo::MapBuilder;
    /// use galileo::galileo_types::latlon;
    /// use galileo::galileo_types::geo::GeoPoint;
    /// # use approx::assert_relative_eq;
    ///
    /// let map = MapBuilder::default().with_position(latlon!(55.0, 37.0)).build();
    ///
    /// assert_relative_eq!(map.view().position().unwrap().lat(), 55.0, epsilon = 1e-6);
    /// assert_relative_eq!(map.view().position().unwrap().lon(), 37.0, epsilon = 1e-6);
    /// ```
    pub fn with_position(mut self, position: impl GeoPoint<Num = f64>) -> Self {
        self.position = Some(GeoPoint2d::from(&position));
        self.projected_position = None;
        self
    }

    /// Sets the center point of the map to the given geographical coordinates.
    ///
    /// If the given geographical point cannot be projected to the map [Crs], the map will not be
    /// renderred. To set position of a map using CRS that cannot be projected into from
    /// geographical coordinates, use [`MapBuilder::with_projected_position()`] instead.
    ///
    /// Replaces the values set by the [`MapBuilder::with_projected_position()`] and
    /// [`MapBuilder::with_position()`] methods.
    ///
    /// Defaults to [0, 0].
    ///
    /// ```
    /// use galileo::MapBuilder;
    /// use galileo::galileo_types::geo::GeoPoint;
    /// # use approx::assert_relative_eq;
    ///
    /// let map = MapBuilder::default().with_latlon(55.0, 37.0).build();
    ///
    /// assert_relative_eq!(map.view().position().unwrap().lat(), 55.0, epsilon = 1e-6);
    /// assert_relative_eq!(map.view().position().unwrap().lon(), 37.0, epsilon = 1e-6);
    /// ```
    pub fn with_latlon(self, lat: f64, lon: f64) -> Self {
        self.with_position(latlon!(lat, lon))
    }

    /// Sets the center point of the map to the coordinates in the map [Crs].
    ///
    /// Replaces the values set by the [`MapBuilder::with_position`] and
    /// [`MapBuilder::with_latlon()`] methods.
    ///
    /// Defaults to [0, 0].
    ///
    /// ```
    /// use galileo::MapBuilder;
    /// use galileo::galileo_types::cartesian::Point2d;
    /// use galileo::galileo_types::geo::GeoPoint;
    /// # use approx::assert_relative_eq;
    ///
    /// let position = Point2d::new(338639.2, 4404718.1);
    /// let map = MapBuilder::default().with_projected_position(position).build();
    ///
    /// assert_relative_eq!(map.view().position().unwrap().lat(), 36.752887, epsilon = 1e-6);
    /// assert_relative_eq!(map.view().position().unwrap().lon(), 3.042048, epsilon = 1e-6);
    /// ```
    pub fn with_projected_position(mut self, position: impl CartesianPoint2d<Num = f64>) -> Self {
        self.projected_position = Some(Point2d::new(position.x(), position.y()));
        self.position = None;
        self
    }

    /// Sets the [resolution](MapView::resolution()) of the map.
    ///
    /// Replaces the value set by the [`MapBuilder::with_z_level`] method.
    ///
    /// Defaults to `9783.939620500008`, which corresponds to z-level 4 on the standard Web
    /// Mercator tile schema used by most services.
    ///
    /// ```
    /// use galileo::MapBuilder;
    /// # use approx::assert_relative_eq;
    ///
    /// let map = MapBuilder::default().with_resolution(1000.0).build();
    ///
    /// assert_relative_eq!(map.view().resolution(), 1000.0);
    /// ```
    pub fn with_resolution(mut self, resolution: f64) -> Self {
        self.resolution = Some(resolution);
        self.z_level = None;
        self
    }

    /// Sets the [resolution](MapView::resolution()) of the map to the resolution corresponding to the given z-level.
    ///
    /// Z-level is the index of level of detail in a [`TileSchema`]. The map itself does not have a
    /// tile schema, so the builder will look through the layers added by
    /// [`MapBuilder::with_layer()`] method. It will then use the tile schema of the first layer
    /// that uses it.
    ///
    /// If no layers in the map have a tile schema, default resolution value will be used.
    ///
    /// ```
    /// use galileo::MapBuilder;
    /// # use approx::assert_relative_eq;
    ///
    /// # use galileo::layer::raster_tile_layer::RasterTileLayerBuilder;
    /// # let tile_layer = RasterTileLayerBuilder::new_rest(|_| unimplemented!()).build().unwrap();
    ///
    /// let map = MapBuilder::default().with_layer(tile_layer).with_z_level(5).build();
    ///
    /// assert_relative_eq!(map.view().resolution(), 4891.96981025, epsilon = 1e-6);
    /// ```
    pub fn with_z_level(mut self, z_level: u32) -> Self {
        self.z_level = Some(z_level);
        self.resolution = None;
        self
    }

    /// Sets [Crs] of the map.
    ///
    /// Defaults to [`Crs::EPSG3857`], which is Web Mercator projection on a `WGS84` ellipsoid.
    ///
    /// ```
    /// use galileo::MapBuilder;
    /// use galileo::galileo_types::geo::Crs;
    ///
    /// let map = MapBuilder::default().with_crs(Crs::EPSG3857).build();
    ///
    /// assert_eq!(*map.view().crs(), Crs::EPSG3857);
    /// ```
    pub fn with_crs(mut self, crs: Crs) -> Self {
        self.crs = Some(crs);
        self
    }

    /// Adds a layer at the top of the map.
    ///
    /// ```
    /// use galileo::MapBuilder;
    ///
    /// # use galileo::layer::raster_tile_layer::RasterTileLayerBuilder;
    /// # let tile_layer = RasterTileLayerBuilder::new_rest(|_| unimplemented!()).build().unwrap();
    ///
    /// let map = MapBuilder::default().with_layer(tile_layer).build();
    ///
    /// assert_eq!(map.layers().len(), 1);
    /// ```
    pub fn with_layer(mut self, layer: impl Layer + 'static) -> Self {
        self.layers.push(Box::new(layer));
        self
    }

    /// Sets a [messenger](Messenger) implementation to the map.
    pub fn with_messenger(mut self, messenger: impl Messenger + 'static) -> Self {
        self.messenger = Some(Box::new(messenger));
        self
    }

    /// Consumes the builder and creates a map instance.
    ///
    /// If some of the parameters are not specified before calling `build`, they will be set to the
    /// default values.
    pub fn build(self) -> Map {
        let MapBuilder {
            position,
            projected_position,
            resolution,
            z_level,
            crs,
            layers,
            messenger,
        } = self;
        let crs = crs.unwrap_or(Crs::EPSG3857);

        let resolution = if let Some(z_level) = z_level {
            match layers.iter().filter_map(|layer| layer.tile_schema()).next() {
                Some(schema) => schema.lod_resolution(z_level).unwrap_or(DEFAULT_RESOLUTION),
                None => DEFAULT_RESOLUTION,
            }
        } else {
            resolution.unwrap_or(DEFAULT_RESOLUTION)
        };

        let view = if let Some(position) = position {
            MapView::new_with_crs(&position, resolution, crs)
        } else {
            let projected_position = projected_position.unwrap_or_default();
            MapView::new_projected_with_crs(&projected_position, resolution, crs)
        };

        Map::new(view, layers, messenger)
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use galileo_types::geo::{Datum, ProjectionType};
    use galileo_types::latlon;

    use super::*;
    use crate::layer::raster_tile_layer::RestTileProvider;
    use crate::layer::RasterTileLayer;
    use crate::TileSchema;

    fn test_tile_schema() -> TileSchema {
        TileSchema::web(18)
    }

    fn test_tile_layer() -> RasterTileLayer {
        let tile_schema = test_tile_schema();
        let tile_provider = RestTileProvider::new(|_| unimplemented!(), None, false);
        RasterTileLayer::new(tile_schema, tile_provider, None)
    }

    struct TestMessenger;
    impl Messenger for TestMessenger {
        fn request_redraw(&self) {
            todo!()
        }
    }

    fn test_messenger() -> impl Messenger {
        TestMessenger
    }

    #[test]
    fn constructs_map_with_default_parameters() {
        let map = MapBuilder::default().build();

        assert_eq!(map.view().position(), Some(latlon!(0.0, 0.0)));
        assert_eq!(map.view().resolution(), DEFAULT_RESOLUTION);
        assert_eq!(*map.view().crs(), Crs::EPSG3857);
        assert!(map.layers().is_empty());
        assert!(map.messenger.is_none());
    }

    #[test]
    fn with_position_sets_position() {
        let position1 = latlon!(10.0, 0.0);
        let position2 = latlon!(20.0, 10.1);

        let map = MapBuilder::default().with_position(position1).build();
        assert_relative_eq!(
            map.view().position().unwrap().lat(),
            position1.lat(),
            epsilon = 1e-6
        );
        assert_relative_eq!(
            map.view().position().unwrap().lon(),
            position1.lon(),
            epsilon = 1e-6
        );

        let map = MapBuilder::default()
            .with_position(position1)
            .with_position(position2)
            .build();
        assert_relative_eq!(
            map.view().position().unwrap().lat(),
            position2.lat(),
            epsilon = 1e-6
        );
        assert_relative_eq!(
            map.view().position().unwrap().lon(),
            position2.lon(),
            epsilon = 1e-6
        );
    }

    #[test]
    fn with_position_replaces_projected_position() {
        let position = latlon!(10.0, 0.0);
        let map = MapBuilder::default()
            .with_projected_position(Point2d::new(100.0, 100.0))
            .with_position(position)
            .build();
        assert_relative_eq!(
            map.view().position().unwrap().lat(),
            position.lat(),
            epsilon = 1e-6
        );
        assert_relative_eq!(
            map.view().position().unwrap().lon(),
            position.lon(),
            epsilon = 1e-6
        );
    }

    #[test]
    fn with_projected_position_sets_position() {
        let projection = Crs::EPSG3857
            .get_projection::<GeoPoint2d, Point2d>()
            .unwrap();
        let position1 = Point2d::new(10.0, 0.0);
        let projected1 = projection.unproject(&position1).unwrap();
        let position2 = Point2d::new(20.0, 10.1);
        let projected2 = projection.unproject(&position2).unwrap();

        let map = MapBuilder::default()
            .with_projected_position(position1)
            .build();
        assert_eq!(map.view().position(), Some(projected1));

        let map = MapBuilder::default()
            .with_projected_position(position1)
            .with_projected_position(position2)
            .build();
        assert_eq!(map.view().position(), Some(projected2));
    }

    #[test]
    fn with_projected_position_replaces_position() {
        let projection = Crs::EPSG3857
            .get_projection::<GeoPoint2d, Point2d>()
            .unwrap();
        let position = latlon!(10.0, 0.0);
        let projected = projection.unproject(&Point2d::new(100.0, 100.0)).unwrap();
        let map = MapBuilder::default()
            .with_position(position)
            .with_projected_position(Point2d::new(100.0, 100.0))
            .build();
        assert_eq!(map.view().position(), Some(projected));
    }

    #[test]
    fn with_resolution_sets_resolution() {
        let resolution1 = 100.0;
        let resolution2 = 200.0;

        let map = MapBuilder::default().with_resolution(resolution1).build();
        assert_relative_eq!(map.view().resolution(), resolution1);

        let map = MapBuilder::default()
            .with_resolution(resolution1)
            .with_resolution(resolution2)
            .build();
        assert_relative_eq!(map.view().resolution(), resolution2);
    }

    #[test]
    fn with_resolution_replaces_z_level() {
        let resolution = 100.0;
        let map = MapBuilder::default()
            .with_layer(test_tile_layer())
            .with_z_level(5)
            .with_resolution(resolution)
            .build();

        assert_relative_eq!(map.view().resolution(), resolution);
    }

    #[test]
    fn with_z_level_sets_resolution() {
        let z_level = 10;
        let map = MapBuilder::default()
            .with_layer(test_tile_layer())
            .with_z_level(z_level)
            .build();

        assert_relative_eq!(
            map.view().resolution(),
            test_tile_schema().lod_resolution(z_level).unwrap()
        );
    }

    #[test]
    fn with_z_level_sets_default_resolution_if_no_tile_schema() {
        let map = MapBuilder::default().with_z_level(3).build();

        assert_relative_eq!(map.view().resolution(), DEFAULT_RESOLUTION,);
    }

    #[test]
    fn with_z_level_sets_default_resolution_if_invalid_z_level() {
        let map = MapBuilder::default()
            .with_layer(test_tile_layer())
            .with_z_level(42)
            .build();

        assert_relative_eq!(map.view().resolution(), DEFAULT_RESOLUTION,);
    }

    #[test]
    fn with_z_level_replaces_resolution() {
        let z_level = 17;
        let map = MapBuilder::default()
            .with_layer(test_tile_layer())
            .with_resolution(100.0)
            .with_z_level(z_level)
            .build();

        assert_relative_eq!(
            map.view().resolution(),
            test_tile_schema().lod_resolution(z_level).unwrap()
        );
    }

    #[test]
    fn with_layer_adds_layers() {
        let map = MapBuilder::default().with_layer(test_tile_layer()).build();

        assert_eq!(map.layers.len(), 1);

        let map = MapBuilder::default()
            .with_layer(test_tile_layer())
            .with_layer(test_tile_layer())
            .build();

        assert_eq!(map.layers.len(), 2);
    }

    #[test]
    fn with_crs_sets_crs() {
        let crs1 = Crs::new(
            Datum::WGS84,
            ProjectionType::Other("laea lon_0=10 lat_0=52 x_0=4321000 y_0=3210000".into()),
        );
        let crs2 = Crs::EPSG3857;

        let map = MapBuilder::default().with_crs(crs1.clone()).build();
        assert_eq!(*map.view().crs(), crs1);

        let map = MapBuilder::default()
            .with_crs(crs1.clone())
            .with_crs(crs2.clone())
            .build();
        assert_eq!(*map.view().crs(), crs2);
    }

    #[test]
    fn with_messenger_sets_messenger() {
        let messenger = test_messenger();
        let map = MapBuilder::default().with_messenger(messenger).build();

        assert!(map.messenger.is_some());
    }
}
