use wgpu::util::DeviceExt;

// CPU representation of vertex
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

// Front facing is CCW
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


const INDICES: &[u16] = &[
    0, 1, 2, 2, 3, 0,
    4, 5, 6, 6, 7, 4,
    8, 9, 10, 10, 11, 8,
    12, 13, 15, 15, 13, 14,
    16, 17, 18, 18, 19, 16,
    20, 22, 21, 20, 23, 22,
];

pub struct Mesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
}

impl Mesh {
    pub fn cube(device: &wgpu::Device) -> Self {
        // Convert CPU array -> GPU vertices buffer
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

         // Convert CPU array -> GPU index buffer (defines which vertices to reuse and the order they are drawn)
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
 
        Self {
            vertex_buffer,
            index_buffer,
            num_indices: INDICES.len() as u32,
        }
    }

    pub fn from_vertices(device: &wgpu::Device, vertices: &[Vertex], indices: &[u16]) -> Self {
    }
}