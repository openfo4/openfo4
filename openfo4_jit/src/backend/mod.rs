use std::range::Range;

use slab::Slab;

use crate::{Instruction, SourceLoc, Value};

#[cfg(feature = "cranelift")]
pub mod cranelift;

#[derive(Clone, Debug)]
pub struct FunctionDefinitionDesc {
    pub instructions: Vec<Instruction>,
    pub num_values: usize,
    pub num_blocks: usize,
    pub num_variables: usize,
    pub block_params: Vec<Vec<Value>>,
    pub stack_slots: Slab<StackSlotDesc>,
    pub srcloc: SourceLocMap,
}

#[derive(Copy, Clone, Debug)]
pub struct StackSlotDesc {
    pub size: usize,
    pub align: usize,
}

#[derive(Clone, Debug, Default)]
pub struct SourceLocMap {
    // FIXME: We can use a range map like thing here.
    locs: Vec<Option<SourceLoc>>,
}

impl SourceLocMap {
    pub fn insert(&mut self, start: usize, end: usize, loc: SourceLoc) {
        self.locs.resize(end, None);
        for index in start..end {
            self.locs[index] = Some(loc);
        }
    }

    pub fn get(&self, index: usize) -> Option<SourceLoc> {
        self.locs.get(index).copied().flatten()
    }

    pub fn iter(&self) -> SourceLocMapIter<'_> {
        SourceLocMapIter {
            index: 0,
            map: self,
        }
    }
}

impl<'a> IntoIterator for &'a SourceLocMap {
    type Item = <SourceLocMapIter<'a> as Iterator>::Item;
    type IntoIter = SourceLocMapIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Clone, Debug)]
pub struct SourceLocMapIter<'a> {
    index: usize,
    map: &'a SourceLocMap,
}

impl<'a> Iterator for SourceLocMapIter<'a> {
    type Item = (Range<usize>, SourceLoc);

    fn next(&mut self) -> Option<Self::Item> {
        let mut start = 0;
        let mut loc = None;
        for (i, l) in self.map.locs[self.index..].iter().enumerate() {
            loc = *l;
            start = i;
            if loc.is_some() {
                break;
            }
        }

        start = self.index + start;
        let loc = loc?;

        for (i, l) in self.map.locs[start..].iter().enumerate() {
            if *l != Some(loc) {
                let end = start + i;
                self.index = end;
                return Some((Range { start, end }, loc));
            }
        }

        self.index = self.map.locs.len();
        Some((
            Range {
                start,
                end: self.map.locs.len(),
            },
            loc,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct TextWithSrcLocs<T> {
    pub text: T,
    pub srclocs: SourceLocMap,
}
