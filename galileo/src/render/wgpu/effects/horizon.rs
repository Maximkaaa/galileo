use galileo_types::cartesian::CartesianPoint3d;
use nalgebra::Rotation3;
use wgpu::util::DeviceExt;
use wgpu::{BindGroupLayout, Device, Queue, RenderPass, RenderPipeline, TextureFormat};

use crate::render::wgpu::pipelines::default_targets;
use crate::render::wgpu::{pipelines, WgpuVertexBuffers};
use crate::{Color, MapView};

/// Configuration for the horizon effect.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct HorizonOptions {
    horizon_color: Color,
    sky_color: Color,
    ground_break_z: f32,
    sky_break_z: f32,
}

impl Default for HorizonOptions {
    fn default() -> Self {
        Self {
            horizon_color: Color::WHITE,
            sky_color: Color::from_hex("#87CEEB"),
            ground_break_z: 0.05,
            sky_break_z: 0.3,
        }
    }
}

impl HorizonOptions {
    /// Sets the color of the mist at the horizon line.
    ///
    /// Default color is white.
    pub fn with_horizon_color(mut self, color: Color) -> Self {
        self.horizon_color = color;
        self
    }

    /// Sets the color of the sky.
    ///
    /// Default color is sky blue.
    pub fn with_sky_color(mut self, color: Color) -> Self {
        self.sky_color = color;
        self
    }

    /// Sets the width of the misty part of horizon below the horizon line.
    ///
    /// The default value is `0.05`. Use larger value for wider horizon line.
    pub fn with_ground_break_z(mut self, z: f32) -> Self {
        self.ground_break_z = z;
        self
    }

    /// Sets the width of the transition part from horizon line to the sky.
    ///
    /// The default value is `0.3`. The smaller this value, the faster the color of sky get
    /// saturated above the horizon.
    pub fn with_sky_break_z(mut self, z: f32) -> Self {
        self.sky_break_z = z;
        self
    }
}

pub struct HorizonPipeline {
    wgpu_pipeline: RenderPipeline,
    buffers: WgpuVertexBuffers,
    binding: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
}

impl HorizonPipeline {
    pub fn create(
        device: &Device,
        format: TextureFormat,
        map_view_layout: &BindGroupLayout,
        options: HorizonOptions,
    ) -> Self {
        let buffers = [HorizonVertex::wgpu_desc()];
        let shader = device.create_shader_module(wgpu::include_wgsl!("./shaders/horizon.wgsl"));

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Horizon uniform buffer"),
            size: size_of::<HorizonUniform>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let horizon_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: None,
            });

        let horizon_binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &horizon_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("horizon_bind_group"),
        });

        let targets = default_targets(format);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[map_view_layout, &horizon_bind_group_layout],
            push_constant_ranges: &[],
        });
        let desc =
            pipelines::default_pipeline_descriptor(&layout, &shader, &targets, &buffers, false);
        let wgpu_pipeline = device.create_render_pipeline(&desc);

        Self {
            wgpu_pipeline,
            binding: horizon_binding,
            uniform_buffer,
            buffers: Self::generate_buffers(device, options),
        }
    }

    pub fn render<'a>(
        &'a self,
        view: &MapView,
        queue: &Queue,
        render_pass: &mut RenderPass<'a>,
        bundle_index: u32,
    ) {
        render_pass.set_pipeline(&self.wgpu_pipeline);

        render_pass.set_bind_group(1, &self.binding, &[]);
        let Some(position) = view.projected_position() else {
            return;
        };
        let horizon_distance = (view.size().half_width().min(view.size().half_height())
            * view.resolution()
            * view.horizon_k()) as f32;
        let translate = nalgebra::Translation3::new(position.x() as f32, position.y() as f32, 0.0)
            .to_homogeneous();
        let scale = nalgebra::Scale3::new(horizon_distance, horizon_distance, horizon_distance)
            .to_homogeneous();
        let rotation_z = Rotation3::new(nalgebra::Vector3::new(
            0f32,
            0f32,
            -view.rotation_z() as f32,
        ))
        .to_homogeneous();
        let mtx = translate * scale * rotation_z;

        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[HorizonUniform {
                transform: mtx.data.0,
            }]),
        );

        render_pass.set_vertex_buffer(0, self.buffers.vertex.slice(..));
        render_pass.set_index_buffer(self.buffers.index.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(
            0..self.buffers.index_count,
            0,
            bundle_index..(bundle_index + 1),
        );
    }

    fn generate_buffers(device: &Device, options: HorizonOptions) -> WgpuVertexBuffers {
        let (mut vertices, mut indices) = Self::generate_ring(
            0.0,
            options.sky_break_z,
            options.horizon_color,
            options.sky_color,
            0,
        );
        let (mut sky_vertices, mut sky_indices) = Self::generate_ring(
            options.sky_break_z,
            1.0,
            options.sky_color,
            options.sky_color,
            vertices.len() as u16,
        );
        vertices.append(&mut sky_vertices);
        indices.append(&mut sky_indices);

        let (mut ground_vertices, mut ground_indices) = Self::generate_ring(
            -options.ground_break_z,
            0.0,
            options.horizon_color.with_alpha(0),
            options.horizon_color,
            vertices.len() as u16,
        );
        vertices.append(&mut ground_vertices);
        indices.append(&mut ground_indices);

        let index_count = indices.len() as u32;
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Horizon vertex buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Horizon index buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        WgpuVertexBuffers {
            vertex: vertex_buffer,
            index: index_buffer,
            index_count,
        }
    }

    fn generate_ring(
        z0: f32,
        z1: f32,
        color0: Color,
        color1: Color,
        base_index: u16,
    ) -> (Vec<HorizonVertex>, Vec<u16>) {
        const SEGMENTS: usize = 32;

        let start_angle = std::f32::consts::FRAC_PI_4;

        let mut vertices = vec![
            HorizonVertex {
                position: [start_angle.cos(), start_angle.sin(), z0],
                color: color0.to_u8_array(),
            },
            HorizonVertex {
                position: [start_angle.cos(), start_angle.sin(), z1],
                color: color1.to_u8_array(),
            },
        ];
        let mut indices = vec![];
        let step = std::f32::consts::PI * 2.0 / SEGMENTS as f32;
        for i in 1..SEGMENTS {
            let first = vertices.len() as u16 - 2 + base_index;
            vertices.push(HorizonVertex {
                position: [
                    (step * i as f32 + start_angle).cos(),
                    (step * i as f32 + start_angle).sin(),
                    z0,
                ],
                color: color0.to_u8_array(),
            });
            vertices.push(HorizonVertex {
                position: [
                    (step * i as f32 + start_angle).cos(),
                    (step * i as f32 + start_angle).sin(),
                    z1,
                ],
                color: color1.to_u8_array(),
            });

            indices.append(&mut vec![first, first + 1, first + 2]);
            indices.append(&mut vec![first + 1, first + 3, first + 2]);
        }

        let last = vertices.len() as u16 - 2 + base_index;
        indices.append(&mut vec![
            last,
            last + 1,
            base_index,
            last + 1,
            base_index + 1,
            base_index,
        ]);

        (vertices, indices)
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct HorizonVertex {
    pub position: [f32; 3],
    pub color: [u8; 4],
}

impl HorizonVertex {
    fn wgpu_desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<HorizonVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Uint8x4,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct HorizonUniform {
    transform: [[f32; 4]; 4],
}
