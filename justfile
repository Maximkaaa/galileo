[private]
default:
  @just --list

# Compiles the web example and serves it using python dev server
web_example NAME:
  wasm-pack build web-example --release --target no-modules --target-dir target --features {{NAME}}
  cd web-example && python3 -m http.server

# Download font files needed for examples
get_fonts:
  if command -v wget > /dev/null; then wget https://Maximkaaa.github.io/fonts.zip; else curl -L https://Maximkaaa.github.io/fonts.zip -o fonts.zip; fi
  unzip fonts.zip -d galileo/examples/data
  rm fonts.zip

# Performs code formatting and all code checks that are done by CI
check:
  just fmt
  just check_clippy
  just check_wasm
  just check_typos
  just test

# Formats code according to the code style of the project
fmt:
  cargo +nightly fmt --all
  
# Runs clippy checks
check_clippy:
  cargo +stable clippy --all-targets --features geojson --features fontconfig-dlopen -- -D warnings

# Checks if wasm compilation works as expected
check_wasm:
  RUSTFLAGS='--cfg getrandom_backend="wasm_js"' cargo check --target wasm32-unknown-unknown --all-features

# Runs all tests
test:
  just unit_tests
  just doc_tests

# Runs unit tests
unit_tests:
  cargo test --features _tests,geojson,fontconfig-dlopen

# Runs doc tests
doc_tests:
  cargo test --doc --features geojson,fontconfig-dlopen

# Checks the source code for typos
check_typos:
  typos
