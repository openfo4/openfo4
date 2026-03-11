import types;

struct SunMaterial {
    Vec3 color;
    f32 scale;
}

SamplerState sampler;
Texture2D<Vec4> sun;

struct Input {
    [[vk::location(0)]] Vec2 uv;
}

[shader("fragment")]
func main(input: Input) -> Vec4 {
    return sun.Sample(sampler, input.uv) * 1.0;
}
