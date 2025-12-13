import types;

[[vk::push_constant]] vector<u32, 2> viewport_size;

[[vk::binding(0, 0)]] StructuredBuffer<Vertex> vertices;
[[vk::binding(1, 0)]] StructuredBuffer<Instance> instances;

struct Instance {
    u32 texture_index;
}

struct Vertex {
    Vec2 position;
    Vec2 uv;
    Vec4 color;
}

struct Input {
    u32 draw_id: SV_DrawIndex;
    u32 vertex_index: SV_VertexID;
    u32 first_vertex: SV_StartVertexLocation;
}

struct Output {
    Vec4 clip_position: SV_Position;
    [[vk::location(0)]] u32 texture_index;
    [[vk::location(1)]] Vec2 uv;
    [[vk::location(2)]] Vec4 color;
}

[shader("vertex")]
func main(input: Input) -> Output {
    let instance = instances[input.draw_id];
    let vertex = vertices[input.first_vertex + input.vertex_index];

    let x = (vertex.position.x / viewport_size.x) * 2.0 - 1.0;
    let y = (vertex.position.y / viewport_size.y) * -2.0 + 1.0;

    Output output;
    output.clip_position = Vec4(x, y, 0.0, 1.0);
    output.uv = vertex.uv;
    output.color = vertex.color;
    output.texture_index = instance.texture_index;
    return output;
}
