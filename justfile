[private]
default:
  @just --list

# Compiles the web example and serves it using python dev server
web_example NAME:
  wasm-pack build web-example --release --target no-modules --target-dir target --features {{NAME}}
  cd web-example && python3 -m http.server

# Download font files needed for examples
get_fonts:
  wget https://Maximkaaa.github.io/fonts.zip
  unzip fonts.zip -d galileo/examples/data
  rm fonts.zip
