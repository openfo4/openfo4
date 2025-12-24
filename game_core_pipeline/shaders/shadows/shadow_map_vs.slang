import types;
import material;
import shared;

ConstantBuffer<Mat4> light_view_proj;

StructuredBuffer<f32> positions;
StructuredBuffer<MeshOffsets> offsets;
StructuredBuffer<InstanceOut> instances;
StructuredBuffer<u32> index_buffer;
StructuredBuffer<Material> materials;


struct Input {
    u32 draw_id: SV_DrawIndex;
    // In Slang SV_VertexID is VertexIndex - BaseVertex.
    // We dispatch one vertex shader invocation per index
    // in the index buffer and then do indexing manually.
    // This value refers to current index of the vertex.
    u32 index: SV_VertexID;
}

struct Output {
    vector<f32, 4> clip_position: SV_Position;
}

[shader("vertex")]
func main(input: Input) -> Output {
    let instance = instances[input.draw_id];
    let offsets = offsets[instance.offsets_index];
    let material = materials[instance.material_index];

    let vertex_index = index_buffer[instance.index_offset + input.index];

    let position = fetch<3>(positions, offsets.position, vertex_index);

    Output output;
    output.clip_position = mul(light_view_proj, mul(instance.transform, Vec4(position, 1.0)));
    output.clip_position.z += material.depth_bias;
    return output;
}

func fetch<let N: i32>(buffer: StructuredBuffer<f32>, offset: u32, index: u32) -> vector<f32, N> {
    var value = vector<f32, N>(0.0);
    for (var i = 0; i < N; i++) {
        value[i] = buffer[offset + index * N + i];
    }
    return value;
}
