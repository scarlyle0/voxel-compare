use std::sync::Arc;
use winit::{event_loop::ActiveEventLoop, keyboard::KeyCode, window::Window};
 
use crate::{
    camera::CameraBundle,
    controller::CameraController,
    gpu_context::GpuContext,
    mesh::{Mesh, Vertex},
    texture,
};

pub struct State {
    ctx: GpuContext,
 
    render_pipeline: wgpu::RenderPipeline,
    mesh: Mesh,

    diffuse: texture::TextureBundle,
    depth_texture: texture::Texture,
 
    camera: CameraBundle,
    camera_controller: CameraController,
}

impl State {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<State>{
        let ctx = GpuContext::new(window).await?;

        
        let camera = CameraBundle::new(&ctx.device, ctx.aspect_ratio());
        let camera_controller = CameraController::new(0.2);

        let mesh = Mesh::cube(&ctx.device);

        let diffuse = texture::TextureBundle::from_image_bytes(
            &ctx.device,
            &ctx.queue,
            include_bytes!("strawberry.jpg"),
            "strawberry.jpg",
        );

        let depth_texture = texture::Texture::create_depth_texture(&ctx.device, &ctx.config, "depth_texture");

        // Tell GPU how to draw geometry using our shaders
        let shader = ctx.device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        let render_pipeline_layout =
            ctx.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    Some(&diffuse.bind_group_layout),
                    Some(&camera.bind_group_layout),
                ],
                immediate_size: 0,
            });

        let render_pipeline = ctx.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                    format: ctx.config.format,
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
            ctx,
            render_pipeline,
            mesh,
            diffuse,
            camera,
            camera_controller,
            depth_texture,
        })
    }

    // Resize surface when window size changes
    pub fn resize(&mut self, width: u32, height: u32) {
        self.ctx.resize(width, height);
        self.camera.set_aspect(self.ctx.aspect_ratio());
        self.depth_texture = texture::Texture::create_depth_texture(
            &self.ctx.device,
            &self.ctx.config,
            "depth_texture",
        );
    }

    pub fn update(&mut self) {
        self.camera_controller.update_camera(&mut self.camera.camera);
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
        self.ctx.window.request_redraw();

        if !self.ctx.is_surface_configured {
            return Ok(());
        }
            
        let output = match self.ctx.surface.get_current_texture() {
            // Normal case, return texture
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
            // Inoptimal config, recompute
            wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => {
                self.ctx.surface.configure(&self.ctx.device, &self.ctx.config);
                surface_texture
            }
            // Some issue
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => {
                // Skip this frame
                return Ok(());}
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.ctx.surface.configure(&self.ctx.device, &self.ctx.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                anyhow::bail!("Lost device");
            }
        };

        // Texture view with default settings
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create command encoder/buffer to create commands to send to GPU
        let mut encoder = self.ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
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
            render_pass.set_bind_group(0, &self.diffuse.bind_group, &[]);
            render_pass.set_bind_group(1, &self.camera.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.mesh.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.mesh.num_indices, 0, 0..1);
        }

        // Finish encoding, send command buffer to GPU
        self.ctx.queue.submit(std::iter::once(encoder.finish()));
        // Tell swapchain ready to display!
        output.present();

        Ok(())
    }
}