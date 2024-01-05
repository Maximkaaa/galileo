// Vertex shader

struct ViewUniform {
    view_proj: mat4x4<f32>,
    resolution: f32,
}

@group(0) @binding(0)
var<uniform> transform: ViewUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) norm: vec2<f32>,
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
    out.color = model.color;

    var vertex_position = transform.view_proj * vec4<f32>(model.position, 1.0);
    var norm = vec4<f32>(model.norm * transform.resolution * vertex_position[3] * 2.0, 0.0, 0.0);
    out.clip_position = vertex_position + norm;

    return out;
}


// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
