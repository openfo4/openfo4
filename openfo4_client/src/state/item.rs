use openfo4_esp::FormId;

use crate::state::esp::EspLoader;

#[derive(Clone, Debug)]
pub struct Item {
    pub form: FormId,
    pub attachments: Vec<Attachment>,
}

#[derive(Clone, Debug)]
pub struct Attachment {}
