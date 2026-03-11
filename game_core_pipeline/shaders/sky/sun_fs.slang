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
    let color = sampled.rgb * material.color * material.scale;
    return Vec4(color, sampled.a);
}
