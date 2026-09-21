#[derive(Clone, Debug)]
pub enum ErrorKind {
    CallMethodOnNull { callee: String },
    NullArray,
    ArrayAddWithNegativeCount,
    NullVar,
    WrongVarType,
    NullStruct,
    NullClass,
    ArrayOutOfBounds,
}

impl ErrorKind {
    pub fn to_str_leak(&self) -> &'static str {
        match self {
            Self::CallMethodOnNull { callee } => {
                let msg = format!("cannot call method `{callee}` on null object");
                String::leak(msg)
            }
            Self::NullArray => "cannot access null array",
            Self::ArrayAddWithNegativeCount => "array add with negative count",
            Self::NullVar => "cannot access null var",
            Self::WrongVarType => "wrong var type",
            Self::NullStruct => "cannot access null struct",
            Self::NullClass => "cannot access null object",
            Self::ArrayOutOfBounds => "array access is out of bounds",
        }
    }
}
