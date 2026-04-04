use winit::window::Window;
use std::sync::Arc;

pub struct State {
    window: Arc<Window>

    surface: wgpu::Surface<'static>, // Represents the window which images are presented
    device: wgpu::Device, // Open connection to graphics device to create resources
    queue: wgpu::Queue, // Submit things to run on GPU
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
}

impl State {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<State>{
        Ok(Self {
            window
        })
    }
}