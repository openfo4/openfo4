use std::num::NonZeroU32;

use openfo4_esp::FormId;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ModuleId(pub NonZeroU32);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RecordId {
    pub module: ModuleId,
    pub form: FormId,
}

impl RecordId {
    pub const fn new(module: ModuleId, form: FormId) -> Self {
        Self { module, form }
    }
}
