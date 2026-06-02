# Voxel Compare

A real-time voxel terrain renderer written in Rust, built as a hands-on exploration of two fundamentally different rendering techniques side by side. Press **Tab** at any time to switch between them.

---

## Renderers

### Rasterisation

The traditional approach. At startup the world is divided into 33×33 chunks (radius 16). Each chunk samples a Perlin noise heightmap to determine which voxels are solid, then builds a mesh. Only faces that border empty air are emitted (face culling). The resulting vertex and index buffers are uploaded to the GPU once and drawn every frame via a standard vertex/fragment pipeline.

Lighting is baked into the vertex colours at mesh-build time: top faces are full brightness, side faces are darkened by a fixed multiplier, and bottom faces darker still.

### SVO Ray March

Instead of sending geometry to the camera, rays are fired from the camera into the scene one per pixel, and the first solid voxel each ray hits is what gets drawn.

The scene is stored as a **Sparse Voxel Octree (SVO)**: a 512³ cubic region recursively subdivided into octants. Empty subtrees are collapsed to a single null value, so the structure is compact and large empty regions can be skipped in a single step. At startup the octree is built on the CPU and uploaded to the GPU as a flat storage buffer.

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

```bash
git clone https://github.com/scarlyle0/voxel-compare
cd voxel-demo
cargo run
```

---

## Project Structure

```
src/
├── main.rs              Entry point
├── app.rs               winit event loop
├── state.rs             Per-frame update + render dispatch; Tab toggle
│
├── render/
│   ├── gpu_context.rs   wgpu device / surface / queue setup
│   └── texture.rs       Depth texture helper
│
├── input/
│   ├── camera.rs        Camera matrices, GPU uniform, bind group
│   └── controller.rs    WASD keyboard input → camera movement
│
├── chunk/
│   ├── chunk.rs         Per-chunk voxel data + face-culled mesh builder
│   ├── terrain.rs       Generates ChunkMesh list for the rasteriser
│   ├── vertex.rs        Vertex struct (position + colour)
│   └── raster.wgsl      Rasterisation vertex + fragment shaders
│
└── svo/
    ├── svo.rs           Builds the Sparse Voxel Octree; uploads to GPU
    ├── svo_pipeline.rs  wgpu pipeline for the ray march pass
    └── ray_march.wgsl   SVO ray march fragment shader
```
