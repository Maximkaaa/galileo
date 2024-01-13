**Galileo** is a general purpose cross-platform geo-rendering library.

# Web examples

![Raster tile layer](https://maximkaaa.github.io/galileo/osm_256.png)
![Lambert projection](https://maximkaaa.github.io/galileo/lambert_sm.png)
![Feature layers](https://maximkaaa.github.io/galileo/countries_sm.png)
![Many points](https://maximkaaa.github.io/galileo/many_points.gif)

* [Raster tile layer (OSM)](https://maximkaaa.github.io/galileo/raster_tiles/)
* [Vector tile layer (Maplibre)](https://maximkaaa.github.io/galileo/vector_tiles/)
  * Use buttons at the top to change the style of the map
  * Click on any object to get information about it
* [Feature layer](https://maximkaaa.github.io/galileo/feature_layers/)
  * NOTE! Contains large dataset (~16 MB), might take some time to load
  * Draws 250 countries' borders, consisting of ~4000 polygons with ~500K vertices, plus 40K city points
  * Move mouse pointer to highlight any country, click on a country to write its name into console
* [Map in Lambert Equal Area projection](https://maximkaaa.github.io/galileo/lambert/)
  * Takes data set (country borders) in Mercator projection and draws it to the map in LAEA projection
* [Very many points](https://maximkaaa.github.io/galileo/many_points/)
  * Enjoy 3.6 million points heat up your room with GPU.

# Overview

## General purpose

Architecture of **Galileo** is designed specifically to allow different modes of use:
* client GUI mapping applications
* server modules
* CLI tools

This does bring certain challenges and adds to complexity. For example, caching
of loaded and rendered objects works completely differently in server in client
applications. **Galileo** is designed in such a way as to allow the user of the
library to make all the relevant decisions and not to force one way.

It also does not make any assumptions about tile layer schemas, CRS and datums.
Feature layers support transparent re-projecting into the map CRS (support for
projecting tile layers is planned but not implemented yet).

## Cross-platform

At the moment Galileo uses `wgpu` backend to render the map. This means that
it can be used on any platform that `wgpu` supports:
* All major desktop platforms: Linux, MacOS, Windows
* Mobile platforms: Android, iOS
* Web though compiling to WASM (and using either WebGL or newer and cooler WebGPU)

Still, the backend is not integral part of the Galileo design, so we will probably
try other promising backends (like [vello](https://github.com/linebender/vello)).

## Features

Galileo is an active WIP, here is the list of the features that are already present:
* raster tile layers
* vector tile layers with styling
* vector geo-data layers (feature layers) with styling
* 3d view and 3d object rendering
* user-input handling on layers (mouse only at the moment, touch is WIP)
* support for different projections and tile schemes
* high performance

# Roadmap

There are so many things that we all want from mapping engine, but it's impossible to have
them all done at the same time. So here's our current plan and priorities:

### v0.1 - usabilitification

* Architecture and basic building blocks of the library
* Basic styling to be able to use Galileo for simple but useful applications
* Support main source types (TMS tiles, 2d geometries, MVT)
* Basic support for projections

### v0.2 - bueautification

* Advanced styling for features and vector tiles (image points, gradients, etc.)
* Support for more source and style formats
* Text label rendering
* Advanced feature layers (clusters, heatmaps, etc.)

### v0.3 - 3d-fication

* 3d globe, atmosphere and stars around to make your dark hours brighter
* Terrain rendering to draw every mountain you climbed
* 3d models to put your house on those mountains
* Advanced support for projections and CRSs

# Running examples

There are examples in `galileo` crate that can be run with `cargo run --example <name>`
command. Web examples are separate creates in `wasm_examples` directory. These are
excluded from the workspace (because Cargo does not like cross-platform workspaces).
To run those you will need to [install wasm-pack](https://rustwasm.github.io/wasm-pack/installer/):

```shell
wasm-pack build wasm_examples/countries --target no-modules --release
```

After that open `index.html` in your browser (must be served from `localhost`, use your
favourite developer server).

## Cross-compile from Linux to Windows

Install the target:

```shell
rustup target add x86_64-px-window-gnu
```

Install cross-linker. For Debian/Ubuntu:

```
sudo apt-get install mingw-w64
```

And then build it:

```shell
cargo build --target x86_64-px-windows-gnu
```

# Sponsoring

If you think this library can be useful to you or someone you love, consider supporting its development. Sponsoring
comes with additional advantages:
* Increase development speed. All funds will be spent on development.
* Make your needs our priority.
* See your logo on the project's page.

Please, contact the maintainer of the project with any questions related to sponsoring.

# License

You can use this library without any worries as it is licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

# Contributing

Feature requests, pull requests, bug reports, comments, questions and discussion are welcome. Please, follow the code
of conduct when contributing.

Note, that since the library is still in early stages of development, any part may change at any moment. So before
starting any major undertaking with it or within it, open a discussion to sync your ideas with others' ideas.