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
    @location(0) vertex_norm: vec2<f32>,
    @location(2) instance_position: vec3<f32>,
    @location(3) instance_size: f32,
    @location(4) instance_color: vec4<f32>,
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
    out.color = model.instance_color;
    var instance_position = transform.view_proj * vec4<f32>(model.instance_position, 1.0);
    var vertex_delta = vec4<f32>(model.vertex_norm * model.instance_size * transform.inv_screen_size * 2.0 * instance_position[3], 0.0, 0.0);

    out.clip_position = instance_position + vertex_delta;

    return out;
}


// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
