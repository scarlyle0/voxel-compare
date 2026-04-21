use fastnoise_lite::FastNoiseLite;
use crate::mesh::Vertex;

pub const CHUNK_SIZE: usize = 16;
pub const CHUNK_HEIGHT: usize = 64;

pub struct Chunk {
    voxels: Vec<u8>,
    pub chunk_x: i32,
    pub chunk_z: i32,
}
