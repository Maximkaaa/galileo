[private]
default:
  @just --list

# Compiles the web example and serves it using python dev server
web_example NAME:
  wasm-pack build web-example --release --target no-modules --target-dir target --features {{NAME}}
  cd web-example && python3 -m http.server
