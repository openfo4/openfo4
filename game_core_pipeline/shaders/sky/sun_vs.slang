import types;


struct Constants {
    Mat4 camera_view_proj;
    Mat4 transform;
}

struct Input {
    u32 index: SV_VertexID;
}

struct Output {
    Vec4 clip_position: SV_Position;
    [[vk::location(0)]] Vec2 uv;
}

static const u8[] INDICES = { 2, 1, 0, 3, 1, 2 };

static const Vec3[] POSITIONS = {
    Vec3(-1.0, -1.0, 0.0),
    Vec3(-1.0, 1.0, 0.0),
    Vec3(1.0, -1.0, 0.0),
    Vec3(1.0, 1.0, 0.0),
};

static const Vec2[] UVS = {
    Vec2(0.0, 0.0),
    Vec2(1.0, 0.0),
    Vec2(0.0, 1.0),
    Vec2(1.0, 1.0),
};

ConstantBuffer<Constants> constants;

[shader("vertex")]
func main(input: Input) -> Output {
    let index = INDICES[input.index];
    let position = POSITIONS[index];
    let uv = UVS[index];

    Output output;
    output.clip_position = mul(mul(constants.camera_view_proj, constants.transform), Vec4(position, 1.0));
    output.uv = uv;
    return output;
}
