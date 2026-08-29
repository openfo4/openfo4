import types;

Texture2D<Vec4>[] textures;
SamplerState sampler;

struct Input {
    Vec4 clip_position: SV_Position;
    [[vk::location(0)]] u32 texture_index;
    [[vk::location(1)]] Vec2 uv;
    [[vk::location(2)]] Vec4 color;
}

[shader("fragment")]
func main(input: Input) -> Vec4 {
    return input.color * textures[input.texture_index].Sample(sampler, input.uv);
}
