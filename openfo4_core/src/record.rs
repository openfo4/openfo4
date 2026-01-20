use std::hash::{Hash, Hasher};
use std::num::NonZeroU32;

use openfo4_esp::FormId;

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

impl std::fmt::Debug for RecordId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecordId")
            .field("module", &self.module)
            .field("form", &FormId::new(0, self.form.form()))
            .finish()
    }
}
