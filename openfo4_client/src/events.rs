pub mod animation;
pub mod combat;
pub mod inventory;
pub mod quest;
pub mod trigger;

use crate::cell::CellId;

#[derive(Clone, Debug)]
pub enum Event {
    UnloadCell(UnloadCell),
}

#[derive(Copy, Clone, Debug)]
pub struct UnloadCell {
    pub cell: CellId,
}
