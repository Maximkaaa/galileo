import * as galileo from "galileo";

let builder = galileo.MapBuilder
    .new()
    .with_raster_tiles((tile_index) => `https://tile.openstreetmap.org/${tile_index.z}/${tile_index.x}/${tile_index.y}.png`);

builder.build_into(document.body)
    .then(async (map) => {
        await map.run();
    });
