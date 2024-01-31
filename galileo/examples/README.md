<table>
<thead>
<tr>
    <th>Example</th>
    <td>Image</td>
    <td>Description</td>
</tr>
</thead>
<tbody>
<tr>
<td>

[raster_tiles](./raster_tiles.rs)

</td>
<td>

![i](https://maximkaaa.github.io/galileo/osm_256.png)

</td>
<td>

* Create a map with one raster tile layer (OSM)
* Set initial map position and zoom
 
</td>
</tr>
<tr>
<td>

[vector_tiles](./vector_tiles.rs)

</td>
<td>

![i](https://maximkaaa.github.io/galileo/vector_tiles_256.png)

</td>
<td>

* Create a map with one vector tile layer (MapLibre)
* Configure layer styling with the style file
* Get information about objects in the tiles by click

</td>
</tr>
<tr>
<td>

[feature_layers](./feature_layers.rs)

</td>
<td>

![i](https://maximkaaa.github.io/galileo/feature_layers_256.png)

</td>
<td>

* Create a map with feature layers without a tile base map
* Use symbols to set advanced styles for features based on their properties
* Change properties of the features when hovering mouse over them
* Modify how the features are displayed based on changed properties
* Get information about features by click

</td>
</tr>
<tr>
<td>

[lambert](./lambert.rs)

</td>
<td>

![i](https://maximkaaa.github.io/galileo/lambert_sm.png)

</td>
<td>

* Render feature layer in Lambert Equal Area projection
* Get and update features on cursor hover

</td>
</tr>
<tr>
<td>

[many_points](./many_points.rs)

</td>
<td>

![i](https://maximkaaa.github.io/galileo/many_points.gif)

</td>
<td>

* Render ~3_000_000 3D points over a map

</td>
</tr>
<tr>
<td>

[las](./las.rs)

</td>
<td>

![i](https://maximkaaa.github.io/galileo/bridge.gif)

</td>
<td>

* Read ~19_000_000 points from a LAS data set (lidar laser scanning result)
* Render all the points without pre-grouping (to demonstrate renderer performance limits)
* NOTE: before running the example, load the dataset. Read module-level docs in the example file.
* NOTE 2: You probably want to run this example in `--release` mode

</td>
</tr>
<tr>
<td>

[render_to_file](./render_to_file.rs)

</td>
<td>

You can generate an image yourself running this example

</td>
<td>

* Run a map without a window
* Load GEOJSON file to a feature layer
* Render the map to a `.png` file

</td>
</tr>
<tr>
<td>

[georust](./georust.rs)

</td>
<td>

![i](https://maximkaaa.github.io/galileo/georust.png)

</td>
<td>

* Load features as `geo-types` geometries using `geo-zero` crate
* Display the features with pin images

</td>
</tr>
</tbody>
</table>