use winit::window::Window;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use crate::texture;
use crate::controller::CameraController;
use winit::{
    keyboard::KeyCode,
    event_loop::ActiveEventLoop,
};
use crate::texture::Texture;

// CPU representation of vertex
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

const VERTICES: &[Vertex] = &[
    Vertex { position: [-0.5, -0.5,  0.5], tex_coords: [0.0, 0.0] },
    Vertex { position: [ 0.5, -0.5,  0.5], tex_coords: [1.0, 0.0] },
    Vertex { position: [ 0.5,  0.5,  0.5], tex_coords: [1.0, 1.0] },
    Vertex { position: [-0.5,  0.5,  0.5], tex_coords: [0.0, 1.0] },

    Vertex { position: [ 0.5, -0.5, -0.5], tex_coords: [0.0, 0.0] },
    Vertex { position: [-0.5, -0.5, -0.5], tex_coords: [1.0, 0.0] },
    Vertex { position: [-0.5,  0.5, -0.5], tex_coords: [1.0, 1.0] },
    Vertex { position: [ 0.5,  0.5, -0.5], tex_coords: [0.0, 1.0] },

    Vertex { position: [-0.5,  0.5, -0.5], tex_coords: [0.0, 0.0] },
    Vertex { position: [-0.5,  0.5,  0.5], tex_coords: [1.0, 0.0] },
    Vertex { position: [ 0.5,  0.5,  0.5], tex_coords: [1.0, 1.0] },
    Vertex { position: [ 0.5,  0.5, -0.5], tex_coords: [0.0, 1.0] },

    Vertex { position: [-0.5, -0.5, -0.5], tex_coords: [0.0, 0.0] },
    Vertex { position: [ 0.5, -0.5, -0.5], tex_coords: [1.0, 0.0] },
    Vertex { position: [ 0.5, -0.5,  0.5], tex_coords: [1.0, 1.0] },
    Vertex { position: [-0.5, -0.5,  0.5], tex_coords: [0.0, 1.0] },

    Vertex { position: [-0.5, -0.5, -0.5], tex_coords: [0.0, 0.0] },
    Vertex { position: [-0.5, -0.5,  0.5], tex_coords: [1.0, 0.0] },
    Vertex { position: [-0.5,  0.5,  0.5], tex_coords: [1.0, 1.0] },
    Vertex { position: [-0.5,  0.5, -0.5], tex_coords: [0.0, 1.0] },

    Vertex { position: [ 0.5, -0.5, -0.5], tex_coords: [0.0, 0.0] },
    Vertex { position: [ 0.5, -0.5,  0.5], tex_coords: [1.0, 0.0] },
    Vertex { position: [ 0.5,  0.5,  0.5], tex_coords: [1.0, 1.0] },
    Vertex { position: [ 0.5,  0.5, -0.5], tex_coords: [0.0, 1.0] },
];


// Front facing is CCW

const INDICES: &[u16] = &[
    0, 1, 2, 2, 3, 0,
    4, 5, 6, 6, 7, 4,
    8, 9, 10, 10, 11, 8,
    12, 13, 15, 15, 13, 14,
    16, 17, 18, 18, 19, 16,
    20, 22, 21, 20, 23, 22,
];

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

pub struct State {
    window: Arc<Window>,

    surface: wgpu::Surface<'static>, // Represents the window which images are presented
    device: wgpu::Device, // Open connection to graphics device to create resources
    queue: wgpu::Queue, // Submit things to run on GPU
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,

    render_pipeline: wgpu::RenderPipeline,

    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer, 
    num_indices: u32,

    diffuse_bind_group: wgpu::BindGroup,
    diffuse_texture: texture::Texture,

    camera: Camera,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_controller: CameraController,

    depth_texture: Texture,
}

impl State {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<State>{
        let window_size = window.inner_size();

        // Instance is connection to graphics backend, allows request to adapter (choose gpu) and surface
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        });

         // Represents the window which images are presented
        let surface = instance.create_surface(window.clone()).unwrap();

        // Choose GPU
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }).await?;

        // Use adapter to create device and queue (handle for gpu connection, interface for commands to send to gpu)
        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::empty(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            required_limits: wgpu::Limits::default(),
            memory_hints: Default::default(),
            trace: wgpu::Trace::Off,
        }).await?;

        // Swap chain / Surface config
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter().find(|f| f.is_srgb()).copied().unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: window_size.width,
            height: window_size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        // Load image and create GPU texture, view, and sampler from image data
        let diffuse_bytes = include_bytes!("strawberry.jpg");
        let diffuse_texture = texture::Texture::from_bytes(&device, &queue, diffuse_bytes, "strawberry.jpg").unwrap();

        let depth_texture = texture::Texture::create_depth_texture(&device, &config, "depth_texture");
        
        // Layout describing how the shader access texture + sampler
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        // Bind texture and sampler to GPU shader bindings
        let diffuse_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                    }
                ],
                label: Some("diffuse_bind_group"),
            }
        );

        
        let camera = CameraBundle::new(&ctx.device, ctx.aspect_ratio());
        let camera_controller = CameraController::new(0.2);

        // Convert CPU array -> GPU vertices buffer
        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );
        // Convert CPU array -> GPU index buffer (defines which vertices to reuse and the order they are drawn)
        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(INDICES),
                usage: wgpu::BufferUsages::INDEX,
            }
        );

        let num_indices = INDICES.len() as u32;

        // Tell GPU how to draw geometry using our shaders
        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        let render_pipeline_layout =
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[
                Some(&texture_bind_group_layout),
                Some(&camera_bind_group_layout),
            ],
            immediate_size: 0,
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[
                    Vertex::desc(),  // describes how vertex buffer data is laid out for the vertex shader
                ],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState { // How to interpret vertices when converting to triangle
                topology: wgpu::PrimitiveTopology::TriangleList, // 3 vertices = 1 triangle
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false, 
            },
            multiview_mask: None,
            cache: None,
        });
        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            render_pipeline,
            vertex_buffer,
            num_indices,
            index_buffer,
            diffuse_bind_group,
            diffuse_texture,
            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            camera_controller,
            depth_texture,
        })
    }

    // Resize surface when window size changes
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.camera.set_aspect(self.ctx.aspect_ratio());
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;
            self.depth_texture = texture::Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
        }
    }

    pub fn update(&mut self) {
        self.camera_controller.update_camera(&mut self.camera);
        self.camera_uniform.update_view_proj(&self.camera);
        self.camera.sync_to_gpu(&self.ctx.queue);
    }

    pub fn handle_key(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        if code == KeyCode::Escape && is_pressed {
            event_loop.exit();
        } else {
            self.camera_controller.handle_key(code, is_pressed);
        }
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        // Schedule another redraw
        self.window.request_redraw(); 

        if !self.is_surface_configured {
            return Ok(());
        }
            
        let output = match self.surface.get_current_texture() {
            // Normal case, return texture
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
            // Inoptimal config, recompute
            wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => {
                self.surface.configure(&self.device, &self.config);
                surface_texture
            }
            // Some issue
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => {
                // Skip this frame
                return Ok(());}
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                anyhow::bail!("Lost device");
            }
        };

        // Texture view with default settings
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create command encoder/buffer to create commands to send to GPU
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // Creates a render pass: instructions for drawing into the surface
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                // Where we draw our color to
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
            render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        // Finish encoding, send command buffer to GPU
        self.queue.submit(std::iter::once(encoder.finish()));
        // Tell swapchain ready to display!
        output.present();

        Ok(())
    }
}