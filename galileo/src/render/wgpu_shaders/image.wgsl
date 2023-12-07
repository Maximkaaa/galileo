// Vertex shader

struct ViewUniform {
    view_proj: mat4x4<f32>,
    resolution: f32,
}

@group(1) @binding(0)
var<uniform> transform: ViewUniform;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) opacity: f32,
    @location(2) tex_coord: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(1) tex_coord: vec2<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coord = model.tex_coord;
    out.clip_position = transform.view_proj * vec4<f32>(model.position, 0.0, 1.0);

    return out;
}


// Fragment shader

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.tex_coord);
}
