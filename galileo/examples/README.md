Run all simple examples with:
```shell
cargo run --example <example_name>
```

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

- Create a map with one raster tile layer (OSM)
- Set initial map position and zoom

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

- Create a map with one vector tile layer (MapLibre)
- Configure layer styling with the style file
- Get information about objects in the tiles by click

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

- Create a map with feature layers without a tile base map
- Use symbols to set advanced styles for features based on their properties
- Change properties of the features when hovering mouse over them
- Modify how the features are displayed based on changed properties
- Get information about features by click
- Hide/show features by clicking on them

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

- Render feature layer in Lambert Equal Area projection
- Get and update features on cursor hover

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

- Render ~3_000_000 3D points over a map

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

- Read ~19_000_000 points from a LAS data set (lidar laser scanning result)
- Render all the points without pre-grouping (to demonstrate renderer performance limits)
- NOTE: before running the example, load the dataset. Read module-level docs in the example file.
- NOTE 2: You probably want to run this example in `--release` mode

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

- Run a map without a window
- Load GEOJSON file to a feature layer
- Render the map to a `.png` file

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

- Load features as `geo-types` geometries using `geo-zero` crate
- Display the features with pin images

</td>
</tr>
<tr>
<td>

[with_egui](./with_egui)

</td>
<td>

![i](https://maximkaaa.github.io/galileo/with_egui.png)

</td>
<td>

- Same as the raster tiles example, but with support for [egui](https://www.egui.rs/).

</td>
</tr>
<tr>
<td>

[highlight_features](./highlight_features.rs)

</td>
<td>

![i](https://maximkaaa.github.io/galileo/highlight_features.png)

</td>
<td>

- Get and update features on cursor hover
- Show a different pin image based on feature state

</td>
</tr>
<tr>
<td>

[linestring](./linestring.rs)

</td>
<td>

![i](https://private-user-images.githubusercontent.com/47693/468604646-b2ea71c2-48e2-4108-aef9-d9b52d2c522f.png?jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJnaXRodWIuY29tIiwiYXVkIjoicmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbSIsImtleSI6ImtleTUiLCJleHAiOjE3NTMxMjE5MTUsIm5iZiI6MTc1MzEyMTYxNSwicGF0aCI6Ii80NzY5My80Njg2MDQ2NDYtYjJlYTcxYzItNDhlMi00MTA4LWFlZjktZDliNTJkMmM1MjJmLnBuZz9YLUFtei1BbGdvcml0aG09QVdTNC1ITUFDLVNIQTI1NiZYLUFtei1DcmVkZW50aWFsPUFLSUFWQ09EWUxTQTUzUFFLNFpBJTJGMjAyNTA3MjElMkZ1cy1lYXN0LTElMkZzMyUyRmF3czRfcmVxdWVzdCZYLUFtei1EYXRlPTIwMjUwNzIxVDE4MTMzNVomWC1BbXotRXhwaXJlcz0zMDAmWC1BbXotU2lnbmF0dXJlPTlhYTE1N2Q2NWY4ZTg3ZmVhYTA5MTY0OWY1YjU5YjEyODY5NjA2YjEwMjMxYTEzZTcxNmM0Y2IyYTY2YjAzOTMmWC1BbXotU2lnbmVkSGVhZGVycz1ob3N0In0.JvoXZIbkPuJ5Uu2L8Jx1p07gbVvmZpBrkmrOFqShTVg)

</td>
<td>

- Renders a `LineString` defined in a geojson `FeatureCollection` as a `Contour` in a `FeatureLayer`. Very similar to
MapLibre GL example ['Add a GeoJSON line'](https://maplibre.org/maplibre-gl-js/docs/examples/add-a-geojson-line/)

</td>
</tr>
</tbody>
</table>
