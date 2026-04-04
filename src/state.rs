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
        let window_size = window.inner_size();

        // Instance is connection to graphics backend, allows request to adapter (choose gpu) and surface
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY
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

        Ok(Self {
            window
        })
    }
}