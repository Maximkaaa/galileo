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
