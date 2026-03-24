import types;

struct CloudLayer {
    Vec4 color;
    u32 texture;
}

StructuredBuffer<CloudLayer> layers;
Texture2D<Vec4>[] textures;
SamplerState sampler;

static const float4 colors[] = {
    float4(0,0,0,1),
    float4(189.0f / 255.0f, 236.0f / 255.0f, 182.0f / 255.0f, 1.0f),
    float4(108.0f / 255.0f, 112.0f / 255.0f,  89.0f / 255.0f, 1.0f),
    float4(203.0f / 255.0f, 208.0f / 255.0f, 204.0f / 255.0f, 1.0f),
    float4(250.0f / 255.0f, 210.0f / 255.0f,  1.0f / 255.0f, 1.0f),
    float4(220.0f / 255.0f, 156.0f / 255.0f,   0.0f / 255.0f, 1.0f),
    float4( 42.0f / 255.0f, 100.0f / 255.0f, 120.0f / 255.0f, 1.0f),
    float4(120.0f / 255.0f, 133.0f / 255.0f, 139.0f / 255.0f, 1.0f),
    float4(121.0f / 255.0f,  85.0f / 255.0f,  61.0f / 255.0f, 1.0f),
    float4(157.0f / 255.0f, 145.0f / 255.0f,  1.0f / 255.0f, 1.0f),
    float4(166.0f / 255.0f,  94.0f / 255.0f,  46.0f / 255.0f, 1.0f),
    float4(203.0f / 255.0f,  40.0f / 255.0f,  33.0f / 255.0f, 1.0f),
    float4(243.0f / 255.0f, 159.0f / 255.0f,  24.0f / 255.0f, 1.0f),
    float4(250.0f / 255.0f, 210.0f / 255.0f,  1.0f / 255.0f, 1.0f),
    float4(114.0f / 255.0f,  20.0f / 255.0f,  34.0f / 255.0f, 1.0f),
    float4( 64.0f / 255.0f,  58.0f / 255.0f,  58.0f / 255.0f, 1.0f),
    float4(157.0f / 255.0f, 161.0f / 255.0f, 170.0f / 255.0f, 1.0f),
    float4(164.0f / 255.0f, 125.0f / 255.0f, 144.0f / 255.0f, 1.0f),
    float4(248.0f / 255.0f,   0.0f / 255.0f,   0.0f / 255.0f, 1.0f),
    float4(120.0f / 255.0f,  31.0f / 255.0f,  25.0f / 255.0f, 1.0f),
    float4( 51.0f / 255.0f,  47.0f / 255.0f,  44.0f / 255.0f, 1.0f),
    float4(180.0f / 255.0f,  76.0f / 255.0f,  67.0f / 255.0f, 1.0f),
    float4(125.0f / 255.0f, 132.0f / 255.0f, 113.0f / 255.0f, 1.0f),
    float4(161.0f / 255.0f,  35.0f / 255.0f,  18.0f / 255.0f, 1.0f),
    float4(142.0f / 255.0f,  64.0f / 255.0f,  42.0f / 255.0f, 1.0f),
    float4(130.0f / 255.0f, 137.0f / 255.0f, 143.0f / 255.0f, 1.0f),
};

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

    let color = textures[layer.texture].Sample(sampler, input.uv) * layer.color * input.color;
    return color;
}
