use winit::window::Window;
use std::sync::Arc;

pub struct State {
    window: Arc<Window>,

    surface: wgpu::Surface<'static>, // Represents the window which images are presented
    device: wgpu::Device, // Open connection to graphics device to create resources
    queue: wgpu::Queue, // Submit things to run on GPU
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
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

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
        })
    }

    // Resize surface when window size changes
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;
        }
    }

    pub fn update(&mut self) {
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
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                // Where we draw our color to
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
        }

        // Finish encoding, send command buffer to GPU
        self.queue.submit(std::iter::once(encoder.finish()));
        // Tell swapchain ready to display!
        output.present();

        Ok(())
    }
}