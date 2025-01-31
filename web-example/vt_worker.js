importScripts("./pkg/galileo_web_example.js");

const {process_message, init_vt_worker} = wasm_bindgen;

async function init_worker() {
    await wasm_bindgen("./pkg/galileo_web_example_bg.wasm");
    
    console.log("Starting vt worker");
    init_vt_worker();

    self.onmessage = async event => {
        let result = await process_message(event.data);
        self.postMessage(result, null, [result]);
    }
}

init_worker();
