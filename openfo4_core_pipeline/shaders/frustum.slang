import types;

public struct Frustum {
    f32 cos_v;
    f32 sin_v;
    f32 cos_h;
    f32 sin_h;
    f32 znear;
    f32 zfar;

    // Test a sphere against this frustum and returns whether it is partially or wholy visible.
    public func sphere_is_visible(Vec3 center, f32 radius) -> bool {
        // Based on
        // https://github.com/zeux/niagara/blob/567ebe331247e122425eaf1a6319d8c64daf816d/src/shaders/drawcull.comp.glsl#L77
        // and the formula: `z * sin(a) - abs(x) * cos(a) > -radius`.
        // With `a` being half the field of view angle.
        var visible = true;
        visible &= center.z * this.sin_v - abs(center.x) * this.cos_v > -radius;
        visible &= center.z * this.sin_h - abs(center.y) * this.cos_h > -radius;

        visible &= -center.z + radius > this.znear;
        visible &= -center.z - radius < this.zfar;

        return visible;
    }
}
