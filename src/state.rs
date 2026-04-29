use std::sync::Arc;
use winit::{event_loop::ActiveEventLoop, keyboard::KeyCode, window::Window};

use crate::{
    camera::CameraBundle,
    controller::CameraController,
    gpu_context::GpuContext,
    mesh::Vertex,
    ray_march_renderer::RayMarchRenderer,
    svo::SvoBuffers,
    texture,
    world::World,
};

pub struct State {
    ctx: GpuContext,

    // Rasterisation renderer
    raster_pipeline: wgpu::RenderPipeline,
    world: World,
    depth_texture: texture::Texture,

    // SVO ray march renderer
    ray_march: RayMarchRenderer,

    // Shared
    camera: CameraBundle,
    camera_controller: CameraController,
    use_ray_march: bool,

    last_frame_time: std::time::Instant,
    frame_count: u32,
    frame_time_accum: f32,
    fps: f32,
}

impl State {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<State> {
        let ctx = GpuContext::new(window).await?;

        let camera = CameraBundle::new(&ctx.device, ctx.aspect_ratio());
        let camera_controller = CameraController::new(1.5);

        let world = World::generate(&ctx.device, 16);
        let depth_texture =
            texture::Texture::create_depth_texture(&ctx.device, &ctx.config, "depth_texture");

        // ── Rasterisation pipeline ────────────────────────────────────────────
        let raster_shader =
            ctx.device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        let raster_layout =
            ctx.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Raster Pipeline Layout"),
                bind_group_layouts: &[Some(&camera.bind_group_layout)],
                immediate_size: 0,
            });

        let raster_pipeline =
            ctx.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Raster Pipeline"),
                layout: Some(&raster_layout),
                vertex: wgpu::VertexState {
                    module: &raster_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Vertex::desc()],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &raster_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: ctx.config.format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: texture::Texture::DEPTH_FORMAT,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });

        // ── SVO ray march renderer ────────────────────────────────────────────
        let svo = SvoBuffers::build_and_upload(&ctx.device);
        let ray_march = RayMarchRenderer::new(&ctx.device, ctx.config.format, &camera, &svo);

        Ok(Self {
            ctx,
            raster_pipeline,
            world,
            depth_texture,
            ray_march,
            camera,
            camera_controller,
            use_ray_march: false,
            last_frame_time: std::time::Instant::now(),
            frame_count: 0,
            frame_time_accum: 0.0,
            fps: 0.0,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.ctx.resize(width, height);
        self.camera.set_aspect(self.ctx.aspect_ratio());
        self.depth_texture =
            texture::Texture::create_depth_texture(&self.ctx.device, &self.ctx.config, "depth_texture");
    }

    pub fn update(&mut self) {
        self.camera_controller.update_camera(&mut self.camera.camera);
        self.camera.sync_to_gpu(&self.ctx.queue);

        let now = std::time::Instant::now();
        let dt = (now - self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;
        self.frame_time_accum += dt;
        self.frame_count += 1;

        if self.frame_time_accum >= 0.5 {
            self.fps = self.frame_count as f32 / self.frame_time_accum;
            self.frame_count = 0;
            self.frame_time_accum = 0.0;

            let mode = if self.use_ray_march { "SVO Ray March" } else { "Rasterisation" };
            self.ctx.window.set_title(&format!(
                "Voxel Demo | {mode} | {:.0} FPS  [Tab] to switch",
                self.fps
            ));
        }
    }

    pub fn handle_key(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        match code {
            KeyCode::Escape if is_pressed => event_loop.exit(),
            KeyCode::Tab if is_pressed => {
                self.use_ray_march = !self.use_ray_march;
                let mode = if self.use_ray_march { "SVO Ray March" } else { "Rasterisation" };
                self.ctx.window.set_title(&format!(
                    "Voxel Demo | {mode} | {:.0} FPS  [Tab] to switch",
                    self.fps
                ));
            }
            _ => { self.camera_controller.handle_key(code, is_pressed); }
        }
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        self.ctx.window.request_redraw();

        if !self.ctx.is_surface_configured {
            return Ok(());
        }

        let output = match self.ctx.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) => t,
            wgpu::CurrentSurfaceTexture::Suboptimal(t) => {
                self.ctx.surface.configure(&self.ctx.device, &self.ctx.config);
                t
            }
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => return Ok(()),
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.ctx.surface.configure(&self.ctx.device, &self.ctx.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Lost => anyhow::bail!("Lost device"),
        };

        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        if self.use_ray_march {
            self.render_ray_march(&mut encoder, &view);
        } else {
            self.render_raster(&mut encoder, &view);
        }

        self.ctx.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }

    // ── Rasterisation pass ────────────────────────────────────────────────────
    fn render_raster(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Raster Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.53, g: 0.81, b: 0.98, a: 1.0 }),
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

        pass.set_pipeline(&self.raster_pipeline);
        pass.set_bind_group(0, &self.camera.bind_group, &[]);

        for chunk in &self.world.chunks {
            pass.set_vertex_buffer(0, chunk.vertex_buffer.slice(..));
            pass.set_index_buffer(chunk.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..chunk.num_indices, 0, 0..1);
        }
    }

    // ── Ray march pass ────────────────────────────────────────────────────────
    fn render_ray_march(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Ray March Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.53, g: 0.81, b: 0.98, a: 1.0 }),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });

        pass.set_pipeline(&self.ray_march.pipeline);
        pass.set_bind_group(0, &self.camera.bind_group, &[]);
        pass.set_bind_group(1, &self.ray_march.svo_bind_group, &[]);
        pass.draw(0..3, 0..1); // fullscreen triangle
    }
}
