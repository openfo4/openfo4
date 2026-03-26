import types;

struct CloudLayer {
    Vec4 color;
    Vec2 uv_offset;
    u32 texture;
}

StructuredBuffer<CloudLayer> layers;
Texture2D<Vec4>[] textures;
SamplerState sampler;

struct Input {
    [[vk::location(0)]] u32 cloud;
    [[vk::location(1)]] Vec4 color;
    [[vk::location(2)]] Vec2 uv;
}

[shader("fragment")]
func main(input: Input) -> Vec4 {
    let layer = layers[input.cloud];
    if (layer.texture == u32.maxValue) {
        return Vec4(0.0);
    }

    let uv = input.uv + layer.uv_offset;
    let color = textures[layer.texture].Sample(sampler, uv) * layer.color * input.color;
    return color;
}
