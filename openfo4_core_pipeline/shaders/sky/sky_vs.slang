import types;

struct Constants {
    Mat4 camera_view_proj;
    Mat4 transform;
}

ConstantBuffer<Constants> constants;
StructuredBuffer<f32> positions;
StructuredBuffer<f32> colors;

struct Input {
    u32 index: SV_VertexID;
}

struct Output {
    Vec4 clip_position: SV_Position;
    [[vk::location(0)]] Vec4 color;
}

[shader("vertex")]
func main(input: Input) -> Output {
    let position = fetch<3>(positions, 0, input.index);
    let color = fetch<4>(colors, 0, input.index);

    Output output;
    output.clip_position = mul(mul(constants.camera_view_proj, constants.transform), Vec4(position, 1.0));
    output.color = color;
    return output;
}

func fetch<let N: i32>(buffer: StructuredBuffer<f32>, offset: u32, index: u32) -> vector<f32, N> {
    var value = vector<f32, N>(0.0);
    for (var i = 0; i < N; i++) {
        value[i] = buffer[offset + index * N + i];
    }
    return value;
}
