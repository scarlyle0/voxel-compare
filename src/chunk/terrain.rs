use fastnoise_lite::{FastNoiseLite, FractalType, NoiseType};
use wgpu::util::DeviceExt;

use crate::chunk::chunk::Chunk;

pub struct ChunkMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
}

pub struct Terrain {
    pub chunks: Vec<ChunkMesh>,
}

impl Terrain {
    pub fn generate(device: &wgpu::Device, radius: i32) -> Self {
        let mut noise = FastNoiseLite::new();
        noise.set_noise_type(Some(NoiseType::Perlin));
        noise.set_fractal_type(Some(FractalType::FBm));
        noise.set_fractal_octaves(Some(4));
        noise.set_fractal_lacunarity(Some(2.0));
        noise.set_fractal_gain(Some(0.5));
        noise.set_frequency(Some(0.02));

        let mut chunks = Vec::new();

        for cx in -radius..=radius {
            for cz in -radius..=radius {
                let chunk = Chunk::generate(&noise, cx, cz);
                let (vertices, indices) = chunk.build_mesh();

                if indices.is_empty() {
                    continue;
                }

                let vertex_buffer =
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Chunk Vertex Buffer"),
                        contents: bytemuck::cast_slice(&vertices),
                        usage: wgpu::BufferUsages::VERTEX,
                    });

                let index_buffer =
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Chunk Index Buffer"),
                        contents: bytemuck::cast_slice(&indices),
                        usage: wgpu::BufferUsages::INDEX,
                    });

                chunks.push(ChunkMesh {
                    vertex_buffer,
                    index_buffer,
                    num_indices: indices.len() as u32,
                });
            }
        }

        Terrain { chunks }
    }
}
