#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: glam::Mat4 = glam::Mat4::from_cols(
    glam::Vec4::new(1.0, 0.0, 0.0, 0.0),
    glam::Vec4::new(0.0, 1.0, 0.0, 0.0),
    glam::Vec4::new(0.0, 0.0, 0.5, 0.0),
    glam::Vec4::new(0.0, 0.0, 0.5, 1.0),
);

pub struct Camera {
    pub eye: glam::Vec3,
    pub target: glam::Vec3,
    pub up: glam::Vec3,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera {
    fn build_view_projection_matrix(&self) -> glam::Mat4 {
        // Moves world to be at position and rotation of camera
        let view = glam::Mat4::look_at_rh(self.eye, self.target, self.up);
        // Warps scene to give the effect of depth
        let proj = glam::Mat4::perspective_rh(self.fovy.to_radians(), self.aspect, self.znear, self.zfar);

        return OPENGL_TO_WGPU_MATRIX * proj * view;
    }
}

// GPU representation of the camera, must match the uniform layout in shader.wgsl
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_proj:     [[f32; 4]; 4],  // used by rasteriser vertex shader
    inv_view_proj: [[f32; 4]; 4],  // used by ray march fragment shader
    position:      [f32; 3],       // camera world position for ray origin
    _pad:          f32,
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_proj:     glam::Mat4::IDENTITY.to_cols_array_2d(),
            inv_view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
            position:      [0.0; 3],
            _pad:          0.0,
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        let vp = camera.build_view_projection_matrix();
        self.view_proj     = vp.to_cols_array_2d();
        self.inv_view_proj = vp.inverse().to_cols_array_2d();
        self.position      = camera.eye.to_array();
    }
}

// Everything needed to drive the camera from the GPU side:
// the logical camera, its uniform mirror, and the GPU buffer + bind group.
pub struct CameraBundle {
    pub camera: Camera,
    pub uniform: CameraUniform,
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl CameraBundle {
    pub fn new(device: &wgpu::Device, aspect: f32) -> Self {
        use wgpu::util::DeviceExt;

        let camera = Camera {
            eye: glam::Vec3::new(16.0, 90.0, 90.0),
            target: glam::Vec3::new(0.0, 25.0, 0.0),
            up: glam::Vec3::Y,
            aspect,
            fovy: 60.0,
            znear: 0.1,
            zfar: 2000.0,
        };

        let mut uniform = CameraUniform::new();
        uniform.update_view_proj(&camera);

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("camera_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera_bind_group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        Self {
            camera,
            uniform,
            buffer,
            bind_group,
            bind_group_layout,
        }
    }

    // Call once per frame after the controller has moved the camera.
    pub fn sync_to_gpu(&mut self, queue: &wgpu::Queue) {
        self.uniform.update_view_proj(&self.camera);
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform]));
    }

    pub fn set_aspect(&mut self, aspect: f32) {
        self.camera.aspect = aspect;
    }
}
