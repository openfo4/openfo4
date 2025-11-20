mod object;

#[derive(Copy, Clone)]
pub union RawVal {
    i32: i32,
    f32: f32,
    ptr: *const u8,
}
