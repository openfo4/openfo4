use std::fmt::{self, Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::num::{NonZeroU32, ParseIntError};
use std::str::FromStr;

use openfo4_esp::FormId;
use thiserror::Error;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ModuleId(pub NonZeroU32);

#[derive(Copy, Clone)]
pub struct RecordId {
    pub module: ModuleId,
    // Note that only the Form part is relevant, the upper plugin bits
    // are to be ignored. For this reason we have manual Debug, Eq and Hash impls.
    pub form: FormId,
}

impl RecordId {
    pub const DANGLING: Self = Self {
        module: ModuleId(NonZeroU32::MAX),
        form: FormId::NULL,
    };

    pub const fn new(module: ModuleId, form: FormId) -> Self {
        Self { module, form }
    }
}

impl PartialEq for RecordId {
    fn eq(&self, other: &Self) -> bool {
        self.module == other.module && self.form.form() == other.form.form()
    }
}

impl Eq for RecordId {}

impl Hash for RecordId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.module.hash(state);
        self.form.form().hash(state);
    }
}

impl Display for RecordId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:x}", self.module.0.get())?;
        write!(f, "{:06x}", self.form.form())?;
        Ok(())
    }
}

#[derive(Clone, Debug, Error)]
pub enum ParseRecordIdError {
    #[error("invalid char: {0}")]
    InvalidChar(ParseIntError),
    #[error("zero module")]
    ZeroModule,
}

impl FromStr for RecordId {
    type Err = ParseRecordIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let module_bytes = s.len().saturating_sub(6);

        let module =
            u32::from_str_radix(&s[..module_bytes], 16).map_err(ParseRecordIdError::InvalidChar)?;
        let form =
            u32::from_str_radix(&s[module_bytes..], 16).map_err(ParseRecordIdError::InvalidChar)?;

        let module = NonZeroU32::new(module).ok_or(ParseRecordIdError::ZeroModule)?;

        Ok(Self {
            module: ModuleId(module),
            form: FormId(form),
        })
    }
}

impl Debug for RecordId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("RecordId")
            .field("module", &self.module)
            .field("form", &FormId::new(0, self.form.form()))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use openfo4_esp::FormId;

    use crate::record::{ModuleId, RecordId};

    #[test]
    fn parse_record_id() {
        assert_eq!(
            "12345678".parse::<RecordId>().unwrap(),
            RecordId {
                module: ModuleId(NonZeroU32::new(0x12).unwrap()),
                form: FormId(0x345678),
            }
        );
    }

    #[test]
    fn parse_record_id_empty() {
        assert!("".parse::<RecordId>().is_err());
    }

    #[test]
    fn parse_record_id_missing_module() {
        assert!("123456".parse::<RecordId>().is_err());
    }

    #[test]
    fn record_id_to_str_from_str_identity() {
        let r = RecordId {
            module: ModuleId(NonZeroU32::new(101).unwrap()),
            form: FormId(0x2345),
        };

        assert_eq!(r.to_string().parse::<RecordId>().unwrap(), r);
    }
}
