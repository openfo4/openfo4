use crate::{FloatCompareOp, IntCompareOp, Type, Word};

pub fn iadd(ty: Type, lhs: Word, rhs: Word) -> Word {
    match ty {
        Type::I8 => {
            let lhs: i8 = lhs.try_into().unwrap();
            let rhs: i8 = rhs.try_into().unwrap();
            lhs.wrapping_add(rhs).into()
        }
        Type::I16 => {
            let lhs: i16 = lhs.try_into().unwrap();
            let rhs: i16 = rhs.try_into().unwrap();
            lhs.wrapping_add(rhs).into()
        }
        Type::I32 => {
            let lhs: i32 = lhs.try_into().unwrap();
            let rhs: i32 = rhs.try_into().unwrap();
            lhs.wrapping_add(rhs).into()
        }
        Type::I64 => {
            let lhs: i64 = lhs.try_into().unwrap();
            let rhs: i64 = rhs.try_into().unwrap();
            lhs.wrapping_add(rhs).into()
        }
        Type::Pointer => {
            let lhs: i64 = lhs.try_into().unwrap();
            let rhs: i64 = rhs.try_into().unwrap();
            lhs.wrapping_add(rhs).into()
        }
        Type::F32 | Type::F64 => unreachable!(),
    }
}

pub fn isub(ty: Type, lhs: Word, rhs: Word) -> Word {
    match ty {
        Type::I8 => {
            let lhs: i8 = lhs.try_into().unwrap();
            let rhs: i8 = rhs.try_into().unwrap();
            lhs.wrapping_add(rhs).into()
        }
        Type::I16 => {
            let lhs: i16 = lhs.try_into().unwrap();
            let rhs: i16 = rhs.try_into().unwrap();
            lhs.wrapping_add(rhs).into()
        }
        Type::I32 => {
            let lhs: i32 = lhs.try_into().unwrap();
            let rhs: i32 = rhs.try_into().unwrap();
            lhs.wrapping_add(rhs).into()
        }
        Type::I64 => {
            let lhs: i64 = lhs.try_into().unwrap();
            let rhs: i64 = rhs.try_into().unwrap();
            lhs.wrapping_add(rhs).into()
        }
        Type::Pointer => {
            let lhs: i64 = lhs.try_into().unwrap();
            let rhs: i64 = rhs.try_into().unwrap();
            lhs.wrapping_add(rhs).into()
        }
        Type::F32 | Type::F64 => unreachable!(),
    }
}

pub fn imul(ty: Type, lhs: Word, rhs: Word) -> Word {
    match ty {
        Type::I8 => {
            let lhs: i8 = lhs.try_into().unwrap();
            let rhs: i8 = rhs.try_into().unwrap();
            lhs.wrapping_mul(rhs).into()
        }
        Type::I16 => {
            let lhs: i16 = lhs.try_into().unwrap();
            let rhs: i16 = rhs.try_into().unwrap();
            lhs.wrapping_mul(rhs).into()
        }
        Type::I32 => {
            let lhs: i32 = lhs.try_into().unwrap();
            let rhs: i32 = rhs.try_into().unwrap();
            lhs.wrapping_mul(rhs).into()
        }
        Type::I64 => {
            let lhs: i64 = lhs.try_into().unwrap();
            let rhs: i64 = rhs.try_into().unwrap();
            lhs.wrapping_mul(rhs).into()
        }
        Type::Pointer | Type::F32 | Type::F64 => unreachable!(),
    }
}

pub fn sdiv(ty: Type, lhs: Word, rhs: Word) -> Word {
    match ty {
        Type::I8 => {
            let lhs: i8 = lhs.try_into().unwrap();
            let rhs: i8 = rhs.try_into().unwrap();
            lhs.wrapping_div(rhs).into()
        }
        Type::I16 => {
            let lhs: i16 = lhs.try_into().unwrap();
            let rhs: i16 = rhs.try_into().unwrap();
            lhs.wrapping_div(rhs).into()
        }
        Type::I32 => {
            let lhs: i32 = lhs.try_into().unwrap();
            let rhs: i32 = rhs.try_into().unwrap();
            lhs.wrapping_div(rhs).into()
        }
        Type::I64 => {
            let lhs: i64 = lhs.try_into().unwrap();
            let rhs: i64 = rhs.try_into().unwrap();
            lhs.wrapping_div(rhs).into()
        }
        Type::Pointer | Type::F32 | Type::F64 => unreachable!(),
    }
}

pub fn udiv(ty: Type, lhs: Word, rhs: Word) -> Word {
    match ty {
        Type::I8 => {
            let lhs: i8 = lhs.try_into().unwrap();
            let rhs: i8 = rhs.try_into().unwrap();
            let lhs = lhs.cast_unsigned();
            let rhs = rhs.cast_unsigned();
            lhs.wrapping_div(rhs).cast_signed().into()
        }
        Type::I16 => {
            let lhs: i16 = lhs.try_into().unwrap();
            let rhs: i16 = rhs.try_into().unwrap();
            let lhs = lhs.cast_unsigned();
            let rhs = rhs.cast_unsigned();
            lhs.wrapping_div(rhs).cast_signed().into()
        }
        Type::I32 => {
            let lhs: i32 = lhs.try_into().unwrap();
            let rhs: i32 = rhs.try_into().unwrap();
            let lhs = lhs.cast_unsigned();
            let rhs = rhs.cast_unsigned();
            lhs.wrapping_div(rhs).cast_signed().into()
        }
        Type::I64 => {
            let lhs: i64 = lhs.try_into().unwrap();
            let rhs: i64 = rhs.try_into().unwrap();
            let lhs = lhs.cast_unsigned();
            let rhs = rhs.cast_unsigned();
            lhs.wrapping_div(rhs).cast_signed().into()
        }
        Type::Pointer | Type::F32 | Type::F64 => unreachable!(),
    }
}

pub fn srem(ty: Type, lhs: Word, rhs: Word) -> Word {
    match ty {
        Type::I8 => {
            let lhs: i8 = lhs.try_into().unwrap();
            let rhs: i8 = rhs.try_into().unwrap();
            lhs.wrapping_rem(rhs).into()
        }
        Type::I16 => {
            let lhs: i16 = lhs.try_into().unwrap();
            let rhs: i16 = rhs.try_into().unwrap();
            lhs.wrapping_rem(rhs).into()
        }
        Type::I32 => {
            let lhs: i32 = lhs.try_into().unwrap();
            let rhs: i32 = rhs.try_into().unwrap();
            lhs.wrapping_rem(rhs).into()
        }
        Type::I64 => {
            let lhs: i64 = lhs.try_into().unwrap();
            let rhs: i64 = rhs.try_into().unwrap();
            lhs.wrapping_rem(rhs).into()
        }
        Type::Pointer | Type::F32 | Type::F64 => unreachable!(),
    }
}

pub fn urem(ty: Type, lhs: Word, rhs: Word) -> Word {
    match ty {
        Type::I8 => {
            let lhs: i8 = lhs.try_into().unwrap();
            let rhs: i8 = rhs.try_into().unwrap();
            let lhs = lhs.cast_unsigned();
            let rhs = rhs.cast_unsigned();
            lhs.wrapping_rem(rhs).cast_signed().into()
        }
        Type::I16 => {
            let lhs: i16 = lhs.try_into().unwrap();
            let rhs: i16 = rhs.try_into().unwrap();
            let lhs = lhs.cast_unsigned();
            let rhs = rhs.cast_unsigned();
            lhs.wrapping_rem(rhs).cast_signed().into()
        }
        Type::I32 => {
            let lhs: i32 = lhs.try_into().unwrap();
            let rhs: i32 = rhs.try_into().unwrap();
            let lhs = lhs.cast_unsigned();
            let rhs = rhs.cast_unsigned();
            lhs.wrapping_rem(rhs).cast_signed().into()
        }
        Type::I64 => {
            let lhs: i64 = lhs.try_into().unwrap();
            let rhs: i64 = rhs.try_into().unwrap();
            let lhs = lhs.cast_unsigned();
            let rhs = rhs.cast_unsigned();
            lhs.wrapping_rem(rhs).cast_signed().into()
        }
        Type::Pointer | Type::F32 | Type::F64 => unreachable!(),
    }
}

pub fn smin(ty: Type, lhs: Word, rhs: Word) -> Word {
    match ty {
        Type::I8 => {
            let lhs: i8 = lhs.try_into().unwrap();
            let rhs: i8 = rhs.try_into().unwrap();
            lhs.min(rhs).into()
        }
        Type::I16 => {
            let lhs: i16 = lhs.try_into().unwrap();
            let rhs: i16 = rhs.try_into().unwrap();
            lhs.min(rhs).into()
        }
        Type::I32 => {
            let lhs: i32 = lhs.try_into().unwrap();
            let rhs: i32 = rhs.try_into().unwrap();
            lhs.min(rhs).into()
        }
        Type::I64 => {
            let lhs: i64 = lhs.try_into().unwrap();
            let rhs: i64 = rhs.try_into().unwrap();
            lhs.min(rhs).into()
        }
        Type::Pointer | Type::F32 | Type::F64 => unreachable!(),
    }
}

pub fn umin(ty: Type, lhs: Word, rhs: Word) -> Word {
    match ty {
        Type::I8 => {
            let lhs: i8 = lhs.try_into().unwrap();
            let rhs: i8 = rhs.try_into().unwrap();
            let lhs = lhs.cast_unsigned();
            let rhs = rhs.cast_unsigned();
            lhs.min(rhs).cast_signed().into()
        }
        Type::I16 => {
            let lhs: i16 = lhs.try_into().unwrap();
            let rhs: i16 = rhs.try_into().unwrap();
            let lhs = lhs.cast_unsigned();
            let rhs = rhs.cast_unsigned();
            lhs.min(rhs).cast_signed().into()
        }
        Type::I32 => {
            let lhs: i32 = lhs.try_into().unwrap();
            let rhs: i32 = rhs.try_into().unwrap();
            let lhs = lhs.cast_unsigned();
            let rhs = rhs.cast_unsigned();
            lhs.min(rhs).cast_signed().into()
        }
        Type::I64 => {
            let lhs: i64 = lhs.try_into().unwrap();
            let rhs: i64 = rhs.try_into().unwrap();
            let lhs = lhs.cast_unsigned();
            let rhs = rhs.cast_unsigned();
            lhs.min(rhs).cast_signed().into()
        }
        Type::Pointer | Type::F32 | Type::F64 => unreachable!(),
    }
}

pub fn smax(ty: Type, lhs: Word, rhs: Word) -> Word {
    match ty {
        Type::I8 => {
            let lhs: i8 = lhs.try_into().unwrap();
            let rhs: i8 = rhs.try_into().unwrap();
            lhs.max(rhs).into()
        }
        Type::I16 => {
            let lhs: i16 = lhs.try_into().unwrap();
            let rhs: i16 = rhs.try_into().unwrap();
            lhs.max(rhs).into()
        }
        Type::I32 => {
            let lhs: i32 = lhs.try_into().unwrap();
            let rhs: i32 = rhs.try_into().unwrap();
            lhs.max(rhs).into()
        }
        Type::I64 => {
            let lhs: i64 = lhs.try_into().unwrap();
            let rhs: i64 = rhs.try_into().unwrap();
            lhs.max(rhs).into()
        }
        Type::Pointer | Type::F32 | Type::F64 => unreachable!(),
    }
}

pub fn umax(ty: Type, lhs: Word, rhs: Word) -> Word {
    match ty {
        Type::I8 => {
            let lhs: i8 = lhs.try_into().unwrap();
            let rhs: i8 = rhs.try_into().unwrap();
            let lhs = lhs.cast_unsigned();
            let rhs = rhs.cast_unsigned();
            lhs.max(rhs).cast_signed().into()
        }
        Type::I16 => {
            let lhs: i16 = lhs.try_into().unwrap();
            let rhs: i16 = rhs.try_into().unwrap();
            let lhs = lhs.cast_unsigned();
            let rhs = rhs.cast_unsigned();
            lhs.max(rhs).cast_signed().into()
        }
        Type::I32 => {
            let lhs: i32 = lhs.try_into().unwrap();
            let rhs: i32 = rhs.try_into().unwrap();
            let lhs = lhs.cast_unsigned();
            let rhs = rhs.cast_unsigned();
            lhs.max(rhs).cast_signed().into()
        }
        Type::I64 => {
            let lhs: i64 = lhs.try_into().unwrap();
            let rhs: i64 = rhs.try_into().unwrap();
            let lhs = lhs.cast_unsigned();
            let rhs = rhs.cast_unsigned();
            lhs.max(rhs).cast_signed().into()
        }
        Type::Pointer | Type::F32 | Type::F64 => unreachable!(),
    }
}

pub fn icmp(ty: Type, lhs: Word, rhs: Word, op: IntCompareOp) -> Word {
    let (overflow, carry, zero, signed) = match ty {
        Type::I8 => {
            let lhs: i8 = lhs.try_into().unwrap();
            let rhs: i8 = rhs.try_into().unwrap();
            let (res, of) = lhs.overflowing_sub(rhs);
            let (_, cf) = lhs.cast_unsigned().overflowing_sub(rhs.cast_unsigned());
            (of, cf, res == 0, res.is_negative())
        }
        Type::I16 => {
            let lhs: i16 = lhs.try_into().unwrap();
            let rhs: i16 = rhs.try_into().unwrap();
            let (res, of) = lhs.overflowing_sub(rhs);
            let (_, cf) = lhs.cast_unsigned().overflowing_sub(rhs.cast_unsigned());
            (of, cf, res == 0, res.is_negative())
        }
        Type::I32 => {
            let lhs: i32 = lhs.try_into().unwrap();
            let rhs: i32 = rhs.try_into().unwrap();
            let (res, of) = lhs.overflowing_sub(rhs);
            let (_, cf) = lhs.cast_unsigned().overflowing_sub(rhs.cast_unsigned());
            (of, cf, res == 0, res.is_negative())
        }
        Type::I64 => {
            let lhs: i64 = lhs.try_into().unwrap();
            let rhs: i64 = rhs.try_into().unwrap();
            let (res, of) = lhs.overflowing_sub(rhs);
            let (_, cf) = lhs.cast_unsigned().overflowing_sub(rhs.cast_unsigned());
            (of, cf, res == 0, res.is_negative())
        }
        Type::Pointer => {
            let lhs: isize = lhs.try_into().unwrap();
            let rhs: isize = rhs.try_into().unwrap();
            let (res, of) = lhs.overflowing_sub(rhs);
            let (_, cf) = lhs.cast_unsigned().overflowing_sub(rhs.cast_unsigned());
            (of, cf, res == 0, res.is_negative())
        }
        Type::F32 | Type::F64 => unreachable!(),
    };

    // Based on http://unixwiz.net/techtips/x86-jumps.html
    Word::from(i8::from(match op {
        IntCompareOp::Equal => zero,
        IntCompareOp::NotEqual => !zero,
        IntCompareOp::SignedLessThan => signed != overflow,
        IntCompareOp::SignedLessThanOrEqual => zero || signed != overflow,
        IntCompareOp::SignedGreaterThan => !zero && signed == overflow,
        IntCompareOp::SignedGreaterThanOrEqual => signed == overflow,
        IntCompareOp::UnsignedLessThan => carry,
        IntCompareOp::UnsignedLessThanOrEqual => carry || zero,
        IntCompareOp::UnsignedGreaterThan => !carry && !zero,
        IntCompareOp::UnsignedGreaterThanOrEqual => !carry,
    }))
}

pub fn fcmp(ty: Type, lhs: Word, rhs: Word, op: FloatCompareOp) -> Word {
    let diff = match ty {
        Type::F32 => {
            let lhs: f32 = lhs.try_into().unwrap();
            let rhs: f32 = rhs.try_into().unwrap();
            (lhs - rhs).signum()
        }
        Type::F64 => {
            let lhs: f64 = lhs.try_into().unwrap();
            let rhs: f64 = rhs.try_into().unwrap();
            (lhs - rhs).signum() as f32
        }
        _ => unreachable!(),
    };

    Word::from(i8::from(match op {
        FloatCompareOp::Equal => diff == 0.0,
        FloatCompareOp::NotEqual => diff != 0.0 || diff.is_nan(),
        FloatCompareOp::LessThan => !diff.is_nan() && diff < 0.0,
        FloatCompareOp::LessThanOrEqual => !diff.is_nan() && diff <= 0.0,
        FloatCompareOp::GreaterThan => !diff.is_nan() && diff > 0.0,
        FloatCompareOp::GreaterThanOrEqual => !diff.is_nan() && diff >= 0.0,
    }))
}

pub fn fadd(ty: Type, lhs: Word, rhs: Word) -> Word {
    match ty {
        Type::F32 => {
            let lhs: f32 = lhs.try_into().unwrap();
            let rhs: f32 = rhs.try_into().unwrap();
            (lhs + rhs).into()
        }
        Type::F64 => {
            let lhs: f64 = lhs.try_into().unwrap();
            let rhs: f64 = rhs.try_into().unwrap();
            (lhs + rhs).into()
        }
        _ => unreachable!(),
    }
}

pub fn fsub(ty: Type, lhs: Word, rhs: Word) -> Word {
    match ty {
        Type::F32 => {
            let lhs: f32 = lhs.try_into().unwrap();
            let rhs: f32 = rhs.try_into().unwrap();
            (lhs - rhs).into()
        }
        Type::F64 => {
            let lhs: f64 = lhs.try_into().unwrap();
            let rhs: f64 = rhs.try_into().unwrap();
            (lhs - rhs).into()
        }
        _ => unreachable!(),
    }
}

pub fn fmul(ty: Type, lhs: Word, rhs: Word) -> Word {
    match ty {
        Type::F32 => {
            let lhs: f32 = lhs.try_into().unwrap();
            let rhs: f32 = rhs.try_into().unwrap();
            (lhs * rhs).into()
        }
        Type::F64 => {
            let lhs: f64 = lhs.try_into().unwrap();
            let rhs: f64 = rhs.try_into().unwrap();
            (lhs * rhs).into()
        }
        _ => unreachable!(),
    }
}

pub fn fdiv(ty: Type, lhs: Word, rhs: Word) -> Word {
    match ty {
        Type::F32 => {
            let lhs: f32 = lhs.try_into().unwrap();
            let rhs: f32 = rhs.try_into().unwrap();
            (lhs / rhs).into()
        }
        Type::F64 => {
            let lhs: f64 = lhs.try_into().unwrap();
            let rhs: f64 = rhs.try_into().unwrap();
            (lhs / rhs).into()
        }
        _ => unreachable!(),
    }
}

pub fn fmin(ty: Type, lhs: Word, rhs: Word) -> Word {
    match ty {
        Type::F32 => {
            let lhs: f32 = lhs.try_into().unwrap();
            let rhs: f32 = rhs.try_into().unwrap();
            lhs.min(rhs).into()
        }
        Type::F64 => {
            let lhs: f64 = lhs.try_into().unwrap();
            let rhs: f64 = rhs.try_into().unwrap();
            lhs.min(rhs).into()
        }
        _ => unreachable!(),
    }
}

pub fn fmax(ty: Type, lhs: Word, rhs: Word) -> Word {
    match ty {
        Type::F32 => {
            let lhs: f32 = lhs.try_into().unwrap();
            let rhs: f32 = rhs.try_into().unwrap();
            lhs.max(rhs).into()
        }
        Type::F64 => {
            let lhs: f64 = lhs.try_into().unwrap();
            let rhs: f64 = rhs.try_into().unwrap();
            lhs.max(rhs).into()
        }
        _ => unreachable!(),
    }
}

pub fn ineg(ty: Type, src: Word) -> Word {
    match ty {
        Type::I8 => {
            let src: i8 = src.try_into().unwrap();
            src.wrapping_neg().into()
        }
        Type::I16 => {
            let src: i16 = src.try_into().unwrap();
            src.wrapping_neg().into()
        }
        Type::I32 => {
            let src: i32 = src.try_into().unwrap();
            src.wrapping_neg().into()
        }
        Type::I64 => {
            let src: i64 = src.try_into().unwrap();
            src.wrapping_neg().into()
        }
        _ => unreachable!(),
    }
}

pub fn fneg(ty: Type, src: Word) -> Word {
    match ty {
        Type::F32 => {
            let src: f32 = src.try_into().unwrap();
            (-src).into()
        }
        Type::F64 => {
            let src: f64 = src.try_into().unwrap();
            (-src).into()
        }
        _ => unreachable!(),
    }
}

pub fn fabs(ty: Type, src: Word) -> Word {
    match ty {
        Type::F32 => {
            let src: f32 = src.try_into().unwrap();
            src.abs().into()
        }
        Type::F64 => {
            let src: f64 = src.try_into().unwrap();
            src.abs().into()
        }
        _ => unreachable!(),
    }
}

pub fn ceil(ty: Type, src: Word) -> Word {
    match ty {
        Type::F32 => {
            let src: f32 = src.try_into().unwrap();
            src.ceil().into()
        }
        Type::F64 => {
            let src: f64 = src.try_into().unwrap();
            src.ceil().into()
        }
        _ => unreachable!(),
    }
}

pub fn floor(ty: Type, src: Word) -> Word {
    match ty {
        Type::F32 => {
            let src: f32 = src.try_into().unwrap();
            src.floor().into()
        }
        Type::F64 => {
            let src: f64 = src.try_into().unwrap();
            src.floor().into()
        }
        _ => unreachable!(),
    }
}

pub fn band(ty: Type, lhs: Word, rhs: Word) -> Word {
    match ty {
        Type::I8 => {
            let lhs: i8 = lhs.try_into().unwrap();
            let rhs: i8 = rhs.try_into().unwrap();
            (lhs & rhs).into()
        }
        Type::I16 => {
            let lhs: i16 = lhs.try_into().unwrap();
            let rhs: i16 = rhs.try_into().unwrap();
            (lhs & rhs).into()
        }
        Type::I32 => {
            let lhs: i32 = lhs.try_into().unwrap();
            let rhs: i32 = rhs.try_into().unwrap();
            (lhs & rhs).into()
        }
        Type::I64 => {
            let lhs: i64 = lhs.try_into().unwrap();
            let rhs: i64 = rhs.try_into().unwrap();
            (lhs & rhs).into()
        }
        _ => unreachable!(),
    }
}

pub fn bor(ty: Type, lhs: Word, rhs: Word) -> Word {
    match ty {
        Type::I8 => {
            let lhs: i8 = lhs.try_into().unwrap();
            let rhs: i8 = rhs.try_into().unwrap();
            (lhs | rhs).into()
        }
        Type::I16 => {
            let lhs: i16 = lhs.try_into().unwrap();
            let rhs: i16 = rhs.try_into().unwrap();
            (lhs | rhs).into()
        }
        Type::I32 => {
            let lhs: i32 = lhs.try_into().unwrap();
            let rhs: i32 = rhs.try_into().unwrap();
            (lhs | rhs).into()
        }
        Type::I64 => {
            let lhs: i64 = lhs.try_into().unwrap();
            let rhs: i64 = rhs.try_into().unwrap();
            (lhs | rhs).into()
        }
        _ => unreachable!(),
    }
}

pub fn bxor(ty: Type, lhs: Word, rhs: Word) -> Word {
    match ty {
        Type::I8 => {
            let lhs: i8 = lhs.try_into().unwrap();
            let rhs: i8 = rhs.try_into().unwrap();
            (lhs ^ rhs).into()
        }
        Type::I16 => {
            let lhs: i16 = lhs.try_into().unwrap();
            let rhs: i16 = rhs.try_into().unwrap();
            (lhs ^ rhs).into()
        }
        Type::I32 => {
            let lhs: i32 = lhs.try_into().unwrap();
            let rhs: i32 = rhs.try_into().unwrap();
            (lhs ^ rhs).into()
        }
        Type::I64 => {
            let lhs: i64 = lhs.try_into().unwrap();
            let rhs: i64 = rhs.try_into().unwrap();
            (lhs ^ rhs).into()
        }
        _ => unreachable!(),
    }
}

pub fn bnot(ty: Type, src: Word) -> Word {
    match ty {
        Type::I8 => {
            let src: i8 = src.try_into().unwrap();
            (!src).into()
        }
        Type::I16 => {
            let src: i16 = src.try_into().unwrap();
            (!src).into()
        }
        Type::I32 => {
            let src: i32 = src.try_into().unwrap();
            (!src).into()
        }
        Type::I64 => {
            let src: i64 = src.try_into().unwrap();
            (!src).into()
        }
        _ => unreachable!(),
    }
}

pub fn popcnt(ty: Type, src: Word) -> Word {
    let count = match ty {
        Type::I8 => i8::try_from(src).unwrap().count_ones(),
        Type::I16 => i16::try_from(src).unwrap().count_ones(),
        Type::I32 => i32::try_from(src).unwrap().count_ones(),
        Type::I64 => i64::try_from(src).unwrap().count_ones(),
        _ => unreachable!(),
    };

    Word::from(count.cast_signed())
}

pub fn ftos(src_ty: Type, src: Word, dst_ty: Type) -> Word {
    match src_ty {
        Type::F32 => {
            let src: f32 = src.try_into().unwrap();

            match dst_ty {
                Type::I8 => (src as i8).into(),
                Type::I16 => (src as i16).into(),
                Type::I32 => (src as i32).into(),
                Type::I64 => (src as i64).into(),
                _ => unreachable!(),
            }
        }
        Type::F64 => {
            let src: f64 = src.try_into().unwrap();

            match dst_ty {
                Type::I8 => (src as i8).into(),
                Type::I16 => (src as i16).into(),
                Type::I32 => (src as i32).into(),
                Type::I64 => (src as i64).into(),
                _ => unreachable!(),
            }
        }
        _ => unreachable!(),
    }
}

pub fn ftou(src_ty: Type, src: Word, dst_ty: Type) -> Word {
    match src_ty {
        Type::F32 => {
            let src: f32 = src.try_into().unwrap();

            match dst_ty {
                Type::I8 => (src as u8).cast_signed().into(),
                Type::I16 => (src as u16).cast_signed().into(),
                Type::I32 => (src as u32).cast_signed().into(),
                Type::I64 => (src as u64).cast_signed().into(),
                _ => unreachable!(),
            }
        }
        Type::F64 => {
            let src: f64 = src.try_into().unwrap();

            match dst_ty {
                Type::I8 => (src as u8).cast_signed().into(),
                Type::I16 => (src as u16).cast_signed().into(),
                Type::I32 => (src as u32).cast_signed().into(),
                Type::I64 => (src as u64).cast_signed().into(),
                _ => unreachable!(),
            }
        }
        _ => unreachable!(),
    }
}

pub fn stof(src_ty: Type, src: Word, dst_ty: Type) -> Word {
    let src: i64 = match src_ty {
        Type::I8 => {
            let src: i8 = src.try_into().unwrap();
            src.into()
        }
        Type::I16 => {
            let src: i16 = src.try_into().unwrap();
            src.into()
        }
        Type::I32 => {
            let src: i32 = src.try_into().unwrap();
            src.into()
        }
        Type::I64 => {
            let src: i64 = src.try_into().unwrap();
            src
        }
        _ => unreachable!(),
    };

    match dst_ty {
        Type::F32 => (src as f32).into(),
        Type::F64 => (src as f64).into(),
        _ => unreachable!(),
    }
}

pub fn utof(src_ty: Type, src: Word, dst_ty: Type) -> Word {
    let src: i64 = match src_ty {
        Type::I8 => {
            let src: i8 = src.try_into().unwrap();
            src.into()
        }
        Type::I16 => {
            let src: i16 = src.try_into().unwrap();
            src.into()
        }
        Type::I32 => {
            let src: i32 = src.try_into().unwrap();
            src.into()
        }
        Type::I64 => {
            let src: i64 = src.try_into().unwrap();
            src
        }
        _ => unreachable!(),
    };

    let src: u64 = src.cast_unsigned();

    match dst_ty {
        Type::F32 => (src as f32).into(),
        Type::F64 => (src as f64).into(),
        _ => unreachable!(),
    }
}

pub fn sextend(src_ty: Type, src: Word, dst_ty: Type) -> Word {
    let src: i64 = match src_ty {
        Type::I8 => i8::try_from(src).unwrap().into(),
        Type::I16 => i16::try_from(src).unwrap().into(),
        Type::I32 => i32::try_from(src).unwrap().into(),
        Type::I64 => i64::try_from(src).unwrap().into(),
        _ => unreachable!(),
    };

    match dst_ty {
        Type::I8 => (src as i8).into(),
        Type::I16 => (src as i16).into(),
        Type::I32 => (src as i32).into(),
        Type::I64 => (src as i64).into(),
        _ => unreachable!(),
    }
}

pub fn uextend(src_ty: Type, src: Word, dst_ty: Type) -> Word {
    let src: i64 = match src_ty {
        Type::I8 => i8::try_from(src).unwrap().into(),
        Type::I16 => i16::try_from(src).unwrap().into(),
        Type::I32 => i32::try_from(src).unwrap().into(),
        Type::I64 => i64::try_from(src).unwrap().into(),
        _ => unreachable!(),
    };
    let src: u64 = src.cast_unsigned();

    match dst_ty {
        Type::I8 => (src as u8).cast_signed().into(),
        Type::I16 => (src as u16).cast_signed().into(),
        Type::I32 => (src as u32).cast_signed().into(),
        Type::I64 => (src as u64).cast_signed().into(),
        _ => unreachable!(),
    }
}

pub fn ireduce(src_ty: Type, src: Word, dst_ty: Type) -> Word {
    let src: i64 = match src_ty {
        Type::I8 => i8::try_from(src).unwrap().into(),
        Type::I16 => i16::try_from(src).unwrap().into(),
        Type::I32 => i32::try_from(src).unwrap().into(),
        Type::I64 => i64::try_from(src).unwrap().into(),
        _ => unreachable!(),
    };

    match dst_ty {
        Type::I8 => (src as i8).into(),
        Type::I16 => (src as i16).into(),
        Type::I32 => (src as i32).into(),
        Type::I64 => (src as i64).into(),
        _ => unreachable!(),
    }
}

pub unsafe fn read(ty: Type, ptr: *const u8) -> Word {
    match ty {
        Type::I8 => {
            let value = unsafe { ptr.cast::<i8>().read() };
            Word::from(value)
        }
        Type::I16 => {
            let value = unsafe { ptr.cast::<i16>().read() };
            Word::from(value)
        }
        Type::I32 => {
            let value = unsafe { ptr.cast::<i32>().read() };
            Word::from(value)
        }
        Type::I64 => {
            let value = unsafe { ptr.cast::<i64>().read() };
            Word::from(value)
        }
        Type::Pointer => {
            let value = unsafe { ptr.cast::<*mut ()>().read() };
            Word(value)
        }
        Type::F32 => {
            let value = unsafe { ptr.cast::<f32>().read() };
            Word::from(value)
        }
        Type::F64 => {
            let value = unsafe { ptr.cast::<f64>().read() };
            Word::from(value)
        }
    }
}

pub unsafe fn write(ty: Type, value: Word, ptr: *mut u8) {
    match ty {
        Type::I8 => {
            let value: i8 = value.try_into().unwrap();
            unsafe {
                ptr.cast::<i8>().write(value);
            }
        }
        Type::I16 => {
            let value: i16 = value.try_into().unwrap();
            unsafe {
                ptr.cast::<i16>().write(value);
            }
        }
        Type::I32 => {
            let value: i32 = value.try_into().unwrap();
            unsafe {
                ptr.cast::<i32>().write(value);
            }
        }
        Type::I64 => {
            let value: i64 = value.try_into().unwrap();
            unsafe {
                ptr.cast::<i64>().write(value);
            }
        }
        Type::Pointer => {
            let value: *mut () = value.0;
            unsafe {
                ptr.cast::<*mut ()>().write(value);
            }
        }
        Type::F32 => {
            let value: f32 = value.try_into().unwrap();
            unsafe {
                ptr.cast::<f32>().write(value);
            }
        }
        Type::F64 => {
            let value: f64 = value.try_into().unwrap();
            unsafe {
                ptr.cast::<f64>().write(value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{IntCompareOp, Type};

    use super::icmp;

    #[test]
    fn test_icmp() {
        fn icmp_i8(lhs: i8, rhs: i8, op: IntCompareOp) -> i8 {
            icmp(Type::I8, lhs.into(), rhs.into(), op)
                .try_into()
                .unwrap()
        }

        fn icmp_u8(lhs: u8, rhs: u8, op: IntCompareOp) -> i8 {
            icmp_i8(lhs.cast_signed(), rhs.cast_signed(), op)
        }

        assert_eq!(icmp_i8(25, 25, IntCompareOp::Equal), 1);
        assert_eq!(icmp_i8(25, 25, IntCompareOp::NotEqual), 0);

        assert_eq!(icmp_i8(25, 50, IntCompareOp::SignedLessThan), 1);
        assert_eq!(icmp_i8(25, 50, IntCompareOp::SignedGreaterThan), 0);

        assert_eq!(icmp_i8(-25, -50, IntCompareOp::SignedGreaterThan), 1);
        assert_eq!(icmp_i8(-25, -50, IntCompareOp::SignedLessThan), 0);

        assert_eq!(icmp_u8(150, 200, IntCompareOp::UnsignedLessThan), 1);
        assert_eq!(icmp_u8(150, 200, IntCompareOp::UnsignedGreaterThan), 0);

        assert_eq!(icmp_u8(200, 0, IntCompareOp::UnsignedGreaterThan), 1);
        assert_eq!(icmp_u8(200, 0, IntCompareOp::UnsignedLessThan), 0);
    }
}
