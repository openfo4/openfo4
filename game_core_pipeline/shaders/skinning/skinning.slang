import types;

ConstantBuffer<u32> instance_count;

StructuredBuffer<f32> positions_in;
StructuredBuffer<f32> normals_in;
StructuredBuffer<f32> tangents_in;

RWStructuredBuffer<f32> positions_out;
RWStructuredBuffer<f32> normals_out;
RWStructuredBuffer<f32> tangents_out;

StructuredBuffer<u8> indices;
StructuredBuffer<f32> weights;

struct SkinInstance {
    u32 bone_offset;
    u32 vertex_count;
    u32 positions_in_offset;
    u32 normals_in_offset;
    u32 tangents_in_offset;
    u32 positions_out_offset;
    u32 normals_out_offset;
    u32 tangents_out_offset;
    u32 indices_offset;
    u32 weights_offset;
}

StructuredBuffer<Mat4> bones;
StructuredBuffer<SkinInstance> instances;

struct BoneTransform {
    Mat4 transform;
    Mat3 rotation;
}

// FIXME: This workgroup layout is suboptimal, change to 32/64.
[shader("compute")]
[numthreads(1, 1, 1)]
func main(
    vector<u32, 2> gid: SV_GroupID,
) {
    let xy = gid;
    let instance_index = xy.x;
    let vertex_index = xy.y;

    if (instance_index >= instance_count) {
        return;
    }

    let instance = instances[instance_index];

    if (vertex_index >= instance.vertex_count) {
        return;
    }

    // https://developer.download.nvidia.com/assets/gamedev/docs/skinning.pdf

    let indices = load<u8, 4>(indices, instance.indices_offset, vertex_index);
    let weights = load<f32, 4>(weights, instance.weights_offset, vertex_index);

    // Note: The caller of the shader must validate that all indices are in bounds
    // given the number of bones. This means we can skip bounds checks here.
    var matrix = Mat4(0.0);
    matrix += bones[instance.bone_offset + indices.x] * weights.x;
    matrix += bones[instance.bone_offset + indices.y] * weights.y;
    matrix += bones[instance.bone_offset + indices.z] * weights.z;
    matrix += bones[instance.bone_offset + indices.w] * weights.w;

    let normal_matrix = Mat3(transpose(inverse(matrix)));

    let position_in = load<f32, 3>(positions_in, instance.positions_in_offset, vertex_index);
    let normal_in = load<f32, 3>(normals_in, instance.normals_in_offset, vertex_index);
    let tangents_in = load<f32, 4>(tangents_in, instance.tangents_in_offset, vertex_index);

    let position = mul(matrix, Vec4(position_in, 1.0)).xyz;
    let normal = normalize(mul(normal_matrix, normal_in));
    let tangent = Vec4(normalize(mul(normal_matrix, tangents_in.xyz)), tangents_in.w);

    store<f32, 3>(positions_out, instance.positions_out_offset, vertex_index, position);
    store<f32, 3>(normals_out, instance.normals_out_offset, vertex_index, normal);
    store<f32, 4>(tangents_out, instance.tangents_out_offset, vertex_index, tangent);
}

func load<T, let N: i32>(buffer: StructuredBuffer<T>, offset: u32, index: u32) -> vector<T, N> {
    var value: vector<T, N>;
    for (var i = 0; i < N; i++) {
        value[i] = buffer[offset + index * N + i];
    }
    return value;
}

func store<T, let N: i32>(buffer: RWStructuredBuffer<T>, offset: u32, index: u32, value: vector<T, N>) {
    for (var i = 0; i < N; i++) {
        buffer[offset + index * N + i] = value[i];
    }
}

// From https://gist.github.com/mattatz/86fff4b32d198d0928d0fa4ff32cf6fa
float4x4 inverse(float4x4 m) {
    float n11 = m[0][0], n12 = m[1][0], n13 = m[2][0], n14 = m[3][0];
    float n21 = m[0][1], n22 = m[1][1], n23 = m[2][1], n24 = m[3][1];
    float n31 = m[0][2], n32 = m[1][2], n33 = m[2][2], n34 = m[3][2];
    float n41 = m[0][3], n42 = m[1][3], n43 = m[2][3], n44 = m[3][3];

    float t11 = n23 * n34 * n42 - n24 * n33 * n42 + n24 * n32 * n43 - n22 * n34 * n43 - n23 * n32 * n44 + n22 * n33 * n44;
    float t12 = n14 * n33 * n42 - n13 * n34 * n42 - n14 * n32 * n43 + n12 * n34 * n43 + n13 * n32 * n44 - n12 * n33 * n44;
    float t13 = n13 * n24 * n42 - n14 * n23 * n42 + n14 * n22 * n43 - n12 * n24 * n43 - n13 * n22 * n44 + n12 * n23 * n44;
    float t14 = n14 * n23 * n32 - n13 * n24 * n32 - n14 * n22 * n33 + n12 * n24 * n33 + n13 * n22 * n34 - n12 * n23 * n34;

    float det = n11 * t11 + n21 * t12 + n31 * t13 + n41 * t14;
    float idet = 1.0f / det;

    float4x4 ret;

    ret[0][0] = t11 * idet;
    ret[0][1] = (n24 * n33 * n41 - n23 * n34 * n41 - n24 * n31 * n43 + n21 * n34 * n43 + n23 * n31 * n44 - n21 * n33 * n44) * idet;
    ret[0][2] = (n22 * n34 * n41 - n24 * n32 * n41 + n24 * n31 * n42 - n21 * n34 * n42 - n22 * n31 * n44 + n21 * n32 * n44) * idet;
    ret[0][3] = (n23 * n32 * n41 - n22 * n33 * n41 - n23 * n31 * n42 + n21 * n33 * n42 + n22 * n31 * n43 - n21 * n32 * n43) * idet;

    ret[1][0] = t12 * idet;
    ret[1][1] = (n13 * n34 * n41 - n14 * n33 * n41 + n14 * n31 * n43 - n11 * n34 * n43 - n13 * n31 * n44 + n11 * n33 * n44) * idet;
    ret[1][2] = (n14 * n32 * n41 - n12 * n34 * n41 - n14 * n31 * n42 + n11 * n34 * n42 + n12 * n31 * n44 - n11 * n32 * n44) * idet;
    ret[1][3] = (n12 * n33 * n41 - n13 * n32 * n41 + n13 * n31 * n42 - n11 * n33 * n42 - n12 * n31 * n43 + n11 * n32 * n43) * idet;

    ret[2][0] = t13 * idet;
    ret[2][1] = (n14 * n23 * n41 - n13 * n24 * n41 - n14 * n21 * n43 + n11 * n24 * n43 + n13 * n21 * n44 - n11 * n23 * n44) * idet;
    ret[2][2] = (n12 * n24 * n41 - n14 * n22 * n41 + n14 * n21 * n42 - n11 * n24 * n42 - n12 * n21 * n44 + n11 * n22 * n44) * idet;
    ret[2][3] = (n13 * n22 * n41 - n12 * n23 * n41 - n13 * n21 * n42 + n11 * n23 * n42 + n12 * n21 * n43 - n11 * n22 * n43) * idet;

    ret[3][0] = t14 * idet;
    ret[3][1] = (n13 * n24 * n31 - n14 * n23 * n31 + n14 * n21 * n33 - n11 * n24 * n33 - n13 * n21 * n34 + n11 * n23 * n34) * idet;
    ret[3][2] = (n14 * n22 * n31 - n12 * n24 * n31 - n14 * n21 * n32 + n11 * n24 * n32 + n12 * n21 * n34 - n11 * n22 * n34) * idet;
    ret[3][3] = (n12 * n23 * n31 - n13 * n22 * n31 + n13 * n21 * n32 - n11 * n23 * n32 - n12 * n21 * n33 + n11 * n22 * n33) * idet;

    return ret;
}
