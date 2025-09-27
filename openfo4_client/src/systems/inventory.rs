use openfo4_esp::FormId;
use slab::Slab;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct InventoryKey(usize);

#[derive(Clone, Debug, Default)]
pub struct Inventory {
    pub items: Slab<ItemStack>,
}

#[derive(Clone, Debug)]
pub struct ItemStack {
    pub item: FormId,
    pub count: u32,
    pub attachments: Vec<InventoryKey>,
}
