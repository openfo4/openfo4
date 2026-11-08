use hashbrown::HashMap;
use openfo4_esp::FormId;
use openfo4_esp::conditions::{
    Combinator, Comparator, ComparisonValue, Condition, ConditionFunction,
};
use openfo4_pex::{
    Binop, CallMethod, ConditionalBranch, Function, Instruction, Unop, Value, VariableDefinition,
};

#[derive(Clone, Debug)]
pub struct Global {
    pub id: FormId,
    pub ty: &'static str,
    pub ident: String,
}

pub struct CondFunc {
    pub func: Function,
    pub globals: Vec<Global>,
}

pub fn build_func(conditions: &[Condition]) -> CondFunc {
    let mut translator = Translator {
        func: Function {
            name: "eval".to_owned(),
            instructions: Vec::new(),
            is_native: false,
            is_static: false,
            params: vec![
                VariableDefinition {
                    name: "entity".to_owned(),
                    ty: "ObjectReference".to_owned(),
                },
                VariableDefinition {
                    name: "target".to_owned(),
                    ty: "ObjectReference".to_owned(),
                },
            ],
            return_ty: "bool".to_owned(),
            locals: Vec::new(),
        },
        next: 0,
        globals: HashMap::new(),
    };

    let mut forward_jumps = Vec::new();
    let mut return_fixups = Vec::new();

    for (index, condition) in conditions.iter().enumerate() {
        let dst = lower_condition(&mut translator, condition);

        // OR has a higher operator precedence than AND.
        // i.e. A AND B OR C is evaulated as A AND (B OR C).
        // We will always treat the last condition as if it had an `And` combinator,
        // which will fold any open `Or` expressions.
        if condition.combinator == Combinator::Or && (index + 1) < conditions.len() {
            forward_jumps.push(translator.func.instructions.len());
            translator
                .func
                .instructions
                .push(Instruction::BranchIfTrue(ConditionalBranch {
                    condition: dst,
                    // Need to fixup later.
                    offset: 0,
                }));

            continue;
        }

        let index = translator.func.instructions.len();
        return_fixups.push(index);
        translator
            .func
            .instructions
            .push(Instruction::BranchIfFalse(ConditionalBranch {
                condition: dst,
                offset: 0,
            }));

        let next_instruction = translator.func.instructions.len();
        for index in forward_jumps.drain(..) {
            let offset = (next_instruction - index) as i32;
            let Instruction::BranchIfTrue(ins) = &mut translator.func.instructions[index] else {
                unreachable!();
            };

            ins.offset = offset;
        }
    }

    translator
        .func
        .instructions
        .push(Instruction::Return(Value::Bool(true)));

    let return_false_index = translator.func.instructions.len();
    translator
        .func
        .instructions
        .push(Instruction::Return(Value::Bool(false)));

    for index in return_fixups {
        let offset = (return_false_index - index) as i32;
        let Instruction::BranchIfFalse(ins) = &mut translator.func.instructions[index] else {
            unreachable!();
        };

        ins.offset = offset;
    }

    let globals = translator.globals.into_values().collect();

    CondFunc {
        func: translator.func,
        globals,
    }
}

fn lower_condition(translator: &mut Translator, condition: &Condition) -> Value {
    let (lhs, ty) = match &condition.function {
        // Constant
        ConditionFunction::IsWindowsPc => (Value::Bool(true), Type::Bool),
        ConditionFunction::IsXBox | ConditionFunction::IsPS3 => (Value::Bool(false), Type::Bool),
        // Transform
        // ConditionFunction::GetPos { axis } => todo!(),
        // ConditionFunction::GetAngle { axis } => todo!(),
        // ConditionFunction::GetStartingPos { axis } => todo!(),
        // ConditionFunction::GetStartingAngle { axis } => todo!(),
        ConditionFunction::GetScale => translator.call_object_func("GetScale", &[], Type::Float),
        // Object Reference
        ConditionFunction::GetLocked => translator.call_object_func("IsLocked", &[], Type::Bool),
        ConditionFunction::GetLockLevel => {
            translator.call_object_func("GetLockLevel", &[], Type::Int)
        }
        ConditionFunction::GetDisabled => {
            translator.call_object_func("IsDisabled", &[], Type::Float)
        }
        ConditionFunction::HasLoaded3D => {
            translator.call_object_func("Is3DLoaded", &[], Type::Bool)
        }
        ConditionFunction::HasKeyword { keyword } => {
            let keyword = translator.create_global(*keyword, "Keyword");
            translator.call_object_func("HasKeyword", &[keyword], Type::Bool)
        }
        ConditionFunction::GetValue { actor_value } => {
            let actor_value = translator.create_global(*actor_value, "ActorValue");
            translator.call_object_func("GetValue", &[actor_value], Type::Float)
        }
        ConditionFunction::GetValuePercent(actor_value) => {
            let actor_value = translator.create_global(*actor_value, "Actorvalue");
            translator.call_object_func("GetValuePercentage", &[actor_value], Type::Float)
        }
        // Animation
        ConditionFunction::GetGraphVariableInt(name) => {
            let name = Value::String(name.as_str().unwrap().to_string());
            translator.call_object_func("GetAnimationVariableInt", &[name], Type::Int)
        }
        ConditionFunction::GetGraphVariableFloat { name } => {
            let name = Value::String(name.as_str().unwrap().to_string());
            translator.call_object_func("GetAnimationVariableFloat", &[name], Type::Float)
        }
        // Actor Properties
        // ConditionFunction::IsActor => {
        //     translator.is_actor();
        //     todo!();
        // }
        ConditionFunction::IsProtected => {
            translator.call_actor_func("IsProtected", &[], Type::Bool)
        }
        ConditionFunction::IsEssential => {
            translator.call_actor_func("IsEssential", &[], Type::Bool)
        }
        ConditionFunction::IsChild => translator.call_actor_func("IsChild", &[], Type::Bool),
        ConditionFunction::IsGuard => translator.call_actor_func("IsGuard", &[], Type::Bool),
        ConditionFunction::GetIsGhost => translator.call_actor_func("IsGhost", &[], Type::Bool),
        ConditionFunction::GetDead => translator.call_actor_func("IsDead", &[], Type::Bool),
        // Actor Movement State
        // ConditionFunction::IsMoving => todo!(),
        // ConditionFunction::IsTurning => todo!(),
        // ConditionFunction::IsRotating => todo!(),
        ConditionFunction::IsRunning => translator.call_actor_func("IsRunning", &[], Type::Bool),
        ConditionFunction::IsSneaking => translator.call_actor_func("IsSneaking", &[], Type::Bool),
        ConditionFunction::IsSprinting => {
            translator.call_actor_func("IsSprinting", &[], Type::Bool)
        }
        ConditionFunction::IsTalking => translator.call_actor_func("IsTalking", &[], Type::Bool),
        ConditionFunction::CanFlyHere => translator.call_actor_func("CanFlyHere", &[], Type::Bool),
        // ConditionFunction::GetMovementDirection => todo!(),
        // ConditionFunction::GetMovementSpeed => todo!(),
        // Actor Inventory
        ConditionFunction::IsWeaponMagicOut | ConditionFunction::IsWeaponOut => {
            translator.call_actor_func("IsWeaponDrawn", &[], Type::Bool)
        }
        ConditionFunction::IsShieldOut => todo!(),
        ConditionFunction::IsTorchOut => todo!(),
        ConditionFunction::WornHasKeyword(keyword) => {
            let keyword = translator.create_global(*keyword, "Keyword");
            translator.call_actor_func("WornHasKeyword", &[keyword], Type::Bool)
        }
        // Combat
        // TODO
        ConditionFunction::IsAttacking => (Value::Bool(false), Type::Bool),
        ConditionFunction::IsMeleeAttacking => (Value::Bool(false), Type::Bool),
        ConditionFunction::GetActorStance => (Value::Int(0), Type::Int),
        x => {
            tracing::warn!("condition not yet implemented {x:?}, will default to false");
            return Value::Bool(false);
        }
    };

    let rhs = match condition.value {
        ComparisonValue::Const(v) => {
            match ty {
                // A boolean value is represented as exactly 1.0.
                Type::Bool => Value::Bool(v == 1.0),
                Type::Int => Value::Int(v as i32),
                Type::Float => Value::Float(v),
            }
        }
        ComparisonValue::Global(global) => {
            let global = translator.create_global(global, "GlobalVariable");
            let value = Value::Ident(translator.alloc_value(Type::Float.to_str()));

            translator
                .func
                .instructions
                .push(Instruction::CallMethod(CallMethod {
                    function_name: "GetValue".to_owned(),
                    object: global,
                    return_assignment: value.clone(),
                    args: Vec::new(),
                }));

            if ty != Type::Float {
                let dst = Value::Ident(translator.alloc_value(ty.to_str()));

                translator.func.instructions.push(Instruction::Cast(Unop {
                    src: value.clone(),
                    dst: dst.clone(),
                }));

                dst
            } else {
                value
            }
        }
    };

    let dst = Value::Ident(translator.alloc_value(Type::Bool.to_str()));

    match condition.comparator {
        Comparator::Equal => {
            translator.func.instructions.push(Instruction::CmpEq(Binop {
                lhs,
                rhs,
                dst: dst.clone(),
            }));
        }
        Comparator::NotEqual => {
            translator.func.instructions.push(Instruction::CmpEq(Binop {
                lhs,
                rhs,
                dst: dst.clone(),
            }));
            translator.func.instructions.push(Instruction::Not(Unop {
                src: dst.clone(),
                dst: dst.clone(),
            }));
        }
        Comparator::LessThan => {
            translator.func.instructions.push(Instruction::CmpLt(Binop {
                lhs,
                rhs,
                dst: dst.clone(),
            }));
        }
        Comparator::LessThanOrEqual => {
            translator.func.instructions.push(Instruction::CmpLe(Binop {
                lhs,
                rhs,
                dst: dst.clone(),
            }));
        }
        Comparator::GreaterThan => {
            translator.func.instructions.push(Instruction::CmpGt(Binop {
                lhs,
                rhs,
                dst: dst.clone(),
            }));
        }
        Comparator::GreaterThanOrEqual => {
            translator.func.instructions.push(Instruction::CmpGe(Binop {
                lhs,
                rhs,
                dst: dst.clone(),
            }));
        }
    }

    dst
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum Type {
    Bool,
    Int,
    Float,
}

impl Type {
    fn to_str(&self) -> &'static str {
        match self {
            Self::Bool => "bool",
            Self::Int => "int",
            Self::Float => "float",
        }
    }
}

struct Translator {
    func: Function,
    next: usize,
    globals: HashMap<FormId, Global>,
}

impl Translator {
    fn call_object_func(&mut self, func: &'static str, args: &[Value], ty: Type) -> (Value, Type) {
        let dst = self.alloc_value(ty.to_str());

        self.func
            .instructions
            .push(Instruction::CallMethod(CallMethod {
                function_name: func.to_owned(),
                object: Value::Ident("entity".to_owned()),
                return_assignment: Value::Ident(dst.clone()),
                args: args.to_vec(),
            }));

        (Value::Ident(dst), ty)
    }

    fn call_actor_func(&mut self, func: &'static str, args: &[Value], ty: Type) -> (Value, Type) {
        let actor = self.alloc_value("Actor");
        let dst = self.alloc_value(ty.to_str());

        self.func.instructions.push(Instruction::Cast(Unop {
            src: Value::Ident("entity".to_owned()),
            dst: Value::Ident(actor.clone()),
        }));

        self.func
            .instructions
            .push(Instruction::CallMethod(CallMethod {
                function_name: func.to_owned(),
                object: Value::Ident(actor),
                return_assignment: Value::Ident(dst.clone()),
                args: args.to_vec(),
            }));

        (Value::Ident(dst), ty)
    }

    fn alloc_value(&mut self, ty: &str) -> String {
        let value = format!("v{}", self.next);
        self.next += 1;

        self.func.locals.push(VariableDefinition {
            name: value.clone(),
            ty: ty.to_owned(),
        });

        value
    }

    fn create_global(&mut self, id: FormId, ty: &'static str) -> Value {
        if let Some(global) = self.globals.get(&id) {
            return Value::Ident(global.ident.clone());
        }

        let value = format!("v{}", self.next);
        self.next += 1;

        self.globals.insert(
            id,
            Global {
                id,
                ty,
                ident: value.clone(),
            },
        );
        Value::Ident(value)
    }
}

#[cfg(test)]
mod tests {
    use openfo4_esp::conditions::{
        Combinator, Comparator, ComparisonValue, Condition, ConditionDataObject, ConditionFunction,
        FunctionReceiver,
    };
    use openfo4_pex::{Binop, ConditionalBranch, Instruction, Value};

    use crate::systems::conditions::compiler::build_func;

    fn make_condition(function: ConditionFunction, combinator: Combinator) -> Condition {
        Condition {
            object: ConditionDataObject::Subject,
            receiver: FunctionReceiver::Subject,
            function,
            comparator: Comparator::Equal,
            value: ComparisonValue::Const(1.0),
            combinator,
        }
    }

    #[test]
    fn build_func_combinator() {
        let conditions = [
            make_condition(ConditionFunction::IsWindowsPc, Combinator::And),
            make_condition(ConditionFunction::IsXBox, Combinator::Or),
            make_condition(ConditionFunction::IsPS3, Combinator::Or),
        ];

        let res = build_func(&conditions);
        assert_eq!(
            res.func.instructions,
            vec![
                Instruction::CmpEq(Binop {
                    lhs: Value::Bool(true), // IsWindowsPc
                    rhs: Value::Bool(true),
                    dst: Value::Ident("v0".to_owned()),
                }),
                Instruction::BranchIfFalse(ConditionalBranch {
                    condition: Value::Ident("v0".to_owned()),
                    offset: 6,
                }),
                Instruction::CmpEq(Binop {
                    lhs: Value::Bool(false), // IsXBox
                    rhs: Value::Bool(true),
                    dst: Value::Ident("v1".to_owned()),
                }),
                Instruction::BranchIfTrue(ConditionalBranch {
                    condition: Value::Ident("v1".to_owned()),
                    offset: 3,
                }),
                Instruction::CmpEq(Binop {
                    lhs: Value::Bool(false), // IsPS3
                    rhs: Value::Bool(true),
                    dst: Value::Ident("v2".to_owned())
                }),
                Instruction::BranchIfFalse(ConditionalBranch {
                    condition: Value::Ident("v2".to_owned()),
                    offset: 2,
                }),
                Instruction::Return(Value::Bool(true)),
                Instruction::Return(Value::Bool(false)),
            ]
        );
    }
}
