struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let x = i32(in.vertex_index) / 2;
    let y = i32(in.vertex_index) & 1;

    let uv = vec2<f32>(f32(x) * 2.0, f32(y) * 2.0);

    out.clip_position = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
    out.uv = uv;

    return out;
}

@group(0) @binding(0)
var t_texture: texture_2d<f32>;
@group(0) @binding(1)
var t_sampler: sampler;
@group(0) @binding(2)
var t_lut: texture_2d<f32>;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = textureSample(t_texture, t_sampler, in.uv).rgb;

    color = tonemap(color);
    // The LUT is already gamma corrected.
    color = fo4_lut(color);

    // Screen space dithering should happen after converting to gamma correction.
    // The bound output texture is already linear, so the GPU will not do a convertion
    // to sRGB.
    color += screen_space_dither(in.clip_position.xy);

    return vec4(color, 1.0);
}

fn tonemap(color: vec3<f32>) -> vec3<f32> {
    // TODO: Extract this value from the input texture.
    let luminance = 0.125;
    // TODO: Allow the camera to provide these values.
    let gray_middle = 0.18;
    let min_exposure = 1.6;
    let max_expsure = 4.5;
    let exposure = clamp(gray_middle / luminance, min_exposure, max_expsure);
    return uncharted2_filmic(color * exposure);
}

// https://64.github.io/tonemapping/#uncharted-2
fn uncharted2_filmic(v: vec3<f32>) -> vec3<f32> {
    let exposure_bias = 2.0;
    let curr = uncharted2_tonemap_partial(v * exposure_bias);

    let W = vec3<f32>(11.2);
    let white_scale = vec3<f32>(1.0) / uncharted2_tonemap_partial(W);
    return curr * white_scale;
}

fn uncharted2_tonemap_partial(x: vec3<f32>) -> vec3<f32> {
    let A = 0.15f;
    let B = 0.50f;
    let C = 0.10f;
    let D = 0.20f;
    let E = 0.02f;
    let F = 0.30f;
    return ((x*(A*x+C*B)+D*E)/(x*(A*x+B)+D*F))-E/F;
}

fn gamma_correct(color: vec3<f32>) -> vec3<f32> {
    var out_color = vec3(0.0);
    out_color.r = linear_to_srgb(color.r);
    out_color.g = linear_to_srgb(color.g);
    out_color.b = linear_to_srgb(color.b);
    return out_color;
}

// sRGB transfer functions
// https://en.wikipedia.org/wiki/SRGB#Transformation

fn linear_to_srgb(color: f32) -> f32 {
    if color <= 0.0031308 {
        return color * 12.92;
    } else {
        return (pow(color, 1.0 / 2.4) * 1.055) - 0.055;
    }
}

fn srgb_to_linear(color: f32) -> f32 {
    if color <= 0.04045 {
        return color / 12.92;
    } else {
        return pow((color + 0.055) / 1.055, 2.4);
    }
}

// Screen space dithering
// https://github.com/bevyengine/bevy/blob/main/crates/bevy_core_pipeline/src/tonemapping/tonemapping_shared.wgsl
// https://media.steampowered.com/apps/valve/2015/Alex_Vlachos_Advanced_VR_Rendering_GDC2015.pdf
fn screen_space_dither(uv: vec2<f32>) -> vec3<f32> {
    var dither = vec3<f32>(dot(vec2<f32>(171.0, 231.0), uv)).xxx;
    dither = fract(dither.rgb / vec3<f32>(103.0, 71.0, 97.0));
    return (dither - 0.5) / 255.0;
}

// https://github.com/TheRealMJP/BakingLab/blob/master/BakingLab/ACES.hlsl

const ACES_INPUT_MATRIX: mat3x3<f32> = mat3x3<f32>(
    vec3<f32>(0.59719, 0.35458, 0.04823),
    vec3<f32>(0.07600, 0.90834, 0.01566),
    vec3<f32>(0.02840, 0.13383, 0.83777)
);

const ACES_OUTPUT_MATRIX: mat3x3<f32> = mat3x3<f32>(
    vec3<f32>( 1.60475, -0.53108, -0.07367),
    vec3<f32>(-0.10208,  1.10813, -0.00605),
    vec3<f32>(-0.00327, -0.07276,  1.07602)
);

fn aces_fitted(color: vec3<f32>) -> vec3<f32> {
    return rtt_and_odt_fit(color * ACES_INPUT_MATRIX) * ACES_OUTPUT_MATRIX;
}

fn rtt_and_odt_fit(v: vec3<f32>) -> vec3<f32> {
    let a = v * (v + 0.0245786) - 0.000090537;
    let b = v * (0.983729 * v + 0.4329510) + 0.238081;
    return a/b;
}

fn fo4_lut(color: vec3<f32>) -> vec3<f32> {
    var c = log2(color) * 0.454545;
    c = exp2(c) * 0.937500 + 0.031250;
    return sample_lut(t_lut, c);
}

fn sample_lut(lut: texture_2d<f32>, color: vec3<f32>) -> vec3<f32> {
    // Our LUT texture is given as a 16*16x16 texture with the higher "layers"
    // in the X dimension representing an increase in blue.
    // Trilinearly interpolate by fetching a bilinear sample from two "layers"
    // and interpolate between them.
    // Once we support 3D textures we can remove all this in favor of simply
    // sampling the 3D texture.
    let r = min(color.r / 16.0, (1.0 - 16.0) / (1.0 - 256.0));
    let b = color.b * 15.0;
    let b_a = floor(b) / 16.0;
    let b_b = max(floor(b) + 1, 1.0) / 16.0;
    let b_s = fract(b);

    let uv1 = vec2<f32>(r + b_a, color.g);
    let uv2 = vec2<f32>(r + b_b, color.g);

    let v1 = textureSample(lut, t_sampler, uv1).rgb;
    let v2 = textureSample(lut, t_sampler, uv2).rgb;
    let v = v1 * (1.0 - b_s) + v2 * b_s;
    return v;
}
