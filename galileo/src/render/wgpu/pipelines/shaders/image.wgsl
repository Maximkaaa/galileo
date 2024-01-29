// Vertex shader

struct ViewUniform {
    view_proj: mat4x4<f32>,
    view_rotation: mat4x4<f32>,
    inv_screen_size: vec2<f32>,
    resolution: f32,
}

@group(0) @binding(0)
var<uniform> transform: ViewUniform;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) opacity: f32,
    @location(2) tex_coord: vec2<f32>,
    @location(3) offset: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) opacity: f32,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coord = model.tex_coord;

    var point_position = transform.view_proj * vec4<f32>(model.position, 0.0, 1.0);
    var vertex_delta = vec4<f32>(model.offset * transform.inv_screen_size * point_position[3] * 2.0, 0.0, 0.0);

    out.clip_position = point_position + vertex_delta;
    out.opacity = model.opacity;

    return out;
}


// Fragment shader

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = textureSample(t_diffuse, s_diffuse, in.tex_coord);
    color[3] = color[3] * in.opacity;

    if color[3] == 0.0 {
        discard;
    }

    return color;
}
