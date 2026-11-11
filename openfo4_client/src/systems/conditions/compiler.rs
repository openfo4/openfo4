use hashbrown::HashMap;
use openfo4_esp::FormId;
use openfo4_esp::conditions::{
    Combinator, Comparator, ComparisonValue, Condition, ConditionFunction, FunctionReceiver,
};
use openfo4_pex::{
    Binop, CallMethod, CallStatic, ConditionalBranch, Function, Instruction, Unop, Value,
    VariableDefinition,
};

use crate::systems::weather::WeatherClassification;

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
        skip_ins: Vec::new(),
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

            let next_ins = translator.func.instructions.len();
            for index in translator.skip_ins.drain(..) {
                let offset = (next_ins - index) as i32;
                let Instruction::BranchIfFalse(ins) = &mut translator.func.instructions[index]
                else {
                    unreachable!();
                };

                ins.offset = offset;
            }

            continue;
        }

        for index in translator.skip_ins.drain(..) {
            return_fixups.push(index);
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
    let subject = Value::Ident("entity".to_owned());

    let entity = match condition.receiver {
        FunctionReceiver::Subject => subject,
        FunctionReceiver::Target => Value::Ident("target".to_owned()),
        FunctionReceiver::Reference(id) => translator.create_global(id, "ObjectReference"),
        FunctionReceiver::CombatTarget => {
            translator.cast_and_call(subject, "Actor", "GetCombatTarget", &[], "Actor")
        }
        FunctionReceiver::MyKiller => {
            translator.cast_and_call(subject, "Actor", "GetKiller", &[], "Actor")
        }
        _ => todo!(),
    };

    let (lhs, ty) = match &condition.function {
        // Constant
        ConditionFunction::IsWindowsPc => (Value::Bool(true), Type::Bool),
        ConditionFunction::IsXBox | ConditionFunction::IsPS3 => (Value::Bool(false), Type::Bool),
        // Transform
        // ConditionFunction::GetPos { axis } => todo!(),
        // ConditionFunction::GetAngle { axis } => todo!(),
        // ConditionFunction::GetStartingPos { axis } => todo!(),
        // ConditionFunction::GetStartingAngle { axis } => todo!(),
        ConditionFunction::GetScale => {
            let value = translator.call(entity, "GetScale", &[], Type::Float);
            (value, Type::Float)
        }
        // Object Reference
        ConditionFunction::GetLocked => {
            let value = translator.call(entity, "IsLocked", &[], Type::Bool);
            (value, Type::Bool)
        }
        ConditionFunction::GetLockLevel => {
            let value = translator.call(entity, "GetLockLevel", &[], Type::Int);
            (value, Type::Int)
        }
        ConditionFunction::GetDisabled => {
            let value = translator.call(entity, "IsDisabled", &[], Type::Bool);
            (value, Type::Bool)
        }
        ConditionFunction::HasLoaded3D => {
            let value = translator.call(entity, "Is3DLoaded", &[], Type::Bool);
            (value, Type::Bool)
        }
        ConditionFunction::HasKeyword { keyword } => {
            let keyword = translator.create_global(*keyword, "Keyword");
            let value = translator.call(entity, "HasKeyword", &[keyword], Type::Bool);
            (value, Type::Bool)
        }
        ConditionFunction::GetValue { actor_value } => {
            let actor_value = translator.create_global(*actor_value, "ActorValue");
            let value = translator.call(entity, "GetValue", &[actor_value], Type::Float);
            (value, Type::Float)
        }
        ConditionFunction::GetValuePercent(actor_value) => {
            let actor_value = translator.create_global(*actor_value, "ActorValue");
            let value = translator.call(entity, "GetValuePercentage", &[actor_value], Type::Float);
            (value, Type::Float)
        }
        ConditionFunction::GetOpenState => {
            let value = translator.call(entity, "GetOpenState", &[], Type::Int);
            (value, Type::Int)
        }
        ConditionFunction::IsPowered => {
            let value = translator.call(entity, "IsPowered", &[], Type::Bool);
            (value, Type::Bool)
        }
        // Animation
        ConditionFunction::GetGraphVariableInt(name) => {
            let name = Value::String(name.as_str().unwrap().to_string());
            let value = translator.call(entity, "GetAnimationVariableInt", &[name], Type::Int);
            (value, Type::Int)
        }
        ConditionFunction::GetGraphVariableFloat { name } => {
            let name = Value::String(name.as_str().unwrap().to_string());
            let value = translator.call(entity, "GetAnimationVariableFloat", &[name], Type::Float);
            (value, Type::Float)
        }
        // Actor Properties
        ConditionFunction::IsActor => {
            translator.cast_or_skip(entity, "Actor");
            (Value::Bool(true), Type::Bool)
        }
        ConditionFunction::IsProtected => {
            let value = translator.cast_and_call(entity, "Actor", "IsProtected", &[], Type::Bool);
            (value, Type::Bool)
        }
        ConditionFunction::IsEssential => {
            let value = translator.cast_and_call(entity, "Actor", "IsEssential", &[], Type::Bool);
            (value, Type::Bool)
        }
        ConditionFunction::IsChild => {
            let value = translator.cast_and_call(entity, "Actor", "IsChild", &[], Type::Bool);
            (value, Type::Bool)
        }
        ConditionFunction::IsGuard => {
            let value = translator.cast_and_call(entity, "Actor", "IsGuard", &[], Type::Bool);
            (value, Type::Bool)
        }
        ConditionFunction::GetIsGhost => {
            let value = translator.cast_and_call(entity, "Actor", "IsGhost", &[], Type::Bool);
            (value, Type::Bool)
        }
        ConditionFunction::GetDead => {
            let value = translator.cast_and_call(entity, "Actor", "IsDead", &[], Type::Bool);
            (value, Type::Bool)
        }
        ConditionFunction::GetIsAlerted => {
            let value = translator.cast_and_call(entity, "Actor", "IsAlerted", &[], Type::Bool);
            (value, Type::Bool)
        }
        ConditionFunction::GetLevel => {
            let value = translator.cast_and_call(entity, "Actor", "GetLevel", &[], Type::Int);
            (value, Type::Int)
        }
        ConditionFunction::IsTalking => {
            let value = translator.cast_and_call(entity, "Actor", "IsTalking", &[], Type::Bool);
            (value, Type::Bool)
        }
        ConditionFunction::IsTrespassing => {
            let value = translator.cast_and_call(entity, "Actor", "IsTrespassing", &[], Type::Bool);
            (value, Type::Bool)
        }
        ConditionFunction::IsBleedingOut => {
            let value = translator.cast_and_call(entity, "Actor", "IsBleedingOut", &[], Type::Bool);
            (value, Type::Bool)
        }
        // Actor Movement State
        // ConditionFunction::IsMoving => todo!(),
        // ConditionFunction::IsTurning => todo!(),
        // ConditionFunction::IsRotating => todo!(),
        ConditionFunction::IsRunning => {
            let value = translator.cast_and_call(entity, "Actor", "IsRunning", &[], Type::Bool);
            (value, Type::Bool)
        }
        ConditionFunction::IsSneaking => {
            let value = translator.cast_and_call(entity, "Actor", "IsSneaking", &[], Type::Bool);
            (value, Type::Bool)
        }
        ConditionFunction::IsSprinting => {
            let value = translator.cast_and_call(entity, "Actor", "IsSprinting", &[], Type::Bool);
            (value, Type::Bool)
        }
        ConditionFunction::CanFlyHere => {
            let value = translator.cast_and_call(entity, "Actor", "CanFlyHere", &[], Type::Bool);
            (value, Type::Bool)
        }
        ConditionFunction::GetIsFlying => {
            let value = translator.cast_and_call(entity, "Actor", "IsFlying", &[], Type::Bool);
            (value, Type::Bool)
        }
        ConditionFunction::GetFlyingState => {
            let value = translator.cast_and_call(entity, "Actor", "GetFlyingState", &[], Type::Int);
            (value, Type::Int)
        }
        ConditionFunction::IsAllowedToFly => {
            let value =
                translator.cast_and_call(entity, "Actor", "IsAllowedToFly", &[], Type::Bool);
            (value, Type::Bool)
        }
        ConditionFunction::IsBeingRidden => {
            let value = translator.cast_and_call(entity, "Actor", "IsBeingRidden", &[], Type::Bool);
            (value, Type::Bool)
        }
        ConditionFunction::IsPlayersLastRiddenMount => {
            let value = translator.cast_and_call(
                entity,
                "Actor",
                "IsPlayersLastRiddenHorse",
                &[],
                Type::Bool,
            );
            (value, Type::Bool)
        }
        // ConditionFunction::GetMovementDirection => todo!(),
        // ConditionFunction::GetMovementSpeed => todo!(),
        // Actor Inventory
        ConditionFunction::IsWeaponMagicOut | ConditionFunction::IsWeaponOut => {
            let value = translator.cast_and_call(entity, "Actor", "IsWeaponDrawn", &[], Type::Bool);
            (value, Type::Bool)
        }
        ConditionFunction::IsShieldOut => todo!(),
        ConditionFunction::IsTorchOut => todo!(),
        ConditionFunction::WornHasKeyword(keyword) => {
            let keyword = translator.create_global(*keyword, "Keyword");
            let value =
                translator.cast_and_call(entity, "Actor", "WornHasKeyword", &[keyword], Type::Bool);
            (value, Type::Bool)
        }
        ConditionFunction::IsOverEncumbered => {
            let value =
                translator.cast_and_call(entity, "Actor", "IsOverEncumbered", &[], Type::Bool);
            (value, Type::Bool)
        }
        // Combat
        // TODO
        ConditionFunction::IsAttacking => (Value::Bool(false), Type::Bool),
        ConditionFunction::IsMeleeAttacking => (Value::Bool(false), Type::Bool),
        ConditionFunction::GetActorStance => (Value::Int(0), Type::Int),
        // Furniture
        ConditionFunction::GetSitting => (Value::Bool(false), Type::Bool),
        ConditionFunction::GetSleeping => (Value::Bool(false), Type::Bool),
        // Weather
        ConditionFunction::IsPleasant
        | ConditionFunction::IsCloudy
        | ConditionFunction::IsRaining
        | ConditionFunction::IsSnowing => {
            let weather =
                translator.call_static_func("Weather", "GetCurrentWeather", &[], "Weather");
            let lhs = translator.call(weather, "GetClassification", &[], Type::Int);
            let rhs = match condition.function {
                ConditionFunction::IsPleasant => WeatherClassification::Pleasant,
                ConditionFunction::IsCloudy => WeatherClassification::Cloudy,
                ConditionFunction::IsRaining => WeatherClassification::Raiy,
                ConditionFunction::IsSnowing => WeatherClassification::Snowy,
                _ => unreachable!(),
            };

            let value = translator.icmp_eq(lhs, Value::Int(rhs.to_i32()));
            (value, Type::Bool)
        }
        // Quest
        ConditionFunction::GetQuestRunning { quest } => {
            let quest = translator.create_global(*quest, "Quest");
            let value = translator.call(quest, "IsRunning", &[], Type::Bool);
            (value, Type::Bool)
        }
        ConditionFunction::GetStage { quest } => {
            let quest = translator.create_global(*quest, "Quest");
            let value = translator.call(quest, "GetCurrentStageID", &[], Type::Int);
            (value, Type::Int)
        }
        ConditionFunction::GetStageDone { quest, stage } => {
            let quest = translator.create_global(*quest, "Quest");
            let stage = Value::Int(stage.cast_signed());
            let value = translator.call(quest, "IsStageDone", &[stage], Type::Bool);
            (value, Type::Bool)
        }
        ConditionFunction::GetQuestCompleted { quest } => {
            let quest = translator.create_global(*quest, "Quest");
            let value = translator.call(quest, "IsCompleted", &[], Type::Bool);
            (value, Type::Bool)
        }
        // Faction
        ConditionFunction::CanPayCrimeGold { faction } => {
            let faction = translator.create_global(*faction, "Faction");
            let value = translator.call(faction, "CanPayCrimeGold", &[], Type::Bool);
            (value, Type::Bool)
        }
        ConditionFunction::GetCrimeGold { faction } => {
            let faction = translator.create_global(*faction, "Faction");
            let value = translator.call(faction, "GetCrimeGold", &[], Type::Int);
            (value, Type::Int)
        }
        ConditionFunction::GetCrimeGoldNonViolent { faction } => {
            let faction = translator.create_global(*faction, "Faction");
            let value = translator.call(faction, "GetCrimeGoldNonViolent", &[], Type::Int);
            (value, Type::Int)
        }
        ConditionFunction::GetCrimeGoldViolent { faction } => {
            let faction = translator.create_global(*faction, "Faction");
            let value = translator.call(faction, "GetCrimeGoldViolent", &[], Type::Int);
            (value, Type::Int)
        }
        ConditionFunction::GetInfamy { faction } => {
            let faction = translator.create_global(*faction, "Faction");
            let value = translator.call(faction, "GetInfamy", &[], Type::Int);
            (value, Type::Int)
        }
        ConditionFunction::GetInfamyNonViolent { faction } => {
            let faction = translator.create_global(*faction, "Faction");
            let value = translator.call(faction, "GetInfamyNonViolent", &[], Type::Int);
            (value, Type::Int)
        }
        ConditionFunction::GetInfamyViolent { faction } => {
            let faction = translator.create_global(*faction, "Faction");
            let value = translator.call(faction, "GetInfamyViolent", &[], Type::Int);
            (value, Type::Int)
        }
        ConditionFunction::GetFactionCombatReaction {
            faction,
            other_faction,
        } => {
            let faction = translator.create_global(*faction, "Faction");
            let other = translator.create_global(*other_faction, "Faction");
            let value = translator.call(faction, "GetFactionReaction", &[other], Type::Int);
            (value, Type::Int)
        }
        ConditionFunction::GetStolenItemValue { faction } => {
            let faction = translator.create_global(*faction, "Faction");
            let value = translator.call(faction, "GetStolenItemValueCrime", &[], Type::Int);
            (value, Type::Int)
        }
        ConditionFunction::GetStolenItemValueNoCrime { faction } => {
            let faction = translator.create_global(*faction, "Faction");
            let value = translator.call(faction, "GetStolenItemValueNoCrime", &[], Type::Int);
            (value, Type::Int)
        }
        ConditionFunction::GetPcEnemyOfFaction { faction } => {
            let faction = translator.create_global(*faction, "Faction");
            let value = translator.call(faction, "IsPlayerEnemy", &[], Type::Bool);
            (value, Type::Bool)
        }
        ConditionFunction::GetPcExpelled { faction } => {
            let faction = translator.create_global(*faction, "Faction");
            let value = translator.call(faction, "IsPlayerExpelled", &[], Type::Bool);
            (value, Type::Bool)
        }
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

trait TypeName {
    fn name(&self) -> &str;
}

impl<'a> TypeName for &'a str {
    fn name(&self) -> &str {
        self
    }
}

impl TypeName for Type {
    fn name(&self) -> &str {
        self.to_str()
    }
}

struct Translator {
    func: Function,
    next: usize,
    globals: HashMap<FormId, Global>,
    skip_ins: Vec<usize>,
}

impl Translator {
    /// Skips over the current condition if the given value is false.
    fn skip_if_false(&mut self, value: Value) {
        let index = self.func.instructions.len();
        self.func
            .instructions
            .push(Instruction::BranchIfFalse(ConditionalBranch {
                condition: value,
                offset: 0,
            }));
        self.skip_ins.push(index);
    }

    /// Casts the given `value` to the `target` or skips the condition if the cast did not suceed.
    fn cast_or_skip(&mut self, src: Value, target: &'static str) -> Value {
        let dst = Value::Ident(self.alloc_value(target));
        self.func.instructions.push(Instruction::Cast(Unop {
            src: src,
            dst: dst.clone(),
        }));
        self.skip_if_false(dst.clone());

        dst
    }

    fn cast_and_call<T>(
        &mut self,
        object: Value,
        class: &'static str,
        func: &'static str,
        args: &[Value],
        ty: T,
    ) -> Value
    where
        T: TypeName,
    {
        let object = self.cast_or_skip(object, class);
        self.call(object, func, args, ty)
    }

    fn call<T>(&mut self, object: Value, func: &'static str, args: &[Value], ty: T) -> Value
    where
        T: TypeName,
    {
        let dst = Value::Ident(self.alloc_value(ty.name()));

        self.func
            .instructions
            .push(Instruction::CallMethod(CallMethod {
                function_name: func.to_owned(),
                object,
                return_assignment: dst.clone(),
                args: args.to_vec(),
            }));

        dst
    }

    fn call_static_func<T>(
        &mut self,
        class: &'static str,
        func: &'static str,
        args: &[Value],
        ty: T,
    ) -> Value
    where
        T: TypeName,
    {
        let dst = Value::Ident(self.alloc_value(ty.name()));

        self.func
            .instructions
            .push(Instruction::CallStatic(CallStatic {
                class: class.to_owned(),
                function: func.to_owned(),
                args: args.to_vec(),
                return_assignment: dst.clone(),
            }));

        dst
    }

    fn icmp_eq(&mut self, lhs: Value, rhs: Value) -> Value {
        let dst = Value::Ident(self.alloc_value(Type::Bool.to_str()));
        self.func.instructions.push(Instruction::CmpEq(Binop {
            lhs,
            rhs,
            dst: dst.clone(),
        }));
        dst
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
        Combinator, Comparator, ComparisonValue, Condition, ConditionFunction, FunctionReceiver,
    };
    use openfo4_pex::{Binop, ConditionalBranch, Function, Instruction, Value};
    use openfo4_script::assembler::assemble;

    use crate::systems::conditions::compiler::build_func;

    fn make_condition(function: ConditionFunction, combinator: Combinator) -> Condition {
        Condition {
            receiver: FunctionReceiver::Subject,
            function,
            comparator: Comparator::Equal,
            value: ComparisonValue::Const(1.0),
            combinator,
        }
    }

    fn assemble_func(s: &str) -> Function {
        let module = assemble(s);
        module
            .classes
            .first()
            .unwrap()
            .states
            .first()
            .unwrap()
            .functions
            .first()
            .unwrap()
            .clone()
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

    #[test]
    fn build_func_skip_into_and() {
        let conditions = [
            make_condition(ConditionFunction::IsTalking, Combinator::And),
            make_condition(ConditionFunction::IsWindowsPc, Combinator::And),
        ];

        let res = build_func(&conditions);
        let expect = assemble_func(
            r#"
            class X:

            function eval(entity: ObjectReference, target: ObjectReference) -> bool:
            local v0: Actor
            local v1: bool
            local v2: bool
            local v3: bool
            v0 = cast entity
            branch_if_false v0 7
            v1 = call_method IsTalking v0
            v2 = cmpeq v1 true
            branch_if_false v2 4
            v3 = cmpeq true true // IsWindowsPc
            branch_if_false v3 2
            return true
            return false
            "#,
        );
        assert_eq!(res.func.instructions, expect.instructions);
    }

    #[test]
    fn build_func_skip_into_or() {
        let conditions = [
            make_condition(ConditionFunction::IsTalking, Combinator::Or),
            make_condition(ConditionFunction::IsWindowsPc, Combinator::Or),
        ];

        let res = build_func(&conditions);
        let expect = assemble_func(
            r#"
            class X:

            function eval(entity: ObjectReference, target: ObjectReference) -> bool:
            local v0: Actor
            local v1: bool
            local v2: bool
            local v3: bool
            v0 = cast entity
            branch_if_false v0 4
            v1 = call_method IsTalking v0
            v2 = cmpeq v1 true
            branch_if_true v2 3
            v3 = cmpeq true true // IsWindowsPc
            branch_if_false v3 2
            return true
            return false
            "#,
        );
        assert_eq!(res.func.instructions, expect.instructions);
    }
}
