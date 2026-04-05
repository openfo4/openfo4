use openfo4_jit::{FunctionBuilder, Type, Value};

use crate::compiler::codegen::CodegenData;

#[derive(Debug)]
pub struct Span {
    ptr: *const u8,
    len: usize,
    span: Option<Value>,
}

impl Span {
    pub fn new(data: &CodegenData, class: &str, func: &str) -> Self {
        let name = format!("{class}.{func}");
        let ptr = data.resources.constants.alloc(name.as_bytes(), 1);

        Self {
            ptr,
            len: name.len(),
            span: None,
        }
    }

    pub fn enter(&mut self, data: &CodegenData, builder: &mut FunctionBuilder<'_>, ctx: Value) {
        assert!(self.span.is_none(), "span already entered");

        let ptr = builder.iconst(Type::Pointer, self.ptr as usize as i64);
        let len = builder.iconst(Type::Pointer, self.len as i64);

        let span = builder.call(data.runtime_funcs.enter_span, &[ctx, ptr, len])[0];
        self.span = Some(span);
    }

    pub fn exit(&mut self, data: &CodegenData, builder: &mut FunctionBuilder<'_>, ctx: Value) {
        let span = self.span.expect("span is not entered");

        builder.call(data.runtime_funcs.exit_span, &[ctx, span]);
    }
}
