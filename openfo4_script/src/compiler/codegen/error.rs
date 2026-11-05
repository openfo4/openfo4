use std::fmt::Display;

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

impl Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CallMethodOnNull { callee } => {
                write!(f, "cannot call method `{callee}` on null object")
            }
            Self::NullArray => write!(f, "cannot access null array"),
            Self::ArrayAddWithNegativeCount => write!(f, "array add with negative count"),
            Self::NullVar => write!(f, "cannot access null var"),
            Self::WrongVarType => write!(f, "wrong var type"),
            Self::NullStruct => write!(f, "cannot access null struct"),
            Self::NullClass => write!(f, "cannot access null object"),
            Self::ArrayOutOfBounds => write!(f, "array access is out of bounds"),
        }
    }
}
