//! Compiler builtin functions

use game_common::string::Lowercase;
use openfo4_jit::{Type as JitType, Value as JitValue};

use crate::compiler::codegen::function::FunctionTranslator;

pub fn call_builtin(
    translator: &mut FunctionTranslator<'_>,
    class: &Lowercase<str>,
    func: &Lowercase<str>,
    args: &[JitValue],
) -> Option<Vec<JitValue>> {
    match (class.as_inner(), func.as_inner()) {
        // Math Script
        // Math.abs(float) -> float
        ("math", "abs") => {
            let src = args[0];
            let res = translator.builder.fabs(src);
            Some(vec![res])
        }
        // Math.Floor(float) -> float
        ("math", "floor") => {
            let src = args[0];
            let res = translator.builder.floor(src);
            let res = translator.builder.convert_float_to_sint(res, JitType::I32);
            Some(vec![res])
        }
        // Math.Ceiling(float) -> float
        ("math", "ceiling") => {
            let src = args[0];
            let res = translator.builder.ceil(src);
            let res = translator.builder.convert_float_to_sint(res, JitType::I32);
            Some(vec![res])
        }
        // Math.DegreesToRadians(float) -> float
        ("math", "degreestoradians") => {
            // See f32::to_radians
            const RADS_PER_DEG: f32 = core::f32::consts::PI / 180.0;

            let lhs = args[0];
            let rhs = translator.builder.fconst(JitType::F32, RADS_PER_DEG);
            let res = translator.builder.fmul(lhs, rhs);
            Some(vec![res])
        }
        // Math.RadiansToDegrees(float) -> float
        ("math", "radianstodegrees") => {
            // See f32::to_degrees
            const PIS_IN_180: f32 = 57.2957795130823208767981548141051703_f32;

            let lhs = args[0];
            let rhs = translator.builder.fconst(JitType::F32, PIS_IN_180);
            let res = translator.builder.fmul(lhs, rhs);
            Some(vec![res])
        }
        // Form Script
        // Not implemented by us.
        ("form", "startobjectprofiling") => Some(Vec::new()),
        // Not implemented by us.
        ("form", "stopobjectprofiling") => Some(Vec::new()),
        _ => None,
    }
}
