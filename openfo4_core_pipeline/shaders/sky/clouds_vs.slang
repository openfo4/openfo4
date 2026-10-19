import types;

struct Constants {
    Mat4 camera_view_proj;
    Mat4 transform;
}

struct Input {
    u32 draw_id: SV_DrawIndex;
    u32 index: SV_VertexID;
}

struct Offset {
    u32 positions;
    u32 colors;
    u32 uvs;
}

ConstantBuffer<Constants> constants;
StructuredBuffer<f32> positions;
StructuredBuffer<f32> colors;
StructuredBuffer<f32> uvs;
StructuredBuffer<Offset> offsets;

struct Output {
    Vec4 clip_position: SV_Position;
    [[vk::location(0)]] u32 cloud;
    [[vk::location(1)]] Vec4 color;
    [[vk::location(2)]] Vec2 uv;
}

[shader("vertex")]
func main(input: Input) -> Output {
    let offset = offsets[input.draw_id];
    let position = fetch<3>(positions, offset.positions, input.index);
    let color = fetch<4>(colors, offset.colors, input.index);
    let uv = fetch<2>(uvs, offset.uvs, input.index);

    Output output;
    output.clip_position = mul(mul(constants.camera_view_proj, constants.transform), Vec4(position, 1.0));
    output.color = color;
    output.cloud = input.draw_id;
    output.uv = uv;
    return output;
}

func fetch<let N: i32>(buffer: StructuredBuffer<f32>, offset: u32, index: u32) -> vector<f32, N> {
    var value = vector<f32, N>(0.0);
    for (var i = 0; i < N; i++) {
        value[i] = buffer[offset + index * N + i];
    }
    return value;
}
