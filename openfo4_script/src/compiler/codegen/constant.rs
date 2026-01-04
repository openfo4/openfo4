use std::alloc::Layout;

use crate::runtime::StringObject;

// FIXME: Add this as an API to the JIT compiler.
pub fn create_string_constant(constants: &mut Vec<ConstantData>, string: &str) -> *const u8 {
    let data_layout = Layout::array::<u8>(string.len()).unwrap();
    let data_ptr = unsafe { std::alloc::alloc(data_layout) };

    unsafe {
        core::ptr::copy_nonoverlapping(string.as_ptr(), data_ptr, string.len());
    }

    let string_layout = Layout::new::<StringObject>();
    let string_ptr = unsafe { std::alloc::alloc(string_layout) };

    unsafe {
        *string_ptr.cast::<StringObject>() = StringObject {
            ptr: data_ptr,
            len: string.len(),
        };
    }

    constants.push(ConstantData {
        ptr: data_ptr,
        layout: data_layout,
    });
    constants.push(ConstantData {
        ptr: string_ptr,
        layout: string_layout,
    });

    string_ptr
}

#[derive(Debug)]
pub struct ConstantData {
    ptr: *mut u8,
    layout: Layout,
}

impl Drop for ConstantData {
    fn drop(&mut self) {
        unsafe {
            std::alloc::dealloc(self.ptr, self.layout);
        }
    }
}
