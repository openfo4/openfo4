use std::{collections::HashMap, num::NonZeroUsize};

use openfo4_jit::Type as JitType;
use openfo4_pex::{Function, Instruction, Value};

use crate::compiler::codegen::{ClassDef, CodegenData, MAX_INSTRUCTIONS_BEFORE_PREEMPT, Type};

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
    pub stack_size: usize,
}

pub fn function_properties(
    data: &CodegenData,
    class: &ClassDef,
    function: &Function,
) -> FunctionProps {
    let mut may_error = false;

    let mut variables = Vec::new();

    for param in &function.params {
        variables.push(StageVariable {
            name: param.name.to_lowercase(),
            ty: data.resolve_ty(&param.ty),
            ty_jit: data.resolve_ty_jit(&param.ty),
        });
    }

    for local in &function.locals {
        variables.push(StageVariable {
            name: local.name.to_lowercase(),
            ty: data.resolve_ty(&local.ty),
            ty_jit: data.resolve_ty_jit(&local.ty),
        });
    }

    let mut locals = HashMap::new();
    for param in &function.params {
        locals.insert(param.name.to_lowercase(), param.ty.to_lowercase());
    }
    for local in &function.locals {
        locals.insert(local.name.to_lowercase(), local.ty.to_lowercase());
    }

    let mut globals = HashMap::new();
    for global in &class.fields {
        globals.insert(global.0.to_lowercase(), global.1.to_lowercase());
    }

    let find_var = |var_name: &str| -> String {
        match &*var_name.to_lowercase() {
            "self" => return class.name.clone(),
            "::state" => return "string".to_owned(),
            _ => (),
        }

        if let Some(ty) = locals.get(&var_name.to_lowercase()) {
            return ty.clone();
        }

        if let Some(ty) = class.fields.get(&var_name.to_lowercase()) {
            return ty.to_lowercase();
        }

        todo!()
    };

    let mut preemption_points = Vec::new();
    let mut instructions_no_preempt = 0;
    let mut stack_size = 0;
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
                let Value::Int(offset) = *offset else {
                    unreachable!();
                };

                if offset < 0 {
                    preemption_points.push(index);
                    stages.push(variables.clone());
                }
            }
            Instruction::BranchIfFalse(op) | Instruction::BranchIfTrue(op) => {
                let Value::Int(offset) = op.offset else {
                    unreachable!();
                };

                if offset < 0 {
                    preemption_points.push(index);
                    stages.push(variables.clone());
                }
            }
            // Preemption should always happen before we call another function.
            // This is more efficient because the callee may be called from
            // multiple callsites, where the caller might be able to guarantee
            // that preemption is not needed yet (e.g. host called functions).
            Instruction::CallMethod(op) => {
                let Value::Ident(object) = &op.object else {
                    unreachable!();
                };

                let Value::Ident(func_name) = &op.function_name else {
                    unreachable!();
                };

                let class_name = find_var(object);

                let func = data.get_function(&class_name, func_name).unwrap();
                let props = func.props.as_ref().unwrap();

                may_error |= props.may_error;
                stack_size = stack_size.max(props.stack_size);

                preemption_points.push(index);
                stages.push(variables.clone());
            }
            Instruction::CallParent(op) => {
                todo!();
                let Value::Ident(func_name) = &op.function else {
                    unreachable!();
                };

                may_error = true;

                preemption_points.push(index);
                stages.push(variables.clone());
            }
            Instruction::CallStatic(op) => {
                let (Value::Ident(class_name), Value::Ident(func_name)) = (&op.class, &op.function)
                else {
                    unreachable!();
                };

                let func = data.get_function(class_name, func_name).unwrap();
                let props = func.props.as_ref().unwrap();

                may_error |= props.may_error;
                stack_size = stack_size.max(props.stack_size);

                preemption_points.push(index);
                stages.push(variables.clone());
            }
            Instruction::PropertyGet(op) => {}
            Instruction::PropertySet(op) => {}
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

    stack_size += local_stack_size;

    FunctionProps {
        may_suspend,
        may_error,
        stages,
        preemption_points,
        stack_size,
    }
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
    pub name: String,
    pub ty: Type,
    pub ty_jit: JitType,
}
