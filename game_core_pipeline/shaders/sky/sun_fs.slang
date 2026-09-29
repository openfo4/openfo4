import types;

struct SunMaterial {
    Vec3 color;
    f32 scale;
}

SamplerState sampler;
Texture2D<Vec4> sun;
ConstantBuffer<SunMaterial> material;

struct Input {
    [[vk::location(0)]] Vec2 uv;
}

[shader("fragment")]
func main(input: Input) -> Vec4 {
    let sampled = sun.Sample(sampler, input.uv);
    var color =srgb_to_linearv(sampled.rgb) * material.color * material.scale;
    // color.r = srgb_to_linear(color.r);
    // color.g = srgb_to_linear(color.g);
    // color.b = srgb_to_linear(color.b);

    return Vec4(color, sampled.a);
}

func srgb_to_linear(color: f32) -> f32 {
    if (color <= 0.04045) {
        return color / 12.92;
    } else {
        return pow((color + 0.055) / 1.055, 2.4);
    }
}

func srgb_to_linearv(color: Vec3) -> Vec3 {
    return Vec3(srgb_to_linear(color.r), srgb_to_linear(color.g), srgb_to_linear(color.b));
}
