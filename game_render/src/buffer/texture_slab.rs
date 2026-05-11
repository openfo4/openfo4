//! Texture Slab
//!
//! Bindless-style pipelines usually need to be able to index into an array of textures from the
//! shader. This requires a stable index to the textures for the duration of the shader invocation.
//!
//! These indices are usually inserted into other buffers and as such can't easily be updated after
//! the texture was first created, making modification of the texture set difficult.
//!
//! [`TextureSlab`] is the solution to this problem: It provides a heap of textures with stable
//! addresses while supporting dynamic modification.

use bytemuck::{Pod, Zeroable};

use crate::api::TextureView;

/// Index into [`TextureSlab`].
///
/// This type guarantees to have the same layout as [`u32`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
#[repr(transparent)]
pub struct TextureSlabIndex(pub u32);

#[derive(Debug)]
pub struct TextureSlab {
    default: TextureView,
    entries: Vec<Option<TextureView>>,
    free_list: Vec<u32>,
    len: usize,
}

impl TextureSlab {
    pub fn new(default: TextureView) -> Self {
        Self {
            default,
            entries: Vec::new(),
            free_list: Vec::new(),
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn insert(&mut self, texture: TextureView) -> TextureSlabIndex {
        let index = match self.free_list.pop() {
            Some(index) => index as usize,
            None => {
                let index = self.entries.len();
                self.entries.resize(index + 1, None);
                index
            }
        };

        let slot = &mut self.entries[index];
        debug_assert!(slot.is_none());
        *slot = Some(texture);

        self.len += 1;
        TextureSlabIndex(index as u32)
    }

    pub fn remove(&mut self, index: TextureSlabIndex) {
        let entry = &mut self.entries[index.0 as usize];
        debug_assert!(entry.is_some());
        *entry = None;

        self.len -= 1;
        self.free_list.push(index.0);
        // TODO: We may want to compact the entries when the last
        // entry is removed.
        // This will require to iteratively pop entries from the
        // back of the entries Vec AND remove the index from
        // the free list.
    }

    pub fn views(&self) -> Vec<&TextureView> {
        self.entries
            .iter()
            .map(|view| view.as_ref().unwrap_or(&self.default))
            .collect()
    }
}
