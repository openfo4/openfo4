import types;

struct SkyColors {
    Vec3 sky_upper;
    f32 sky_scale;
    Vec3 sky_lower;
    Vec3 horizon;
}

ConstantBuffer<SkyColors> sky_colors;

struct Input {
    [[vk::location(0)]] Vec4 color;
}

[shader("fragment")]
func main(input: Input) -> Vec4 {
    let horizon = input.color.r * sky_colors.horizon;
    let sky_lower = input.color.g * sky_colors.sky_lower;
    let sky_upper = input.color.b * sky_colors.sky_upper;

    return Vec4(horizon + sky_lower + sky_upper, 1.0) * sky_colors.sky_scale;
    // return Vec4(input.color.rgb, 1.0);
}
