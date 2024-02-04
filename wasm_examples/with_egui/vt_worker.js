importScripts("./pkg/with_egui.js");

const { load_tile, init_vt_worker } = wasm_bindgen;

async function init_worker() {
  await wasm_bindgen("./pkg/with_egui_bg.wasm");

  init_vt_worker();

  self.onmessage = async (event) => {
    let result = await load_tile(event.data);
    self.postMessage(result, null, [result]);
  };
}

init_worker();
