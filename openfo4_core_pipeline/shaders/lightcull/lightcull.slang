import types;
import frustum;

const static u32 NUM_THREADS = 64;

ConstantBuffer<Frustum> frustum;
ConstantBuffer<Mat4> view;
StructuredBuffer<Vec4> input;
RWStructuredBuffer<u32> output;

ConstantBuffer<u32> input_count;
RWStructuredBuffer<Atomic<u32>> output_count;

[shader("compute")]
[numthreads(NUM_THREADS, 1, 1)] 
func main(
    u32 gid: SV_GroupID,
    u32 tid: SV_GroupThreadID,
) {
    let index = gid * NUM_THREADS + tid;
    if (index >= input_count) {
        return;
    }

    let sphere = input[index];
    var center = (mul(view, Vec4(sphere.xyz, 1.0))).xyz;
    let radius = sphere.w;

    if (!frustum.sphere_is_visible(center, radius)) {
        return;
    }

    let out_index = output_count[0].add(1);
    output[out_index] = index;
}
