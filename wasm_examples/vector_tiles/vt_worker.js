importScripts("./pkg/vector_tiles_example.js");

const {process_message, init_vt_worker} = wasm_bindgen;

async function init_worker() {
    await wasm_bindgen("./pkg/vector_tiles_example_bg.wasm");
    
    init_vt_worker();

    self.onmessage = async event => {
        let result = await process_message(event.data);
        self.postMessage(result, null, [result]);
    }
}

init_worker();
