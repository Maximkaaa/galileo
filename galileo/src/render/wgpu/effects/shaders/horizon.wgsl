// Vertex shader

struct ViewUniform {
    view_proj: mat4x4<f32>,
    view_rotation: mat4x4<f32>,
    inv_screen_size: vec2<f32>,
    resolution: f32,
}

struct HorizonUniform {
    transform: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> transform: ViewUniform;

@group(1) @binding(0)
var<uniform> horizon: HorizonUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<u32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    var color = vec4<f32>(model.color) / 255.0;
    out.color = color;

    let map_position = horizon.transform * vec4<f32>(model.position, 1.0);
    let vertex_position = transform.view_proj * map_position;

    out.clip_position = vertex_position;

    return out;
}


// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}

