# Changelog

## [Unreleased]

## [0.3.0](https://github.com/Maximkaaa/galileo/compare/galileo-v0.2.1...galileo-v0.3.0)

### ⛰️ Features


- *(cache)* File path modifier func (optional) ([#253](https://github.com/Maximkaaa/galileo/pull/253)) - ([abc7689](https://github.com/Maximkaaa/galileo/commit/abc7689eb2e577aa62438a3b0fdfdb2097c64923))
- *(egui)* Pass `CreationContext` to `AppBuilder` ([#245](https://github.com/Maximkaaa/galileo/pull/245)) - ([ee9c28c](https://github.com/Maximkaaa/galileo/commit/ee9c28cf489dd391b42ba302fca69e1a7755ead1))
- *(egui)* Set custom `app_name` in galileo_egui::init (optional) ([#246](https://github.com/Maximkaaa/galileo/pull/246)) - ([cd392a2](https://github.com/Maximkaaa/galileo/commit/cd392a2e4d431033bb2a1ca509212aa77b4c2cb3))
- *(examples)* `LineString` example - ([fb78364](https://github.com/Maximkaaa/galileo/commit/fb783640828a23c050c42656f1a84a4c83e8285d))
- Allow providing offset to render bundles for drawing - ([e6ae8c5](https://github.com/Maximkaaa/galileo/commit/e6ae8c5ad09a4030abd561ba85cce2507eb1ad4a))
- Support HiDPI scaling in galileo-egui ([#220](https://github.com/Maximkaaa/galileo/pull/220)) - ([ac1fd97](https://github.com/Maximkaaa/galileo/commit/ac1fd97215ac2aedbc10df2f4f87511c319d7917))
- Provide a way to get messenger from egui map state - ([6802c5c](https://github.com/Maximkaaa/galileo/commit/6802c5c8e58e98ff30ef1e7ef51f4729d03fcb83))
- Feat(example) switch tile layers at runtime - ([046b92d](https://github.com/Maximkaaa/galileo/commit/046b92df78edc3d01927d1bcdf6a740e9ab9f43a))
- Add `map_geo_to_screen` - ([c3dfc3d](https://github.com/Maximkaaa/galileo/commit/c3dfc3d56a10dc07eb06bcfdb64f2bcc96962c7b))
- Add horizon effect for the wgpu renderer - ([704b23b](https://github.com/Maximkaaa/galileo/commit/704b23b76531f5faa56b8e822c81043897cfae6f))

### 🐛 Bug Fixes


- *(example)* Remove query from file path - ([eed029c](https://github.com/Maximkaaa/galileo/commit/eed029cb3b56771a5221e1650625b656a356f1ce))
- *(example)* Fix vector tiles style switch - ([bdb4416](https://github.com/Maximkaaa/galileo/commit/bdb4416abe4435d5095651fa26a48bf8bb709871))
- *(examples)* Remove file cache for vt layers in examples - ([e11ac55](https://github.com/Maximkaaa/galileo/commit/e11ac5563937fd2c3954ef54f265b389f65e15cd))

### 🚜 Refactor


- Remove ScreenSetInstance in favour of common DisplayInstance - ([3565fe7](https://github.com/Maximkaaa/galileo/commit/3565fe738edd9654f9f2519a002191c2d7c19e03))

### 📚 Documentation


- Doc examples for `_modifier` functions - ([64e4853](https://github.com/Maximkaaa/galileo/commit/64e48532f36e1f4e5ab753c0f4f420629b28a490))

### ⚡ Performance


- Do not duplicate same vector tiles in GPU memory - ([3678449](https://github.com/Maximkaaa/galileo/commit/367844996b7198279d848db801daca4e4914c5ca))

### ⚙️ Miscellaneous Tasks


- Update to egui `0.32` - ([3f27317](https://github.com/Maximkaaa/galileo/commit/3f273175188a030f0c38adf5db4973264e78369e))
- Remove unused assert_matches dependency - ([2d16acc](https://github.com/Maximkaaa/galileo/commit/2d16acc5f0867912f21da140144ad1c1a3bd90a5))
- Remove unneeded lazy_static dependency - ([1426427](https://github.com/Maximkaaa/galileo/commit/1426427a0faf972fe313e23e931c97af532e32b6))
- Remove unneeded geo dependency - ([4b83e3e](https://github.com/Maximkaaa/galileo/commit/4b83e3e18e41a8238803e066f52253faf4d3b0d4))


## [0.2.1](https://github.com/Maximkaaa/galileo/compare/galileo-v0.2.0...galileo-v0.2.1)

### ⛰️ Features


- Wrap tiles along x axis - ([b238ee9](https://github.com/Maximkaaa/galileo/commit/b238ee98a9d4b12bdf024eeb4c3d31f890f06736))
- Add `map_to_screen` and `map_to_screen_clipped` - ([781843b](https://github.com/Maximkaaa/galileo/commit/781843b25f5aa777ce271b20cccedb89b470481f))

### 📚 Documentation


- Fix links RasterTileProvider -> RasterTileLoader in docs - ([d6ef150](https://github.com/Maximkaaa/galileo/commit/d6ef1502f1133f99a21cdfa27c5cc99a365fb787))
- Update links to web examples in readmy - ([e0f17d1](https://github.com/Maximkaaa/galileo/commit/e0f17d1f2ae229a3d4eafe561baeba99cfed69b9))

### ⚙️ Miscellaneous Tasks


- Improve logging of event processor - ([f955b4c](https://github.com/Maximkaaa/galileo/commit/f955b4c0224b859c0cffe8c9d5752f63817e9202))

