[private]
default:
  @just --list

# Compiles the web example and serves it using python3 dev server
web_example NAME:
  just {{ if NAME == "raster_tiles" { "npm_web_example" } else { "python_web_example" } }} {{NAME}}

[private]
npm_web_example NAME:
  wasm-pack build --release galileo 
  cd wasm_examples/{{NAME}} && npm install && npm run build && npm run start

[private]
python_web_example NAME:
  wasm-pack build wasm_examples/{{NAME}} --target no-modules --release --target-dir target
  cd wasm_examples/{{NAME}} && python3 -m http.server
