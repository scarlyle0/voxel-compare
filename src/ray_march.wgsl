// Camera (group 0, binding 0)
struct CameraUniform {
    view_proj: mat4x4<f32>, // rasteriser (not used here)
    inv_view_proj: mat4x4<f32>, // clip→world transform for ray casting
    position: vec3<f32>,
    _pad: f32,
}
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

// SVO (group 1)
struct SvoInfo {
    root: u32, // index of the root node in svo_nodes
    size: u32, // side length of the cubic world in voxels
    _pad0: u32,
    _pad1: u32,
    origin: vec3<f32>, // world-space corner of the SVO box (e.g. -256, 0, -256)
    _pad2: f32,
}
@group(1) @binding(0)
var<uniform> svo_info: SvoInfo;

struct SvoNode {
    children: array<u32, 8>,
}
@group(1) @binding(1)
var<storage, read> svo_nodes: array<SvoNode>;

// Vertex stage: full-screen triangle 
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) ndc: vec2<f32>, // clip-space XY, passed through
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    // Three vertices that cover the entire NDC square [-1,1]×[-1,1].
    var corners = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    let p = corners[vi];
    var out: VertexOutput;
    out.position = vec4<f32>(p, 0.0, 1.0);
    out.ndc = p;
    return out;
}

// Ray–AABB slab test
// Returns (t_near, t_far).  Miss when t_near > t_far or t_far < 0.
fn ray_aabb(
    ro: vec3<f32>, inv_rd: vec3<f32>,
    box_min: vec3<f32>, box_max: vec3<f32>,
) -> vec2<f32> {
    let t0 = (box_min - ro) * inv_rd;
    let t1 = (box_max - ro) * inv_rd;
    let t_near = max(max(min(t0.x, t1.x), min(t0.y, t1.y)), min(t0.z, t1.z));
    let t_far  = min(min(max(t0.x, t1.x), max(t0.y, t1.y)), max(t0.z, t1.z));
    return vec2<f32>(t_near, t_far);
}

// SVO ray marcher
const MAX_STEPS: i32 = 200;
const EPS: f32 = 0.0004;
const SKY: vec3<f32> = vec3<f32>(0.53, 0.81, 0.98);

fn trace(ro: vec3<f32>, rd: vec3<f32>) -> vec4<f32> {
    let inv_rd = 1.0 / rd;
    let world_sz = f32(svo_info.size);
    let world_min = svo_info.origin;
    let world_max = svo_info.origin + world_sz;

    // Early-out: does the ray hit the world AABB at all?
    let wh = ray_aabb(ro, inv_rd, world_min, world_max);
    if wh.x > wh.y || wh.y < 0.0 {
        return vec4<f32>(SKY, 1.0);
    }

    var t = max(wh.x, 0.0) + EPS;
    let t_max = wh.y;

    for (var step = 0; step < MAX_STEPS; step++) {
        if t >= t_max { break; }

        let pos = ro + t * rd;

        // Descend the SVO to find the subtree containing `pos`
        var node_val = svo_info.root;
        var node_min = svo_info.origin;
        var node_size = world_sz;

        // Maximum depth = log2(SVO_SIZE) = 9 halvings → 10 iterations.
        for (var depth = 0; depth < 10; depth++) {

            if node_val == 0u {
                // Empty region
                break;
            }

            if (node_val & 0x80000000u) != 0u {
                // Solid leaf hit
                let packed = node_val & 0x00FFFFFFu;
                let r = f32((packed >> 16u) & 0xFFu) / 255.0;
                let g = f32((packed >>  8u) & 0xFFu) / 255.0;
                let b = f32( packed         & 0xFFu) / 255.0;
                let base = vec3<f32>(r, g, b);

                // Surface normal from the entry face of this voxel.
                let t0v = (node_min              - ro) * inv_rd;
                let t1v = (node_min + node_size  - ro) * inv_rd;
                let tn  = min(t0v, t1v);
                var normal = vec3<f32>(0.0, 1.0, 0.0);
                if tn.x >= tn.y && tn.x >= tn.z {
                    normal = vec3<f32>(-sign(rd.x), 0.0, 0.0);
                } else if tn.y >= tn.z {
                    normal = vec3<f32>(0.0, -sign(rd.y), 0.0);
                } else {
                    normal = vec3<f32>(0.0, 0.0, -sign(rd.z));
                }

                let sun   = normalize(vec3<f32>(0.6, 1.0, 0.4));
                let light = 0.20 + 0.80 * max(dot(normal, sun), 0.0);
                return vec4<f32>(base * light, 1.0);
            }

            // Internal node: step into the child that contains `pos`
            let half = node_size * 0.5;
            let mid  = node_min + half;
            let ox   = select(0u, 1u, pos.x >= mid.x);
            let oy   = select(0u, 1u, pos.y >= mid.y);
            let oz   = select(0u, 1u, pos.z >= mid.z);
            let oct  = ox | (oy << 1u) | (oz << 2u);

            node_min  = node_min + vec3<f32>(f32(ox), f32(oy), f32(oz)) * half;
            node_size = half;
            node_val  = svo_nodes[node_val].children[oct];
        }

        // `node_min`/`node_size` now describe an empty (or out-of-bounds) box.
        // Advance the ray to its far side so the next step enters new space.
        let exit = ray_aabb(ro, inv_rd, node_min, node_min + node_size);
        t = exit.y + EPS;
    }

    return vec4<f32>(SKY, 1.0);
}

// Fragment stage
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Unproject from clip space to world space to get the ray direction.
    let clip    = vec4<f32>(in.ndc.x, in.ndc.y, 1.0, 1.0);
    let world_h = camera.inv_view_proj * clip;
    let rd      = normalize(world_h.xyz / world_h.w - camera.position);

    return trace(camera.position, rd);
}
