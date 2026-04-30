use fastnoise_lite::{FastNoiseLite, FractalType, NoiseType};
use wgpu::util::DeviceExt;

// Side length of the cubic SVO world in voxels (must be a power of two).
pub const SVO_SIZE: u32 = 512;

/// A node in the SVO buffer.
///
/// Each `children[i]` encodes one child octant:
///   0               – empty subtree
///   high bit set    – solid leaf; lower 24 bits = packed RGB8 colour
///   otherwise       – index into the `svo_nodes` storage buffer
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SvoNode {
    pub children: [u32; 8],
}

pub struct SvoBuffers {
    // Uniform: [root_index, grid_size, pad, pad, origin_x, origin_y, origin_z, pad]
    pub info_buffer: wgpu::Buffer,
    // Storage buffer of `SvoNode`
    pub nodes_buffer: wgpu::Buffer,
}

impl SvoBuffers {
    pub fn build_and_upload(device: &wgpu::Device) -> Self {
        println!("Building SVO ({0}^3 grid)...", SVO_SIZE);

        // Voxel grid
        let mut noise = FastNoiseLite::new();
        noise.set_noise_type(Some(NoiseType::Perlin));
        noise.set_fractal_type(Some(FractalType::FBm));
        noise.set_fractal_octaves(Some(4));
        noise.set_fractal_lacunarity(Some(2.0));
        noise.set_fractal_gain(Some(0.5));
        noise.set_frequency(Some(0.02));

        let n = SVO_SIZE as usize;
        let mut grid = vec![0u8; n * n * n];

        // Sample noise at centred world coords so the SVO terrain matches
        let half = n as f32 * 0.5;
        for x in 0..n {
            for z in 0..n {
                let wx = x as f32 - half;
                let wz = z as f32 - half;
                let noise_val = noise.get_noise_2d(wx, wz);
                let h = ((noise_val + 1.0) * 0.5 * 64.0 * 0.6 + 64.0 * 0.15) as usize;
                let h = h.min(63);
                for y in 0..=h {
                    grid[x * n * n + y * n + z] = 1;
                }
            }
        }

        // Build SVO
        // Slot 0 is reserved so that child value 0 unambiguously means "empty".
        let mut nodes: Vec<SvoNode> = vec![SvoNode { children: [0; 8] }];
        let root = build_node(&grid, n, 0, 0, 0, SVO_SIZE, &mut nodes);

        // GPU upload
        // Layout (32 bytes, matches SvoInfo in ray_march.wgsl):
        // u32 root, u32 size, u32 pad, u32 pad,
        // f32 origin_x, f32 origin_y, f32 origin_z, f32 pad
        let half_f = SVO_SIZE as f32 * 0.5;
        let info_bytes: [u8; 32] = bytemuck::cast([
            root,
            SVO_SIZE,
            0u32,
            0u32,
            (-half_f).to_bits(),
            0f32.to_bits(),
            (-half_f).to_bits(),
            0f32.to_bits(),
        ]);

        let info_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("SVO Info Buffer"),
            contents: &info_bytes,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let nodes_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("SVO Nodes Buffer"),
            contents: bytemuck::cast_slice(&nodes),
            usage: wgpu::BufferUsages::STORAGE,
        });

        SvoBuffers { info_buffer, nodes_buffer }
    }
}

// helper
fn voxel_color_packed(y: u32) -> u32 {
    let h = y as f32 / 64.0;
    let (r, g, b): (u32, u32, u32) = if h > 0.70 {
        (0x4D, 0x9E, 0x2E) // grass
    } else if h > 0.40 {
        (0x85, 0x61, 0x38) // dirt
    } else {
        (0x85, 0x85, 0x85) // stone
    };
    (r << 16) | (g << 8) | b
}

// Recursively constructs the SVO bottom-up.  Returns the child descriptor
// for this subtree (0 = empty, high-bit = leaf, else = node index).
fn build_node(
    grid: &[u8],
    stride: usize,
    x: u32,
    y: u32,
    z: u32,
    size: u32,
    nodes: &mut Vec<SvoNode>,
) -> u32 {
    let n = stride as u32;

    // Out-of-bounds octants are empty.
    if x >= n || y >= n || z >= n {
        return 0;
    }

    // At a leaf
    if size == 1 {
        let v = grid[x as usize * stride * stride + y as usize * stride + z as usize];
        return if v == 0 {
            0
        } else {
            0x8000_0000 | voxel_color_packed(y)
        };
    }

    let half = size / 2;
    let mut children = [0u32; 8];
    let mut any_solid = false;

    // Recurse into all 8 children
    for oct in 0u32..8 {
        let cx = x + (oct & 1) * half;
        let cy = y + ((oct >> 1) & 1) * half;
        let cz = z + ((oct >> 2) & 1) * half;
        let child = build_node(grid, stride, cx, cy, cz, half, nodes);
        children[oct as usize] = child;
        if child != 0 {
            any_solid = true;
        }
    }

    if !any_solid {
        return 0;
    }

    let idx = nodes.len() as u32;
    nodes.push(SvoNode { children });
    idx
}
