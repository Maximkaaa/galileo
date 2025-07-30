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
    @location(1) tex_coord: vec2<f32>,
}

struct SetInput {
    @location(10) opacity: f32,
    @location(11) anchor: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) opacity: f32,
};

@vertex
fn vs_main(
    vertex: VertexInput,
    screen_set: SetInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coord = vertex.tex_coord;
    out.opacity = screen_set.opacity;

    var point_position = transform.view_proj * vec4<f32>(screen_set.anchor, 1.0);
    var position_normalized = point_position / point_position[3];

    var vertex_delta = vec4<f32>(vertex.position * transform.inv_screen_size * 2.0, 0.0, 0.0);

    out.clip_position = position_normalized + vertex_delta;

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
