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
    @location(1) color: vec4<u32>,
    @location(10) bundle_opacity: f32,
    @location(11) bundle_offset: vec3<f32>,
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
    color[3] = color[3] * model.bundle_opacity;
    out.color = color;
    out.clip_position = transform.view_proj * vec4<f32>(model.position + model.bundle_offset, 1.0);

    return out;
}

// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
