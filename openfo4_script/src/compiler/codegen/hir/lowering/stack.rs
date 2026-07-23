use std::alloc::Layout;
use std::collections::VecDeque;
use std::hash::Hash;
use std::mem;
use std::num::NonZeroUsize;

use game_common::utils::vec_ext::VecExt;
use hashbrown::{HashMap, HashSet};
use indexmap::IndexMap;
use openfo4_jit::{Block, FunctionBuilder, IntCompareOp, Type as JitType};

use crate::compiler::codegen::hir::lowering::{Local, PTR_TYPE, RESULT_TYPE, Translator};
use crate::compiler::codegen::hir::{Function, Instruction, Variable};

#[derive(Clone, Debug)]
pub struct StackFrameInfo {
    /// Size of the entire stack for this function.
    pub size: usize,
    /// Suspension points after function calls.
    pub call_points: IndexMap<InstructionPosition, SuspensionPoint>,
    pub points: IndexMap<usize, SuspensionPoint>,
}

impl StackFrameInfo {
    pub fn has_no_suspension_points(&self) -> bool {
        self.call_points.is_empty() && self.points.is_empty()
    }

    pub fn suspension_point(&self, tag: NonZeroUsize) -> Option<&SuspensionPoint> {
        self.call_points
            .values()
            .find(|v| v.tag == tag)
            .or_else(|| self.points.values().find(|v| v.tag == tag))
    }

    /// Returns the size of the entire stack frame, aligned up to `align`.
    pub fn size_aligned_to(&self, align: usize) -> usize {
        debug_assert_ne!(align, 0);
        debug_assert!(align.is_power_of_two());
        (self.size + align - 1) & !(align - 1)
    }
}

#[derive(Clone, Debug)]
pub struct SuspensionPoint {
    /// The unique tag identifying our code position.
    pub tag: NonZeroUsize,
    /// The set of variables that this suspension point saves.
    pub variables: Vec<Variable>,
    /// The block to skip towards when resuming.
    pub block: Block,
}

pub fn compute_suspension_points(
    builder: &mut FunctionBuilder<'_>,
    func: &Function,
    locals: &[Local],
) -> StackFrameInfo {
    let mut next_tag = NonZeroUsize::MIN;
    let mut call_points = IndexMap::new();

    for (block_index, block) in func.blocks.iter().enumerate() {
        for (inst_index, instruction) in block.iter().enumerate() {
            match instruction {
                Instruction::Call(_) | Instruction::CallIndirect(_) => {
                    let variables = func.variables().map(|v| v.0).collect();

                    let tag = next_tag;
                    next_tag = next_tag.checked_add(1).unwrap();

                    // A call resumtpion semantically happens in the middle of an instruction
                    // and we cannot just reuse the block at the start of the instruction, if
                    // one exists.
                    let block = builder.create_block(&[]);

                    call_points.insert(
                        InstructionPosition {
                            block: block_index,
                            instruction: inst_index,
                        },
                        SuspensionPoint {
                            tag,
                            variables,
                            block,
                        },
                    );
                }
                _ => {}
            }
        }
    }

    let mut max_size = 0;

    for point in call_points.values_mut() {
        let mut size = PTR_TYPE.size();

        // Sort by alignment descending to minimize stack space.
        point.variables.sort_unstable_by(|a, b| {
            let local_a = &locals[a.0];
            let local_b = &locals[b.0];
            local_b.ty.align().cmp(&local_a.ty.align())
        });

        for var in &point.variables {
            let local = &locals[var.0];
            size = (size + local.ty.align() - 1) & !(local.ty.align() - 1);
            size += local.ty.size();
        }

        max_size = max_size.max(size);
    }

    StackFrameInfo {
        size: max_size,
        points: IndexMap::new(),
        call_points,
    }
}

pub fn generate_suspension_blocks(
    translator: &mut Translator<'_>,
    func: &Function,
    frame: &StackFrameInfo,
) {
    if frame.has_no_suspension_points() {
        return;
    }

    let graph = Graph::new(frame, |var| {
        let ty = translator.locals[var.0].ty;
        (ty.size(), ty.align())
    });

    let offset = PTR_TYPE.size();

    let mut block_per_tag = HashMap::new();

    // Stack Pointer
    let mut root_params = vec![PTR_TYPE, RESULT_TYPE];
    for var in &graph.root().vars {
        let var = graph.root().resolve_var(*var, frame);
        let local = translator.locals[var.0];
        root_params.push(local.ty);
    }

    let root_block = translator.builder.create_block(&root_params);

    for tag in &graph.root().tags {
        block_per_tag.insert(*tag, root_block);
    }

    let mut queue = VecDeque::new();
    let mut stack = Vec::new();

    queue.push_back((graph.root(), offset));
    stack.push((graph.root(), offset, root_block, None));

    while let Some((node, mut offset)) = queue.pop_front() {
        // Stack Pointer, Result Value
        let mut params = vec![PTR_TYPE, RESULT_TYPE];

        for var in &node.vars {
            let var = node.resolve_var(*var, frame);
            let local = &translator.locals[var.0];
            offset = (offset + local.ty.align() - 1) & !(local.ty.align() - 1);
            offset += local.ty.size();
            params.push(local.ty);
        }

        for (child, _) in node.children() {
            let child = graph.get(child);

            let mut params = params.clone();
            for var in &child.vars {
                let var = child.resolve_var(*var, frame);
                let local = translator.locals[var.0];
                params.push(local.ty);
            }

            let block = translator.builder.create_block(&params);
            stack.push((child, offset, block, Some(block)));

            for tag in &child.tags {
                block_per_tag.insert(*tag, block);
            }
        }
    }

    for (node, mut offset, block, parent_block) in stack {
        translator.builder.switch_to_block(block);

        let stack_ptr = translator.builder.block_params(block)[0];
        let result = translator.builder.block_params(block)[1];
        let mut values = translator.builder.block_params(block)[2..].to_vec();

        for var in &node.vars {
            let var = node.resolve_var(*var, frame);
            let local = &translator.locals[var.0];
            let value = values.remove(0);

            offset = (offset + local.ty.align() - 1) & !(local.ty.align() - 1);
            translator.builder.store(value, stack_ptr, offset);

            offset += local.ty.size();
        }

        if let Some(parent) = parent_block {
            values.insert(0, stack_ptr);
            values.insert(1, result);
            translator.builder.jump(parent, &values);
        } else {
            let mut rvals = Vec::new();
            if let Some(ty) = func.returns {
                let ty = translator.lowered_ty_from_id(ty);
                let value = translator.builder.uninit(ty);
                rvals.push(value);
            }

            rvals.push(result);

            // Root block, we can return now.
            translator.builder.jump(translator.return_block, &rvals);
        }
    }

    for point in frame.points.values().chain(frame.call_points.values()) {
        let block = translator.builder.create_block(&[PTR_TYPE, RESULT_TYPE]);
        translator.builder.switch_to_block(block);

        let stack_ptr = translator.builder.block_params(block)[0];
        let result = translator.builder.block_params(block)[1];

        let mut values = vec![stack_ptr, result];
        for var in &point.variables {
            let local = translator.locals[var.0];
            let value = translator.builder.get_var(local.var);
            values.push(value);
        }

        // Store tag at offset 0.
        let tag = translator.builder.iconst(PTR_TYPE, point.tag.get());
        translator.builder.store(tag, stack_ptr, 0);

        let dst_block = block_per_tag.get(&point.tag).unwrap();
        translator.builder.jump(*dst_block, &values);

        translator.suspend_blocks.insert(point.tag, block);
    }
}

pub fn resume(translator: &mut Translator<'_>, frame: &StackFrameInfo, start_block: Block) {
    if frame.has_no_suspension_points() {
        translator.builder.jump(start_block, &[]);
        return;
    }

    let stack_ptr = translator.load_stack_ptr();

    let tag = translator.builder.load(PTR_TYPE, stack_ptr, 0);

    let resume_root_block = translator.builder.create_block(&[]);

    // If tag == 0 we don't resume.
    translator
        .builder
        .brif(tag, resume_root_block, &[], start_block, &[]);

    let blocks = build_resume_blocks(translator, frame);

    let graph = Graph::new(frame, |var| {
        let ty = translator.locals[var.0].ty;
        (ty.size(), ty.align())
    });

    // Tag was already read.
    let offset = PTR_TYPE.size();

    let mut queue = VecDeque::new();
    queue.push_back((graph.root(), offset, Vec::new(), resume_root_block));

    while let Some((node, mut offset, mut vars, node_block)) = queue.pop_front() {
        translator.builder.switch_to_block(node_block);

        for var in &node.vars {
            let var = node.resolve_var(*var, frame);
            let local = &translator.locals[var.0];

            offset = (offset + local.ty.align() - 1) & !(local.ty.align() - 1);

            let value = translator.builder.load(local.ty, stack_ptr, offset);
            vars.push(value);

            offset += local.ty.size();
        }

        // Each block must have at least one leaf or children.
        debug_assert!(node.leaves().count() != 0 || !node.children().is_empty());

        let mut next_block = translator.builder.create_block(&[]);

        let mut num_blocks_remaining = node.leaves().count() + node.children().len();

        for wanted_tag in node.leaves() {
            let dst_block = blocks.get(&wanted_tag).unwrap();

            // The tag will always match on the last entry.
            if num_blocks_remaining == 1 {
                num_blocks_remaining -= 1;
                translator.builder.jump(*dst_block, &vars);
                break;
            }

            let wanted_tag = translator.builder.iconst(PTR_TYPE, wanted_tag.get());
            let tag_matches = translator
                .builder
                .icmp(tag, wanted_tag, IntCompareOp::Equal);

            translator
                .builder
                .brif(tag_matches, *dst_block, &vars, next_block, &[]);

            translator.builder.switch_to_block(next_block);
            next_block = translator.builder.create_block(&[]);
            num_blocks_remaining -= 1;
        }

        debug_assert_eq!(node.children().len(), num_blocks_remaining);

        for (child, tags) in node.children() {
            debug_assert!(!tags.is_empty());

            let block = translator.builder.create_block(&[]);

            // The tag will always match on the last entry.
            if num_blocks_remaining == 1 {
                translator.builder.jump(block, &[]);
            } else {
                let mut any_tag_matches = translator.builder.iconst(JitType::I8, 0);
                for wanted_tag in tags {
                    let wanted_tag = translator.builder.iconst(PTR_TYPE, wanted_tag.get());
                    let tag_matches = translator
                        .builder
                        .icmp(tag, wanted_tag, IntCompareOp::Equal);
                    any_tag_matches = translator.builder.bor(any_tag_matches, tag_matches);
                }

                translator
                    .builder
                    .brif(any_tag_matches, block, &[], next_block, &[]);

                translator.builder.switch_to_block(next_block);
                next_block = translator.builder.create_block(&[]);
                num_blocks_remaining -= 1;
            }

            let node = graph.get(child);

            queue.push_back((node, offset, vars.clone(), block));
        }
    }
}

fn build_resume_blocks(
    translator: &mut Translator<'_>,
    frame: &StackFrameInfo,
) -> HashMap<NonZeroUsize, Block> {
    let mut blocks = HashMap::new();

    for point in frame.points.values().chain(frame.call_points.values()) {
        let params: Vec<_> = point
            .variables
            .iter()
            .map(|var| translator.locals[var.0].ty)
            .collect();

        let block = translator.builder.create_block(&params);
        translator.builder.switch_to_block(block);

        let values = translator.builder.block_params(block).to_vec();
        for (var, value) in point.variables.iter().zip(values) {
            let local = translator.locals[var.0];
            translator.builder.set_var(local.var, value);
        }

        translator.builder.jump(point.block, &[]);
        blocks.insert(point.tag, block);
    }

    blocks
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct InstructionPosition {
    pub block: usize,
    pub instruction: usize,
}

#[derive(Clone, Debug)]
struct Graph {
    blocks: Vec<Class>,
}

impl Graph {
    fn new<F>(frame: &StackFrameInfo, get_size_align: F) -> Self
    where
        F: Fn(Variable) -> (usize, usize),
    {
        let mut blocks: Vec<Class> = Vec::new();
        let mut last_class: HashMap<NonZeroUsize, usize> = HashMap::new();

        let mut index = 0;
        loop {
            let mut layout_per_var = HashMap::<_, HashSet<_>>::new();

            let mut found_any_var = false;
            for point in frame.points.values().chain(frame.call_points.values()) {
                if let Some(var) = point.variables.get(index) {
                    found_any_var = true;

                    let (size, align) = get_size_align(*var);
                    let layout = Layout::from_size_align(size, align).unwrap();

                    layout_per_var.entry(layout).or_default().insert(point.tag);
                }
            }

            for tags in layout_per_var.into_values() {
                let new_index = blocks.push_and_index(Class {
                    vars: vec![index],
                    tags: tags.clone(),
                    children: HashMap::new(),
                });

                for tag in tags {
                    if let Some(index) = last_class.get(&tag) {
                        let block = &mut blocks[*index];
                        block.children.insert(tag, new_index);
                    }

                    last_class.insert(tag, new_index);
                }
            }

            if !found_any_var {
                break;
            }

            index += 1;
        }

        let mut parents = vec![None; blocks.len()];
        for (index, block) in blocks.iter().enumerate() {
            for child in block.children.values() {
                parents[*child] = Some(index);
            }
        }

        // Collapse block A into block B where A dominates B and A dominates no other blocks,
        // i.e. there is only one edge going from A to B.
        for index in (0..blocks.len()).rev() {
            if let Some(parent) = parents[index] {
                let [block, parent] = blocks.get_disjoint_mut([index, parent]).unwrap();

                if block.tags.len() == parent.tags.len() {
                    debug_assert_eq!(block.tags, parent.tags);

                    // We effectively move the block upwards into the parent block, leaving
                    // our block dangling.
                    // Since the graph is a tree the parent is the only node with a reference to
                    // this block, which we have now overwritten.
                    parent.vars.extend(mem::take(&mut block.vars));
                    parent.children = mem::take(&mut block.children);
                }
            }
        }

        Self { blocks }
    }

    fn root(&self) -> &Class {
        &self.blocks[0]
    }

    fn get(&self, index: usize) -> &Class {
        &self.blocks[index]
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Class {
    vars: Vec<usize>,
    tags: HashSet<NonZeroUsize>,
    children: HashMap<NonZeroUsize, usize>,
}

impl Class {
    /// Returns all tags that end at this block.
    fn leaves(&self) -> impl Iterator<Item = NonZeroUsize> {
        self.tags
            .iter()
            .filter(|tag| !self.children.contains_key(*tag))
            .copied()
    }

    fn children(&self) -> HashMap<usize, Vec<NonZeroUsize>> {
        let mut blocks = HashMap::<_, Vec<_>>::new();

        for (tag, block) in &self.children {
            blocks.entry(*block).or_default().push(*tag);
        }

        blocks
    }

    /// Resolve the variable location into a real [`Variable`].
    fn resolve_var(&self, var: usize, frame: &StackFrameInfo) -> Variable {
        let any_tag = *self.tags.iter().next().unwrap();
        let any_tag = frame.suspension_point(any_tag).unwrap();

        let v = any_tag.variables[var];

        if cfg!(debug_assertions) {
            for tag in &self.tags {
                let tag = frame.suspension_point(*tag).unwrap();
                assert_eq!(tag.variables[var], v);
            }
        }

        v
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use hashbrown::HashSet;
    use indexmap::IndexMap;
    use openfo4_jit::Block;

    use crate::compiler::codegen::hir::Variable;
    use crate::compiler::codegen::hir::lowering::stack::{Graph, StackFrameInfo, SuspensionPoint};

    #[test]
    fn build_graph() {
        let a = Variable(0);
        let b = Variable(1);
        let c = Variable(2);
        let d = Variable(3);
        let e = Variable(4);
        let f = Variable(5);

        let mut frame = StackFrameInfo {
            size: 0,
            call_points: IndexMap::new(),
            points: IndexMap::new(),
        };
        frame.points.insert(
            0,
            SuspensionPoint {
                block: Block::DANGLING,
                tag: NonZeroUsize::new(1).unwrap(),
                variables: vec![a, b, c],
            },
        );
        frame.points.insert(
            1,
            SuspensionPoint {
                block: Block::DANGLING,
                tag: NonZeroUsize::new(2).unwrap(),
                variables: vec![a, c, d],
            },
        );
        frame.points.insert(
            2,
            SuspensionPoint {
                tag: NonZeroUsize::new(3).unwrap(),
                variables: vec![a, c],
                block: Block::DANGLING,
            },
        );
        frame.points.insert(
            3,
            SuspensionPoint {
                tag: NonZeroUsize::new(4).unwrap(),
                variables: vec![a, b, c],
                block: Block::DANGLING,
            },
        );
        frame.points.insert(
            4,
            SuspensionPoint {
                tag: NonZeroUsize::new(5).unwrap(),
                variables: vec![a, d, e],
                block: Block::DANGLING,
            },
        );
        frame.points.insert(
            5,
            SuspensionPoint {
                tag: NonZeroUsize::new(6).unwrap(),
                variables: vec![d, e],
                block: Block::DANGLING,
            },
        );
        frame.points.insert(
            6,
            SuspensionPoint {
                tag: NonZeroUsize::new(7).unwrap(),
                variables: vec![a, b, c, d, e, f],
                block: Block::DANGLING,
            },
        );
        frame.points.insert(
            7,
            SuspensionPoint {
                tag: NonZeroUsize::new(8).unwrap(),
                variables: vec![a, b, c, d, e, f],
                block: Block::DANGLING,
            },
        );

        let get_size_align = |var| {
            if var == a || var == b || var == c || var == d {
                (4, 4)
            } else if var == e {
                (2, 2)
            } else if var == f {
                (1, 1)
            } else {
                unreachable!()
            }
        };

        let graph = Graph::new(&frame, get_size_align);

        let t1 = NonZeroUsize::new(1).unwrap();
        let t2 = NonZeroUsize::new(2).unwrap();
        let t3 = NonZeroUsize::new(3).unwrap();
        let t4 = NonZeroUsize::new(4).unwrap();
        let t5 = NonZeroUsize::new(5).unwrap();
        let t6 = NonZeroUsize::new(6).unwrap();
        let t7 = NonZeroUsize::new(7).unwrap();
        let t8 = NonZeroUsize::new(8).unwrap();

        let paths = vec![
            // t1, t2, t4
            vec![
                vec![t1, t2, t3, t4, t5, t6, t7, t8],
                vec![t1, t2, t3, t4, t5, t7, t8],
                vec![t1, t2, t4, t7, t8],
            ],
            // t3
            vec![
                vec![t1, t2, t3, t4, t5, t6, t7, t8],
                vec![t1, t2, t3, t4, t5, t7, t8],
            ],
            // t5
            vec![
                vec![t1, t2, t3, t4, t5, t6, t7, t8],
                vec![t1, t2, t3, t4, t5, t7, t8],
                vec![t5],
            ],
            // t6
            vec![vec![t1, t2, t3, t4, t5, t6, t7, t8], vec![t6]],
            // t7, t8
            vec![
                vec![t1, t2, t3, t4, t5, t6, t7, t8],
                vec![t1, t2, t3, t4, t5, t7, t8],
                vec![t1, t2, t4, t7, t8],
                vec![t7, t8],
            ],
        ];
        let leaf_tags = vec![
            // t1, t2, t4
            vec![t1, t2, t4],
            // t3
            vec![t3],
            // t5
            vec![t5],
            // t6
            vec![t6],
            // t7, t8
            vec![t7, t8],
        ];

        for (path, leaf_tags) in paths.iter().zip(leaf_tags) {
            let mut block = graph.root();

            for (index, tags) in path.iter().enumerate() {
                assert_eq!(block.tags, tags.iter().copied().collect());

                if index + 1 == path.len() {
                    assert_eq!(
                        block.leaves().collect::<HashSet<_>>(),
                        leaf_tags.iter().copied().collect()
                    );
                    break;
                }

                let next_tags = &path[index + 1];

                let next = *block.children.get(&next_tags[0]).unwrap();
                for tag in next_tags {
                    assert_eq!(*block.children.get(tag).unwrap(), next);
                }

                block = graph.get(next);
            }
        }
    }
}
