set dotenv-load := true
export VT_API_KEY := env('VT_API_KEY', "")

[private]
default:
  @just --list

# Compiles the web example and serves it using python dev server
[group('Web Examples')]
run_web_example NAME profile="release":
  just build_web_example {{NAME}} {{profile}}
  cd target/web_examples/{{NAME}} && python3 -m http.server

# Compiles given web example into the folder `target/web_examples/<NAME>`
[group('Web Examples')]
build_web_example NAME profile="release" $VT_API_KEY=`echo $VT_API_KEY`:
  @ echo {{ if VT_API_KEY == "" { "WARNING: VT_API_KEY is not set. Vector tile layers will not work." } else { "VT_API_KEY is set" } }}

  rm -rf target/web_examples/{{NAME}}
  RUSTFLAGS='--cfg getrandom_backend="wasm_js"' wasm-pack build web-example --{{profile}} --target no-modules --target-dir target --features {{NAME}}
  mkdir -p target/web_examples/{{NAME}}
  cp web-example/index.html target/web_examples/{{NAME}}
  cp web-example/vt_worker.js target/web_examples/{{NAME}}
  cp -r web-example/pkg target/web_examples/{{NAME}}
  rm -f target/web_examples/{{NAME}}/pkg/.gitignore

# Build all web examples into `target/web_examples`
[group('Web Examples')]
build_all_web_examples api_key=`echo $VT_API_KEY`:
  just build_web_example raster_tiles
  just build_web_example feature_layers
  just build_web_example egui_app
  just build_web_example georust
  just build_web_example highlight_features
  just build_web_example lambert
  just build_web_example many_points
  just build_web_example vector_tiles
  just build_web_example add_remove_features

[private]
[group('Web Examples')]
publish_web_examples:
  VT_API_KEY=$PUBLIC_VT_API_KEY just build_all_web_examples
  yes | cp -rf target/web_examples ../Maximkaaa.github.io/galileo
  cd ../Maximkaaa.github.io/galileo && git checkout master && git pull && git add . && git commit -m "Update Galileo web examples" && git push

# Performs code formatting and all code checks that are done by CI
[group('Checks')]
check:
  just fmt
  just check_clippy
  just check_wasm
  just check_typos
  just test

# Formats code according to the code style of the project
[group('Checks')]
fmt:
  cargo +nightly fmt --all
  
# Runs clippy checks
[group('Checks')]
check_clippy:
  cargo +stable clippy --all-targets --features geojson --features fontconfig-dlopen -- -D warnings

# Checks if wasm compilation works as expected
[group('Checks')]
check_wasm:
  RUSTFLAGS='--cfg getrandom_backend="wasm_js"' cargo check --target wasm32-unknown-unknown --all-features

# Runs all tests
[group('Checks')]
test:
  just unit_tests
  just doc_tests

# Runs unit tests
[group('Checks')]
unit_tests:
  cargo test --features _tests,geojson,fontconfig-dlopen

# Runs doc tests
[group('Checks')]
doc_tests:
  cargo test --doc --features geojson,fontconfig-dlopen

# Checks the source code for typos
[group('Checks')]
check_typos:
  typos

# Download font files needed for examples
get_fonts:
  if command -v wget > /dev/null; then wget https://Maximkaaa.github.io/fonts.zip; else curl -L https://Maximkaaa.github.io/fonts.zip -o fonts.zip; fi
  unzip fonts.zip -d galileo/examples/data
  rm fonts.zip

