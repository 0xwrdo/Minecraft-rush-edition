// WGSL Chunk Shader with Directional Shading, Minecraft 1.0 Light Curves, and Distance Fog

struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec3<f32>,
    @location(3) light: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) shade: vec3<f32>,
    @location(2) fog_factor: f32,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(in.position, 1.0);
    out.uv = in.uv;
    
    // Combine face directional multiplier with block/sky light brightness
    out.shade = in.color * in.light;

    // Linear distance fog (Minecraft 1.0 style)
    let dist = out.clip_position.z;
    let fog_start = 70.0;
    let fog_end = 120.0;
    out.fog_factor = clamp((dist - fog_start) / (fog_end - fog_start), 0.0, 1.0);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(t_diffuse, s_diffuse, in.uv);
    
    // Alpha cutout for transparent foliage/leaves
    if (tex_color.a < 0.1) {
        discard;
    }

    let shaded = tex_color.rgb * in.shade;
    let fog_color = vec3<f32>(0.68, 0.81, 0.98); // Minecraft Overworld horizon sky blue
    let final_rgb = mix(shaded, fog_color, in.fog_factor);

    return vec4<f32>(final_rgb, tex_color.a);
}

