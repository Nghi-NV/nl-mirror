// Vertex shader for fullscreen quad with aspect ratio correction
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

// Uniforms for aspect ratio correction
struct AspectRatio {
    scale: vec2<f32>,
};

@group(0) @binding(3)
var<uniform> aspect: AspectRatio;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    
    // Generate fullscreen quad from vertex index (0-5 = 2 triangles)
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),  // bottom-left
        vec2<f32>(1.0, -1.0),   // bottom-right
        vec2<f32>(-1.0, 1.0),   // top-left
        vec2<f32>(-1.0, 1.0),   // top-left
        vec2<f32>(1.0, -1.0),   // bottom-right
        vec2<f32>(1.0, 1.0),    // top-right
    );
    
    // Texture coordinates (flip Y for correct orientation)
    var tex_coords = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0),
    );
    
    // Apply aspect ratio correction to positions
    let pos = positions[vertex_index] * aspect.scale;
    out.clip_position = vec4<f32>(pos, 0.0, 1.0);
    out.tex_coords = tex_coords[vertex_index];
    
    return out;
}

// Fragment shader with YUV to RGB conversion on GPU
@group(0) @binding(0)
var y_texture: texture_2d<f32>;
@group(0) @binding(1)
var u_texture: texture_2d<f32>;
@group(0) @binding(2)
var v_texture: texture_2d<f32>;
@group(0) @binding(4)
var tex_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample Y, U, V planes (values are 0-1 range from R8Unorm texture)
    // OpenH264 outputs FULL RANGE YUV (0-255 for all components)
    let y = textureSample(y_texture, tex_sampler, in.tex_coords).r;
    let u = textureSample(u_texture, tex_sampler, in.tex_coords).r - 0.5;  // Center U around 0
    let v = textureSample(v_texture, tex_sampler, in.tex_coords).r - 0.5;  // Center V around 0
    
    // BT.601 Full Range YUV to RGB conversion
    // Standard coefficients for full range input
    // R = Y + 1.402 * V
    // G = Y - 0.344136 * U - 0.714136 * V
    // B = Y + 1.772 * U
    let r = y + 1.402 * v;
    let g = y - 0.344136 * u - 0.714136 * v;
    let b = y + 1.772 * u;
    
    return vec4<f32>(clamp(r, 0.0, 1.0), clamp(g, 0.0, 1.0), clamp(b, 0.0, 1.0), 1.0);
}
