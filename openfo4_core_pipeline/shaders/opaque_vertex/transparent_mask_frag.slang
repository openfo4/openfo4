import types;
import shared;
import material;

StructuredBuffer<InstanceOut> instances;
StructuredBuffer<Material> materials;
Texture2D<Vec4>[] textures;
SamplerState sampler;

struct Input {
    vector<f32, 4> clip_position: SV_Position;
    [[vk::location(0)]] u32 instance_id;
    [[vk::location(1)]] u32 vertex_id;
    [[vk::location(2)]] Vec2 uv;
    [[vk::location(3)]] f32 alpha;
    [[vk::location(4)]] u32 draw_id;
}

struct Output {
    [[vk::location(0)]] vector<u32, 2> ids;
}

[shader("fragment")]
func main(input: Input) -> Output {
    let instance = instances[input.draw_id];
    let material = materials[instance.material_index];

    let alpha = textures[material.base_color_texture].Sample(sampler, input.uv).a * input.alpha * material.base_color.a;
    if (alpha < material.alpha_mask) {
        discard;
    }

    Output output;
    output.ids.x = input.instance_id;
    output.ids.y = input.vertex_id;
    return output;
}
