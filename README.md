**Galileo** is a general purpose cross-platform geo-rendering library.

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
Supporting coordinate transformations for layers on the fly is planned but not yet
implemented.

## Cross-platform

At the moment Galileo uses `wgpu` backend to render the map. This means that
it can be used on any platform that `wgpu` supports:
* All major desktop platforms: Linux, MacOS, Windows
* Mobile platforms: Android, iOS
* Web though compiling to WASM (and using either WebGL or newer and cooler WebGPU)

Still, the backend is not integral part of the Galileo design, so we will probably
try other promising backends (like [vello](https://github.com/linebender/vello)).

# Features

Galileo is an active WIP, here is the list of the features that are already present
(mostly as POC at the moment):
* raster tile layers
* vector tile layers with styling
* vector geo-data layers (feature layers) with styling
* user-input handling on layers (mouse only at the moment, touch is WIP)

# Web examples

![Raster tile layer](https://maximkaaa.github.io/galileo/osm_256.png)
![Vector tile layer](https://maximkaaa.github.io/galileo/vector_tiles_256.png)
![Feature layer](https://maximkaaa.github.io/galileo/feature_layer_256.png)

* [Raster tile layer (OSM)](https://maximkaaa.github.io/galileo/simple_map/)
* [Vector tile layer (Maplibre)](https://maximkaaa.github.io/galileo/vector_tiles/)
  * Use buttons at the top to change the style of the map
  * Click on any object to get information about it
* [Feature layer](https://maximkaaa.github.io/galileo/countries/)
  * NOTE! Contains large dataset (~10 MB), might take some time to load
  * Draws 250 countries' borders, consisting of ~4000 polygons with ~500K vertices
  * Move mouse pointer to highlight any country

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

# License

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

# Contributing

The project is in architecture design stage ATM. If you have a suggestion about high-level
stuff, please, open an issue and lay out your ideas. Also, if you want to create
examples and test stuff on Android, MacOS or iOS, PRs are welcome.

If you want to report a bug, be patient. You will have plenty opportunities in the future.