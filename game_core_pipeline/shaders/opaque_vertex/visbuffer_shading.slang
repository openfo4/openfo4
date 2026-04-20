import types;
import lights;
import shared;
import material;
import camera;
import scene;

import barycentrics;
import pbr;

struct PushConstants {
    SceneData scene;
    Camera camera;
    u32 num_directional_lights;
    u32 num_point_lights;
    u32 num_spot_lights;
    vector<u32, 2> scren_size;
}

ConstantBuffer<PushConstants> constants;

Texture2D<vector<u32, 2>> visbuffer;
RWTexture2D<Vec4> output;

StructuredBuffer<f32> positions;
StructuredBuffer<f32> normals;
StructuredBuffer<f32> uvs;
StructuredBuffer<f32> tangents;
StructuredBuffer<f32> colors;
StructuredBuffer<MeshOffsets> offsets;
StructuredBuffer<InstanceOut> instances;
StructuredBuffer<u32> index_buffer;
StructuredBuffer<InstanceOut> instances_transparent_mask;

StructuredBuffer<DirectionalLight> directional_lights;
StructuredBuffer<PointLight> point_lights;
StructuredBuffer<SpotLight> spot_lights;
StructuredBuffer<Material> materials;
Texture2D<Vec4>[] textures;
SamplerState sampler;
Texture2D<Vec4> shadow_map;
ConstantBuffer<Mat4> light_view_proj;

SamplerComparisonState shadow_sampler;

static const u32 NUM_THREADS = 64;

[SpecializationConstant]
const u32 TILE_SIZE;

[shader("compute")]
[numthreads(TILE_SIZE, TILE_SIZE, 1)]
void main(
    vector<u32, 2> tid: SV_GroupThreadID,
    vector<u32, 2> gid: SV_GroupID,
) {
    let coords = gid * TILE_SIZE + tid;

    if (coords.x >= constants.scren_size.x || coords.y >= constants.scren_size.y) {
        return;
    }

    let input = visbuffer[coords];
    // Value 0 for InstanceID indicates no instance at this pixel.
    if (input.x == 0) {
        return;
    }

    let instance_id = input.x - 1;
    let triangle_id = input.y;

    // TODO: For now the MSB indicates that the instance comes from the
    // transparent instance buffer. In the future we would like to
    // combine both buffers such that this is not necessary.
    InstanceOut instance;
    if ((instance_id & (1 << 31)) != 0) {
        static const u32 MASK = (1 << 31) - 1;
        instance = instances_transparent_mask[instance_id & MASK];
    } else {
        instance = instances[instance_id];
    }

    let offsets = offsets[instance.offsets_index];
    let material = materials[instance.material_index];

    let index0 = index_buffer[instance.index_offset + triangle_id * 3 + 0];
    let index1 = index_buffer[instance.index_offset + triangle_id * 3 + 1];
    let index2 = index_buffer[instance.index_offset + triangle_id * 3 + 2];
    let indices: u32[3] = {index0, index1, index2};

    let pos_0 = fetch<3>(positions, offsets.position, index0);
    let pos_1 = fetch<3>(positions, offsets.position, index1);
    let pos_2 = fetch<3>(positions, offsets.position, index2);

    let world_pos_a = mul(instance.transform, Vec4(pos_0, 1.0));
    let world_pos_b = mul(instance.transform, Vec4(pos_1, 1.0));
    let world_pos_c = mul(instance.transform, Vec4(pos_2, 1.0));

    let pos_a = mul(constants.camera.view_proj, world_pos_a);
    let pos_b = mul(constants.camera.view_proj, world_pos_b);
    let pos_c = mul(constants.camera.view_proj, world_pos_c);

    let ndc_x = (f32(coords.x) / f32(constants.scren_size.x)) * 2.0 - 1.0;
    let ndc_y = (f32(coords.y) / f32(constants.scren_size.y)) * -2.0 + 1.0;
    let ndc = Vec2(ndc_x, ndc_y);

    let barys = Barycentrics(pos_a, pos_b, pos_c, ndc, constants.scren_size);

    let world_position = barys.interpolate(world_pos_a.xyz, world_pos_b.xyz, world_pos_c.xyz);

    let uv = load_uv(barys, offsets, indices, material.uv_scale, material.uv_offset);
    let normal = load_normal(barys, offsets, indices, material, uv, instance);

    let vert_color_a = fetch<4>(colors, offsets.color, index0);
    let vert_color_b = fetch<4>(colors, offsets.color, index1);
    let vert_color_c = fetch<4>(colors, offsets.color, index2);
    let vert_color = barys.interpolate(vert_color_a, vert_color_b, vert_color_c);

    let base_color_texture = material.base_color * sample_material_texture(material.base_color_texture, uv);
    var base_color = base_color_texture * vert_color;
    if ((material.flags & MATERIAL_FLAGS_GRAYSCALE_PALETTE) != 0) {
        let x = clamp(base_color_texture.g, 0.0, 1.0);
        let y = clamp(material.grayscale_palette_scale - (1.0 - vert_color.r), 0.0, 1.0);
        base_color =  textures[NonUniformResourceIndex(material.grayscale_palette_texture)].SampleLevel(sampler, Vec2(x, y), 0.0);
    }

    if ((material.flags & MATERIAL_FLAGS_UNLIT) != 0) {
        output[coords] = Vec4(base_color.rgb, 1.0);
        return;
    }

    let specular_strength = material.specular_strength * sample_material_texture(material.specular_glossiness_texture, uv).r;
    let specular_color = material.specular_color.rgb;

    var roughness = material.roughness * sample_material_texture(material.metallic_roughness_texture, uv).g;
    if ((material.flags & MATERIAL_FLAGS_ROUGHNESS_FROM_GLOSSINESS) != 0) {
        let glossiness = (1.0 - material.roughness) * sample_material_texture(material.specular_glossiness_texture, uv).g;
        roughness = 1.0 - glossiness;
    }

    var metallic = material.metallic * sample_material_texture(material.metallic_roughness_texture, uv).b;
    if ((material.flags & MATERIAL_FLAGS_METALLIC_FROM_SPECULAR) != 0) {
        metallic = compute_metallic_from_specular_color(specular_color * specular_strength);
    }

    // BRDF parameters
    let view_dir = normalize(constants.camera.position - world_position);
    var pbr_material: pbr::Material;
    pbr_material.base_color = base_color.rgb;
    pbr_material.normal = normal;
    pbr_material.metallic = metallic;
    pbr_material.roughness = roughness;
    pbr_material.reflectance = material.reflectance;
    pbr_material.specular_color = specular_color;
    pbr_material.specular_strength = specular_strength;

    var luminance = Vec3(0.0);

    // Ambient light
    luminance += eval_indirect_light(constants.scene.ambient, pbr_material, -view_dir);

    for (var i = 0; i < constants.num_directional_lights; i++) {
        let frag_light_space = mul(light_view_proj, Vec4(world_position, 1.0));
        var proj_coords = (frag_light_space.xyz / frag_light_space.w);
        proj_coords.x = proj_coords.x * 0.5 + 0.5;
        proj_coords.y = 1.0 - (proj_coords.y * 0.5 + 0.5);

        let texture_size = Vec2(4096);
        let texel_size = 1.0 / Vec2(4096);

        var shadow = 0.0;
        if (proj_coords.z > 1.0) {
            shadow = 1.0;
        } else {
            shadow = sample_pcf_shadow(proj_coords.xy, proj_coords.z);
        }

        // FIXME: Early reject, good for performance or not?
        if (shadow == 0.0) {
            continue;
        }

        let light = directional_lights[i].eval();
        luminance += pbr::surface_shading(pbr_material, view_dir, light) * shadow;
    }

    for (var i = 0; i < constants.num_point_lights; i++) {
        let light = point_lights[i].eval(FragParams(world_position));
        luminance += pbr::surface_shading(pbr_material, view_dir, light);
    }

    for (var i = 0; i < constants.num_spot_lights; i++) {
        let light = spot_lights[i].eval(FragParams(world_position));
        luminance += pbr::surface_shading(pbr_material, view_dir, light);
    }

    output[coords] = Vec4(luminance, 1.0);
}

struct InputParams {
    Barycentrics barys;
    Material material;
    InstanceOut instance;
    MeshOffsets offsets;
}

func sample_pcf_shadow(coords: Vec2, depth: f32) -> f32 {
    // https://github.com/TheRealMJP/Shadows/blob/338290a57e7c65d493783fd48f39a28321e84e81/Shadows/PCFKernels.hlsl#L67
    static const float2 PoissonSamples[64] = {
        float2(-0.5119625f, -0.4827938f),
        float2(-0.2171264f, -0.4768726f),
        float2(-0.7552931f, -0.2426507f),
        float2(-0.7136765f, -0.4496614f),
        float2(-0.5938849f, -0.6895654f),
        float2(-0.3148003f, -0.7047654f),
        float2(-0.42215f, -0.2024607f),
        float2(-0.9466816f, -0.2014508f),
        float2(-0.8409063f, -0.03465778f),
        float2(-0.6517572f, -0.07476326f),
        float2(-0.1041822f, -0.02521214f),
        float2(-0.3042712f, -0.02195431f),
        float2(-0.5082307f, 0.1079806f),
        float2(-0.08429877f, -0.2316298f),
        float2(-0.9879128f, 0.1113683f),
        float2(-0.3859636f, 0.3363545f),
        float2(-0.1925334f, 0.1787288f),
        float2(0.003256182f, 0.138135f),
        float2(-0.8706837f, 0.3010679f),
        float2(-0.6982038f, 0.1904326f),
        float2(0.1975043f, 0.2221317f),
        float2(0.1507788f, 0.4204168f),
        float2(0.3514056f, 0.09865579f),
        float2(0.1558783f, -0.08460935f),
        float2(-0.0684978f, 0.4461993f),
        float2(0.3780522f, 0.3478679f),
        float2(0.3956799f, -0.1469177f),
        float2(0.5838975f, 0.1054943f),
        float2(0.6155105f, 0.3245716f),
        float2(0.3928624f, -0.4417621f),
        float2(0.1749884f, -0.4202175f),
        float2(0.6813727f, -0.2424808f),
        float2(-0.6707711f, 0.4912741f),
        float2(0.0005130528f, -0.8058334f),
        float2(0.02703013f, -0.6010728f),
        float2(-0.1658188f, -0.9695674f),
        float2(0.4060591f, -0.7100726f),
        float2(0.7713396f, -0.4713659f),
        float2(0.573212f, -0.51544f),
        float2(-0.3448896f, -0.9046497f),
        float2(0.1268544f, -0.9874692f),
        float2(0.7418533f, -0.6667366f),
        float2(0.3492522f, 0.5924662f),
        float2(0.5679897f, 0.5343465f),
        float2(0.5663417f, 0.7708698f),
        float2(0.7375497f, 0.6691415f),
        float2(0.2271994f, -0.6163502f),
        float2(0.2312844f, 0.8725659f),
        float2(0.4216993f, 0.9002838f),
        float2(0.4262091f, -0.9013284f),
        float2(0.2001408f, -0.808381f),
        float2(0.149394f, 0.6650763f),
        float2(-0.09640376f, 0.9843736f),
        float2(0.7682328f, -0.07273844f),
        float2(0.04146584f, 0.8313184f),
        float2(0.9705266f, -0.1143304f),
        float2(0.9670017f, 0.1293385f),
        float2(0.9015037f, -0.3306949f),
        float2(-0.5085648f, 0.7534177f),
        float2(0.9055501f, 0.3758393f),
        float2(0.7599946f, 0.1809109f),
        float2(-0.2483695f, 0.7942952f),
        float2(-0.4241052f, 0.5581087f),
        float2(-0.1020106f, 0.6724468f),
    };

    // Outside of shadow map texture.
    if (coords.x < 0.0 || coords.y < 0.0 || coords.x > 1.0 || coords.y > 1.0) {
        return 1.0;
    }

    let texel_size = 1.0 / Vec2(4096);

    var sum = 0.0;
    for (int i = 0; i < PoissonSamples.getCount(); i++) {
        let offset = PoissonSamples[i] * texel_size * 8.0;
        let pcf = shadow_map.SampleCmpLevelZero(shadow_sampler, coords + offset, depth);
        sum += pcf;
    }

    return sum / f32(PoissonSamples.getCount());
}

func fetch<let N: i32>(buffer: StructuredBuffer<f32>, offset: u32, index: u32) -> vector<f32, N> {
    var value = vector<f32, N>(0.0);
    for (var i = 0; i < N; i++) {
        value[i] = buffer[offset + index * N + i];
    }
    return value;
}

Gradient load_uv(Barycentrics barys, MeshOffsets offsets, u32 indices[3], Vec2 scale, Vec2 offset) {
    let uv_a = fetch<2>(uvs, offsets.uv, indices[0]) * scale + offset;
    let uv_b = fetch<2>(uvs, offsets.uv, indices[1]) * scale + offset;
    let uv_c = fetch<2>(uvs, offsets.uv, indices[2]) * scale + offset;
    return barys.interpolate_grad(uv_a, uv_b, uv_c);
}

Vec3 load_normal(Barycentrics barys, MeshOffsets offsets, u32 indices[3], Material material, Gradient uv, InstanceOut instance) {
    let normal_a = fetch<3>(normals, offsets.normal, indices[0]);
    let normal_b = fetch<3>(normals, offsets.normal, indices[1]);
    let normal_c = fetch<3>(normals, offsets.normal, indices[2]);
    var vert_normal = barys.interpolate(normal_a, normal_b, normal_c);
    vert_normal = normalize(mul(instance.normal, vert_normal));

    let tangent_a = fetch<4>(tangents, offsets.tangent, indices[0]);
    let tangent_b = fetch<4>(tangents, offsets.tangent, indices[1]);
    let tangent_c = fetch<4>(tangents, offsets.tangent, indices[2]);
    var tangent = barys.interpolate(tangent_a.xyz, tangent_b.xyz, tangent_c.xyz);
    tangent = normalize(mul(instance.normal, tangent));

    // FIXME: This should be the same as cross(vert_normal, tangent) * tangent.w
    // if tangent.w is consistent across all vertices, but somehow it is not?
    let bitan_a = cross(normal_a, tangent_a.xyz) * tangent_a.w;
    let bitan_b = cross(normal_b, tangent_b.xyz) * tangent_b.w;
    let bitan_c = cross(normal_c, tangent_c.xyz) * tangent_c.w;
    var bitangent = barys.interpolate(bitan_a, bitan_b, bitan_c);
    bitangent = normalize(mul(instance.normal, bitangent));

    let tbn = Mat3(tangent, bitangent, vert_normal);


    var normal = sample_material_texture(material.normal_texture, uv).xyz;

    // DirectX normal maps have the Y component flipped.
    if ((material.flags & MATERIAL_FLAGS_FLIP_NORMAL_Y) != 0) {
        normal.y = 1.0 - normal.y;
    }

    // Transform from [0..1] to [-1..1].
    normal = normal * 2.0 - 1.0;

    if ((material.flags & MATERIAL_FLAGS_NORMAL_ENCODING_TWO_COMPONENT) != 0) {
        normal.z = sqrt(clamp(1.0 - dot(normal.xy, normal.xy), 0.0, 1.0));
    }

    normal = normalize(normal);
    normal = normalize(mul(normal, tbn));

    return normal;
}


func compute_metallic_from_specular_color(specular_color: Vec3) -> f32 {
    return max(max(specular_color.r, specular_color.g), specular_color.b);
}

func sample_material_texture(texture_index: u32, uv: Gradient) -> Vec4 {
    // We need to mark the index as non-uniform because a single
    // workgroup may access divergent textures here.
    return textures[NonUniformResourceIndex(texture_index)].SampleGrad(sampler, uv.value, uv.dx, uv.dy);
}
