pub mod inventory;

use crate::cell::CellId;

#[derive(Clone, Debug)]
pub enum Event {
    UnloadCell(UnloadCell),
}

#[derive(Copy, Clone, Debug)]
pub struct UnloadCell {
    pub cell: CellId,
}
