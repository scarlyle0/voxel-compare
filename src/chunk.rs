use fastnoise_lite::FastNoiseLite;
use crate::mesh::Vertex;

pub const CHUNK_SIZE: usize = 16;
pub const CHUNK_HEIGHT: usize = 64;

pub struct Chunk {
    voxels: Vec<u8>,
    pub chunk_x: i32,
    pub chunk_z: i32,
}

impl Chunk {
    pub fn generate(noise: &FastNoiseLite, chunk_x: i32, chunk_z: i32) -> Self {
        let mut voxels = vec![0u8; CHUNK_SIZE * CHUNK_HEIGHT * CHUNK_SIZE];

        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let wx = (chunk_x * CHUNK_SIZE as i32 + x as i32) as f32;
                let wz = (chunk_z * CHUNK_SIZE as i32 + z as i32) as f32;

                let n = noise.get_noise_2d(wx, wz);
                let h = ((n + 1.0) * 0.5 * CHUNK_HEIGHT as f32 * 0.6
                    + CHUNK_HEIGHT as f32 * 0.15) as usize;
                let h = h.min(CHUNK_HEIGHT - 1);

                for y in 0..=h {
                    voxels[Self::idx(x, y, z)] = 1;
                }
            }
        }

        Self { voxels, chunk_x, chunk_z }
    }

    #[inline]
    fn idx(x: usize, y: usize, z: usize) -> usize {
        x * CHUNK_HEIGHT * CHUNK_SIZE + y * CHUNK_SIZE + z
    }

    #[inline]
    fn is_solid(&self, x: i32, y: i32, z: i32) -> bool {
        if x < 0
            || y < 0
            || z < 0
            || x >= CHUNK_SIZE as i32
            || y >= CHUNK_HEIGHT as i32
            || z >= CHUNK_SIZE as i32
        {
            return false;
        }
        self.voxels[Self::idx(x as usize, y as usize, z as usize)] != 0
    }

    pub fn build_mesh(&self) -> (Vec<Vertex>, Vec<u32>) {
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();

        let ox = (self.chunk_x * CHUNK_SIZE as i32) as f32;
        let oz = (self.chunk_z * CHUNK_SIZE as i32) as f32;

        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_HEIGHT {
                for z in 0..CHUNK_SIZE {
                    if self.voxels[Self::idx(x, y, z)] == 0 {
                        continue;
                    }

                    let fx = ox + x as f32;
                    let fy = y as f32;
                    let fz = oz + z as f32;
                    let xi = x as i32;
                    let yi = y as i32;
                    let zi = z as i32;

                    // +Y (top)
                    if !self.is_solid(xi, yi + 1, zi) {
                        add_face(
                            &mut vertices, &mut indices,
                            [fx, fy + 1.0, fz],
                            [fx, fy + 1.0, fz + 1.0],
                            [fx + 1.0, fy + 1.0, fz + 1.0],
                            [fx + 1.0, fy + 1.0, fz],
                            top_color(y),
                        );
                    }
                    // -Y (bottom)
                    if !self.is_solid(xi, yi - 1, zi) {
                        add_face(
                            &mut vertices, &mut indices,
                            [fx, fy, fz],
                            [fx + 1.0, fy, fz],
                            [fx + 1.0, fy, fz + 1.0],
                            [fx, fy, fz + 1.0],
                            darken(side_color(y), 0.5),
                        );
                    }
                    // +X
                    if !self.is_solid(xi + 1, yi, zi) {
                        add_face(
                            &mut vertices, &mut indices,
                            [fx + 1.0, fy, fz + 1.0],
                            [fx + 1.0, fy, fz],
                            [fx + 1.0, fy + 1.0, fz],
                            [fx + 1.0, fy + 1.0, fz + 1.0],
                            darken(side_color(y), 0.8),
                        );
                    }
                    // -X
                    if !self.is_solid(xi - 1, yi, zi) {
                        add_face(
                            &mut vertices, &mut indices,
                            [fx, fy, fz],
                            [fx, fy, fz + 1.0],
                            [fx, fy + 1.0, fz + 1.0],
                            [fx, fy + 1.0, fz],
                            darken(side_color(y), 0.8),
                        );
                    }
                    // +Z
                    if !self.is_solid(xi, yi, zi + 1) {
                        add_face(
                            &mut vertices, &mut indices,
                            [fx, fy, fz + 1.0],
                            [fx + 1.0, fy, fz + 1.0],
                            [fx + 1.0, fy + 1.0, fz + 1.0],
                            [fx, fy + 1.0, fz + 1.0],
                            darken(side_color(y), 0.7),
                        );
                    }
                    // -Z
                    if !self.is_solid(xi, yi, zi - 1) {
                        add_face(
                            &mut vertices, &mut indices,
                            [fx + 1.0, fy, fz],
                            [fx, fy, fz],
                            [fx, fy + 1.0, fz],
                            [fx + 1.0, fy + 1.0, fz],
                            darken(side_color(y), 0.7),
                        );
                    }
                }
            }
        }

        (vertices, indices)
    }
}

fn add_face(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    v0: [f32; 3],
    v1: [f32; 3],
    v2: [f32; 3],
    v3: [f32; 3],
    color: [f32; 3],
) {
    let base = vertices.len() as u32;
    vertices.extend_from_slice(&[
        Vertex { position: v0, color },
        Vertex { position: v1, color },
        Vertex { position: v2, color },
        Vertex { position: v3, color },
    ]);
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn top_color(y: usize) -> [f32; 3] {
    let h = y as f32 / CHUNK_HEIGHT as f32;
    if h > 0.70 {
        [0.30, 0.62, 0.18] // grass
    } else if h > 0.40 {
        [0.52, 0.38, 0.22] // dirt
    } else {
        [0.52, 0.52, 0.52] // stone
    }
}

fn side_color(y: usize) -> [f32; 3] {
    let h = y as f32 / CHUNK_HEIGHT as f32;
    if h > 0.65 {
        [0.52, 0.38, 0.22] // dirt under grass cap
    } else if h > 0.40 {
        [0.52, 0.38, 0.22] // dirt
    } else {
        [0.52, 0.52, 0.52] // stone
    }
}

fn darken(c: [f32; 3], f: f32) -> [f32; 3] {
    [c[0] * f, c[1] * f, c[2] * f]
}