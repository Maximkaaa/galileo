This example is different from other wasm examples, as it shows how the Galileo library will be used in the future
to create web-applications. It shows the basic idea of using FFI to bridge calls between JS and wasm-compiled
Galileo.

To run this example, first build Galileo as wasm package:
```
wasm-pack build --release galileo
```

Make sure that you use `wasm-opt` of version higher than 119.

After the package is created, use `npm` and `webpack` to build and run the example:

```
npm install
npm run build
npm run start
```

