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
    @location(0) position: vec3<f32>,
    @location(1) normal: vec2<f32>,
    @location(2) color: vec4<u32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
    @location(10) bundle_opacity: f32,
) -> VertexOutput {
    var out: VertexOutput;

    var color = vec4<f32>(model.color) / 255.0;
    color[3] = color[3] * bundle_opacity;
    out.color = color;

    var point_position = transform.view_proj * vec4<f32>(model.position, 1.0);
    var vertex_delta = vec4<f32>(model.normal * transform.inv_screen_size * point_position[3] * 2.0, 0.0, 0.0);

    out.clip_position = point_position + vertex_delta;

    return out;
}


// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
