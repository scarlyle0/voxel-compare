# Voxel Demo

A real-time voxel terrain renderer written in Rust, built as a hands-on exploration of two fundamentally different rendering techniques side by side. Press **Tab** at any time to switch between them.

---

## Renderers

### Rasterisation

The traditional approach. At startup the world is divided into 33×33 chunks (radius 16). Each chunk samples a Perlin noise heightmap to determine which voxels are solid, then builds a mesh — only faces that border empty air are emitted (face culling). The resulting vertex and index buffers are uploaded to the GPU once and drawn every frame via a standard vertex/fragment pipeline.

Lighting is baked into the vertex colours at mesh-build time: top faces are full brightness, side faces are darkened by a fixed multiplier, and bottom faces darker still.

### SVO Ray March

A completely different philosophy. Instead of sending geometry to the camera, rays are fired from the camera into the scene — one per pixel — and the first solid voxel each ray hits is what gets drawn.

The scene is stored as a **Sparse Voxel Octree (SVO)**: a 512³ cubic region recursively subdivided into octants. Empty subtrees are collapsed to a single null value, so the structure is compact and large empty regions can be skipped in a single step. At startup the octree is built on the CPU (~280 K nodes) and uploaded to the GPU as a flat storage buffer.

Each frame a single fullscreen triangle is drawn. The fragment shader fires one ray per pixel, descends the SVO to find the first solid voxel, and shades it using the face normal it gets for free from the ray–AABB intersection. No mesh, no vertex buffer, no draw calls per chunk.

---

## Controls

| Key | Action |
|---|---|
| **W / A / S / D** | Fly forward / left / back / right |
| **E / Space** | Fly up |
| **Q / Shift** | Fly down |
| **Tab** | Switch between Rasterisation and SVO Ray March |
| **Escape** | Quit |

The window title always shows the active renderer and current FPS.

---

## Building and Running

Requires a recent stable Rust toolchain and a GPU with Vulkan, Metal, or DirectX 12 support.

```bash
git clone https://github.com/yourname/voxel-demo
cd voxel-demo
cargo run --release
```


---

## Project Structure

```
src/
├── main.rs                 Entry point; module declarations
├── app.rs                  winit event loop (ApplicationHandler)
├── state.rs                Per-frame update + render dispatch; Tab toggle
│
├── gpu_context.rs          wgpu device / surface / queue setup
├── camera.rs               Camera matrices, GPU uniform, bind group
├── controller.rs           WASD keyboard input → camera movement
│
├── world.rs                Generates ChunkMesh list for the rasteriser
├── chunk.rs                Per-chunk voxel data + face-culled mesh builder
├── mesh.rs                 Vertex struct (position + colour)
│
├── svo.rs                  Builds the Sparse Voxel Octree; uploads to GPU
├── ray_march_renderer.rs   wgpu pipeline for the ray march pass
│
├── texture.rs              Depth texture helper
├── shader.wgsl             Rasterisation vertex + fragment shaders
└── ray_march.wgsl          SVO ray march fragment shader
```

---

## How the SVO Ray Marcher Works

### 1 — The Octree

The 512³ world is recursively split into 8 child octants, down to individual 1×1×1 voxels. Each node stores 8 child descriptors as `u32` values:

| Value | Meaning |
|---|---|
| `0` | Empty subtree |
| High bit set | Solid leaf — lower 24 bits are packed RGB |
| Any other value | Index into the flat node buffer |

Subtrees where every voxel is empty are pruned entirely, so the mountain-and-sky scene compresses to ~280 K nodes instead of 134 M.

### 2 — Firing Rays

A fullscreen triangle covers every pixel. In the fragment shader, the pixel's NDC position is unprojected through `inv_view_proj` to get a world-space ray direction. No geometry is needed.

### 3 — Traversal

The marcher steps along the ray. At each position it descends the octree from the root — at each level picking the octant that contains the current point — until it finds either an empty node or a solid leaf.

- **Empty node:** jump the ray to the far side of that node's bounding box in a single step. A 256-unit empty region costs one octree descent and one ray advance.
- **Solid leaf:** shade and return.