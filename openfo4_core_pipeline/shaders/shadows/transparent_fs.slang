import types;
import shared;
import material;

StructuredBuffer<InstanceOut> instances;
StructuredBuffer<Material> materials;
Texture2D<Vec4>[] textures;
SamplerState sampler;

struct Input {
    [[vk::location(0)]] u32 instance_id;
    [[vk::location(1)]] Vec2 uv;
    [[vk::location(2)]] f32 alpha;
}

[shader("fragment")]
func main(input: Input) {
    let instance = instances[input.instance_id];
    let material = materials[instance.material_index];

    let alpha = textures[material.base_color_texture].Sample(sampler, input.uv).a * input.alpha;
    if (alpha < material.alpha_mask) {
        discard;
    }
}
