use std::collections::VecDeque;
use std::iter::FusedIterator;

use game_common::collections::vec_map::{Index, VecMap};
use game_common::utils::vec_ext::VecExt;
use hashbrown::HashMap;
use openfo4_pex::{Function, Instruction};

/// A CFG of a PEX function.
#[derive(Clone, Debug)]
pub struct ControlFlowGraph<'a> {
    blocks: Vec<&'a [Instruction]>,
    succ: VecMap<BlockId, Vec<BlockId>>,
}

impl<'a> ControlFlowGraph<'a> {
    /// Generates the CFG from the given [`Function`].
    pub fn new(func: &'a Function) -> Self {
        let mut blocks = Vec::<&'a [Instruction]>::new();
        let mut queue = VecDeque::new();
        let mut targets = HashMap::new();

        let mut succ = VecMap::<_, Vec<_>>::new();

        let entry_block = BlockId(blocks.push_and_index(&[]));
        queue.push_back((0, entry_block));
        targets.insert(0, entry_block);

        while let Some((start_addr, cur_block)) = queue.pop_front() {
            debug_assert!(targets.contains_key(&start_addr));

            for (index, ins) in func.instructions[start_addr..].iter().enumerate() {
                let index = start_addr + index;

                let seal_cur_block = match ins {
                    Instruction::Branch(op) => {
                        let addr = usize::min(
                            (index as i32 + op.offset) as usize,
                            func.instructions.len(),
                        );

                        let id = *targets.entry(addr).or_insert_with(|| {
                            let index = BlockId(blocks.push_and_index(&[]));
                            queue.push_back((addr, index));
                            index
                        });
                        succ.entry(cur_block).or_default().push(id);

                        true
                    }
                    Instruction::BranchIfFalse(op) | Instruction::BranchIfTrue(op) => {
                        let addr = usize::min(
                            (index as i32 + op.offset) as usize,
                            func.instructions.len(),
                        );
                        let next_addr = index + 1;

                        {
                            let id = *targets.entry(addr).or_insert_with(|| {
                                let index = BlockId(blocks.push_and_index(&[]));
                                queue.push_back((addr, index));
                                index
                            });
                            succ.entry(cur_block).or_default().push(id);
                        }

                        {
                            let id = *targets.entry(next_addr).or_insert_with(|| {
                                let index = BlockId(blocks.push_and_index(&[]));
                                queue.push_back((next_addr, index));
                                index
                            });
                            succ.entry(cur_block).or_default().push(id);
                        }

                        true
                    }
                    Instruction::Return(_) => true,
                    _ => false,
                };

                if seal_cur_block || index + 1 == func.instructions.len() {
                    let instructions = &func.instructions[start_addr..index + 1];
                    blocks[cur_block.0] = instructions;
                    break;
                }
            }
        }

        Self { blocks, succ }
    }

    /// Returns an `Iterator` over all the blocks in the graph.
    ///
    /// No guarantees about the ordering of returned block is given, except that the entry block
    /// is guaranteed to be the first returned block.
    pub fn blocks(&self) -> BlocksIter<'_> {
        BlocksIter {
            index: 0,
            blocks: &self.blocks,
        }
    }

    /// Returns the list of sucessors of the given block.
    pub fn successors(&self, block: BlockId) -> &[BlockId] {
        self.succ
            .get(block)
            .map(|v| v.as_slice())
            .unwrap_or_default()
    }
}

/// Unique identifier for a block inside a [`ControlFlowGraph`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BlockId(usize);

impl Index for BlockId {
    fn index(&self) -> usize {
        self.0
    }
}

/// An `Iterator` over all blocks in a [`ControlFlowGraph`].
#[derive(Clone, Debug)]
pub struct BlocksIter<'a> {
    index: usize,
    blocks: &'a [&'a [Instruction]],
}

impl<'a> Iterator for BlocksIter<'a> {
    type Item = (BlockId, &'a [Instruction]);

    fn next(&mut self) -> Option<Self::Item> {
        let index = BlockId(self.index);
        let block = self.blocks.split_off_first()?;
        self.index += 1;
        Some((index, block))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl<'a> ExactSizeIterator for BlocksIter<'a> {
    fn len(&self) -> usize {
        self.blocks.len()
    }
}

impl<'a> FusedIterator for BlocksIter<'a> {}
