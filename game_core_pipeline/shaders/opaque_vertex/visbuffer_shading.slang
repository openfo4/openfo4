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
        output[coords] = Vec4(0.0);
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

    let uv = load_uv(barys, offsets, indices);
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
    luminance += pbr_material.diffuse_color() * constants.scene.ambient;

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
    let range = 1;

    let texel_size = 1.0 / Vec2(4096);

    var value = 0.0;
    for (int x = -range; x <= range; x++) {
        for (int y = -range; y <= range; y++) {
            let pcf = shadow_map.SampleGrad(sampler, coords + Vec2(x, y) * texel_size, Vec2(0.0), Vec2(0.0)).r;
            if (depth > pcf) {
                value += 1.0;
            }
        }
    }

    let size = range * 2 + 1;
    return value / (size * size);
}

func fetch<let N: i32>(buffer: StructuredBuffer<f32>, offset: u32, index: u32) -> vector<f32, N> {
    var value = vector<f32, N>(0.0);
    for (var i = 0; i < N; i++) {
        value[i] = buffer[offset + index * N + i];
    }
    return value;
}

Gradient load_uv(Barycentrics barys, MeshOffsets offsets, u32 indices[3]) {
    let uv_a = fetch<2>(uvs, offsets.uv, indices[0]);
    let uv_b = fetch<2>(uvs, offsets.uv, indices[1]);
    let uv_c = fetch<2>(uvs, offsets.uv, indices[2]);
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
