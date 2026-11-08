// WGSL 2D Debug Overlay Shader

@group(0) @binding(0)
var t_font: texture_2d<f32>;
@group(0) @binding(1)
var s_font: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position, 0.0, 1.0);
    out.uv = in.uv;
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let font_sample = textureSample(t_font, s_font, in.uv);
    // Use alpha channel or luminance from font bitmap
    let alpha = font_sample.a * in.color.a;
    if (alpha < 0.05) {
        discard;
    }
    return vec4<f32>(in.color.rgb, alpha);
}

