import types;

public struct SceneData {
    public Ambient ambient;
}

public struct Ambient {
    public Vec3 top;
    u32 _pad0;
    public Vec3 bottom;
    u32 _pad1;
    public Vec3 left;
    u32 _pad2;
    public Vec3 right;
    u32 _pad3;
    public Vec3 front;
    u32 _pad4;
    public Vec3 back;
    u32 _pad5;

    public func sample(dir: Vec3) -> Vec3 {
        let wp = pow(max(dir, 0.0), 2.0);
        let wn = pow(max(-dir, 0.0), 2.0);

        let sum = wp.x + wp.y + wp.z + wn.x + wn.y + wn.z;

        let xp = wp.x * this.right;
        let xn = wn.x * this.left;
        let yp = wp.y * this.top;
        let yn = wn.y * this.bottom;
        let zp = wp.z * this.back;
        let zn = wn.z * this.front;

        return (xp + xn + yp + yn + zp + zn) / sum;
    }
}
