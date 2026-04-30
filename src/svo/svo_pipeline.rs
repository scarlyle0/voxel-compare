use crate::{input::camera::CameraBundle, svo::svo::SvoBuffers};

pub struct RayMarchRenderer {
    pub pipeline: wgpu::RenderPipeline,
    pub svo_bind_group: wgpu::BindGroup,
}

impl RayMarchRenderer {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        camera: &CameraBundle,
        svo: &SvoBuffers,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("ray_march.wgsl"));

        // Bind group 1: SVO info uniform + nodes storage
        let svo_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SVO BGL"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let svo_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SVO Bind Group"),
            layout: &svo_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: svo.info_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: svo.nodes_buffer.as_entire_binding() },
            ],
        });

        // pipeline
        let pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Ray March PL"),
                bind_group_layouts: &[
                    Some(&camera.bind_group_layout), // group 0: camera
                    Some(&svo_layout), // group 1: SVO
                ],
                immediate_size: 0,
            });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Ray March Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[], // fullscreen triangle — no vertex buffer needed
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None, // fullscreen — no culling
                ..Default::default()
            },
            depth_stencil: None, // ray marcher owns depth via colour output
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        RayMarchRenderer { pipeline, svo_bind_group }
    }
}
