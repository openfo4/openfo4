use game_common::string::Lowercase;
use hashbrown::HashMap;
use openfo4_jit::Type as JitType;
use openfo4_pex::{Instruction, Value};

use crate::compiler::codegen::{CodegenData, FunctionDef, MAX_INSTRUCTIONS_BEFORE_PREEMPT, Type};

#[derive(Clone, Debug)]
pub struct FunctionProps {
    /// Whether the function may suspend.
    pub may_suspend: bool,
    /// Whether the function may unwind with an error.
    pub may_error: bool,
    /// Instructions indices BEFORE preemption is inserted.
    pub preemption_points: Vec<usize>,
    pub stages: Vec<Stage>,
    /// Stack size that this function requires.
    pub local_stack_size: usize,
    pub stack_frame: StackFrame,
}

pub fn function_properties(data: &CodegenData, func: &FunctionDef) -> FunctionProps {
    let class = data.classes.get(&func.class).unwrap();
    let function = &func.function;

    let mut may_error = false;

    let mut variables = Vec::new();

    for param in &function.params {
        let ty = Lowercase::new(&*param.ty);

        variables.push(StageVariable {
            name: Lowercase::new(param.name.as_str()),
            ty: data.resolve_ty(ty.as_str()),
            ty_jit: data.resolve_ty_jit(ty.as_str()),
        });
    }

    for local in &function.locals {
        let ty = Lowercase::new(&*local.ty);

        variables.push(StageVariable {
            name: Lowercase::new(local.name.as_str()),
            ty: data.resolve_ty(ty.as_str()),
            ty_jit: data.resolve_ty_jit(ty.as_str()),
        });
    }

    let mut locals = HashMap::new();
    for param in &function.params {
        locals.insert(Lowercase::new(&*param.name), Lowercase::new(&*param.ty));
    }
    for local in &function.locals {
        locals.insert(Lowercase::new(&*local.name), Lowercase::new(&*local.ty));
    }

    let mut globals = HashMap::new();
    for global in &class.fields {
        globals.insert(global.0, global.1);
    }

    let find_var = |var_name: &Lowercase<str>| -> Lowercase<String> {
        match var_name.as_inner() {
            "self" => return class.name.clone(),
            "::state" => return Lowercase::new("string"),
            _ => {}
        }

        if let Some(ty) = locals.get(var_name) {
            return ty.clone();
        }

        let mut class = data.get_class(class.name.as_str()).unwrap();
        loop {
            if let Some(ty) = class.fields.get(var_name) {
                return ty.clone();
            }

            if let Some(parent) = &class.parent {
                class = data.get_class(parent.as_str()).unwrap();
            } else {
                break;
            }
        }

        todo!()
    };

    let mut preemption_points = Vec::new();
    let mut instructions_no_preempt = 0;
    let mut stages = Vec::new();

    for (index, instruction) in function.instructions.iter().enumerate() {
        // Insert preemption point.
        // if instructions_no_preempt > MAX_INSTRUCTIONS_BEFORE_PREEMPT {
        //     stages.push(Stage {
        //         variables: variables.clone(),
        //     });

        //     preemption_points.push(index);
        //     instructions_no_preempt = 0;
        // }

        instructions_no_preempt += 1;

        match instruction {
            Instruction::Nop => {}
            Instruction::IAdd(_) => {}
            Instruction::ISub(_) => {}
            Instruction::IMul(_) => {}
            Instruction::IDiv(_) => {}
            Instruction::IMod(_) => {}
            Instruction::INeg(_) => {}
            Instruction::FAdd(_) => {}
            Instruction::FSub(_) => {}
            Instruction::FMul(_) => {}
            Instruction::FDiv(_) => {}
            Instruction::FNeg(_) => {}
            Instruction::Store(_) => {}
            Instruction::CmpEq(_) => {}
            Instruction::CmpLt(_) => {}
            Instruction::CmpLe(_) => {}
            Instruction::CmpGt(_) => {}
            Instruction::CmpGe(_) => {}
            // Insert preemption point at backedges.
            Instruction::Branch(offset) => {

                // if offset < 0 {
                //     preemption_points.push(index);
                //     stages.push(variables.clone());
                // }
            }
            Instruction::BranchIfFalse(op) | Instruction::BranchIfTrue(op) => {

                // if offset < 0 {
                //     preemption_points.push(index);
                //     stages.push(variables.clone());
                // }
            }
            // Preemption should always happen before we call another function.
            // This is more efficient because the callee may be called from
            // multiple callsites, where the caller might be able to guarantee
            // that preemption is not needed yet (e.g. host called functions).
            Instruction::CallMethod(op) => {
                let Value::Ident(object) = &op.object else {
                    unreachable!();
                };

                let object = Lowercase::new(object.as_str());
                let class_name = find_var(object.as_str());

                preemption_points.push(index);
                stages.push(variables.clone());
            }
            Instruction::CallParent(op) => {
                may_error = true;

                preemption_points.push(index);
                stages.push(variables.clone());
            }
            Instruction::CallStatic(op) => {
                preemption_points.push(index);
                stages.push(variables.clone());
            }
            Instruction::PropertyGet(op) => {
                let Value::Ident(object) = &op.value else {
                    unreachable!();
                };

                let object = Lowercase::new(object.as_str());
                let class_name = find_var(object.as_str());
                let prop_name = Lowercase::new(op.property.as_str());

                let Some(func_id) =
                    data.get_property_getter_id(class_name.as_str(), prop_name.as_str())
                else {
                    continue;
                };

                preemption_points.push(index);
                stages.push(variables.clone());
            }
            Instruction::PropertySet(op) => {
                let Value::Ident(object) = &op.object else {
                    unreachable!();
                };

                let object = Lowercase::new(object.as_str());
                let class_name = find_var(object.as_str());
                let prop_name = Lowercase::new(op.property.as_str());

                let Some(func_id) =
                    data.get_property_setter_id(class_name.as_str(), prop_name.as_str())
                else {
                    continue;
                };

                preemption_points.push(index);
                stages.push(variables.clone());
            }
            Instruction::Return(op) => {}
            Instruction::Cast(op) => {}
            Instruction::Is(op) => {}
            _ => (),
        }
    }

    // Reshuffle stage indices so that stages with the variables are next to each other.
    // This allows matching the entire section of stages with a single icmp(LessThan) op.
    let stages = {
        let mut vars = HashMap::<_, Vec<_>>::new();
        for (index, stage_var) in stages.iter().enumerate() {
            vars.entry(stage_var).or_default().push(index);
        }

        let mut next_index = 1;
        let mut stages = vec![
            Stage {
                index: 0,
                variables: Vec::new()
            };
            stages.len()
        ];
        for (vars, indices) in vars {
            for index in indices {
                stages[index] = Stage {
                    index: next_index,
                    variables: vars.clone(),
                };
                next_index += 1;
            }
        }

        stages
    };

    let may_suspend = !preemption_points.is_empty();

    let mut local_stack_size = 0;
    for stage in &stages {
        local_stack_size = local_stack_size.max(stage.size());
    }

    local_stack_size += size_of::<usize>();

    FunctionProps {
        may_suspend,
        may_error,
        stages: stages.clone(),
        preemption_points,
        local_stack_size,
        stack_frame: StackFrame {
            stages,
            size: local_stack_size,
        },
    }
}

#[derive(Clone, Debug)]
pub struct StackFrame {
    pub stages: Vec<Stage>,
    pub size: usize,
}

#[derive(Clone, Debug)]
pub struct Stage {
    pub index: usize,
    pub variables: Vec<StageVariable>,
}

impl Stage {
    pub fn size(&self) -> usize {
        let mut offset = JitType::Pointer.size();

        for var in &self.variables {
            if offset % var.ty_jit.align() != 0 {
                offset += var.ty_jit.align() - (offset % var.ty_jit.align());
            }

            offset += var.ty_jit.size();
        }

        offset
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StageVariable {
    pub name: Lowercase<String>,
    pub ty: Type,
    pub ty_jit: JitType,
}
