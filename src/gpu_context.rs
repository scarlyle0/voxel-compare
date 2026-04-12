use std::sync::Arc;
use winit::window::Window;
 
// Owns the core wgpu infrastructure: surface, device, queue, and swap chain config.
pub struct GpuContext {
    pub window: Arc<Window>,
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub is_surface_configured: bool,
}