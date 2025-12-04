//! Object memory layout

#[repr(C)]
pub struct Header {
    layouts_len: usize,
    // layouts: [ClassLayout],
}
