use openfo4_animation::driver::AnimationDriver;
use openfo4_core::ecs::{Entity, World};
use openfo4_core::record::RecordId;
use openfo4_esp::FormId;
use openfo4_esp::conditions::{
    Combinator, Comparator, ComparisonValue, Condition, ConditionFunction, FunctionReceiver,
};

use crate::components::{ActorProperties, ActorProtection, MovementState};
use crate::state::esp::EspLoader;
use crate::state::npc::ObjectKeywords;
use crate::systems::actor_value::{ActorValue, ActorValueProperties};
use crate::systems::global::GlobalProperties;
use crate::systems::inventory::{Inventory, ItemData};
use crate::systems::movement::{MovementDirection, MovementSpeed};

pub fn evaluate(
    world: &World,
    esp_loader: &EspLoader,
    globals: &GlobalProperties,
    record: RecordId,
    conditions: &[Condition],
    subject: Entity,
    target: Entity,
) -> bool {
    dbg!(record);

    let mut result = true;
    let mut result_or = false;
    let mut last_op_is_or = false;

    for condition in conditions {
        let entity = match condition.receiver {
            FunctionReceiver::Subject => subject,
            FunctionReceiver::Target => target,
        };

        let eval = FuncEvaluator {
            world,
            entity,
            esp_loader,
            record,
        };

        let lhs = match &condition.function {
            // Constant
            ConditionFunction::IsWindowsPc => Value::Bool(true),
            ConditionFunction::IsPS3 => Value::Bool(false),

            // Actor Movement State
            ConditionFunction::IsMoving => eval.movement_state(|state| state.is_moving),
            ConditionFunction::IsRunning => todo!(),
            ConditionFunction::IsTurning => todo!(),
            ConditionFunction::IsRotating => todo!(),
            ConditionFunction::IsSprinting => eval.movement_state(|state| state.is_sprinting),
            ConditionFunction::IsSneaking => eval.movement_state(|state| state.is_sneaking),
            ConditionFunction::GetMovementDirection => {
                // 1 = Forward
                // 2 = Right
                // 3 = Backward
                // 4 = Left
                match world.get::<&MovementDirection>(entity) {
                    Ok(dir) => {
                        let dir = dir.get();
                        if dir.x.abs() > dir.z.abs() {
                            if dir.x.is_sign_positive() {
                                Value::I32(2)
                            } else {
                                Value::I32(4)
                            }
                        } else {
                            if dir.z.is_sign_negative() {
                                Value::I32(1)
                            } else {
                                Value::I32(3)
                            }
                        }
                    }
                    Err(_) => Value::I32(0),
                }
            }
            ConditionFunction::GetMovementSpeed => match world.get::<&MovementSpeed>(entity) {
                Ok(speed) => Value::F32(speed.speed),
                Err(_) => Value::F32(0.0),
            },
            // Actor Furniture State
            ConditionFunction::GetSitting => {
                // TODO
                Value::Bool(false)
            }
            ConditionFunction::GetSleeping => {
                // TODO
                Value::Bool(false)
            }
            // Actor Attach State
            ConditionFunction::IsAttacking => {
                // TODO
                Value::Bool(false)
            }
            ConditionFunction::IsMeleeAttacking => {
                // TODO
                Value::Bool(false)
            }
            ConditionFunction::GetActorStance => {
                // TODO
                Value::I32(0)
            }
            // Actor Inventory
            ConditionFunction::IsWeaponMagicOut => eval.inventory_bool(|inv| {
                inv.items().any(|(_, item)| match &item.data {
                    ItemData::Weapon(data) => data.drawn && item.equipped,
                    _ => false,
                })
            }),
            ConditionFunction::IsWeaponOut => eval.inventory_bool(|inv| {
                inv.items().any(|(_, item)| match &item.data {
                    ItemData::Weapon(data) => data.drawn && item.equipped,
                    _ => false,
                })
            }),
            ConditionFunction::IsShieldOut => {
                todo!()
            }
            ConditionFunction::IsTorchOut => {
                todo!()
            }
            ConditionFunction::WornHasKeyword(keyword) => {
                let keyword = esp_loader.resolve_form(*keyword, record).unwrap();
                eval.inventory_bool(|inv| {
                    inv.items()
                        .any(|(_, item)| item.combined_keywords.contains(&keyword))
                })
            }
            // Actor Properties
            ConditionFunction::IsActor => eval.actor_property(|_| true),
            ConditionFunction::IsProtected => {
                eval.actor_property(|props| props.protection == ActorProtection::Protected)
            }
            ConditionFunction::IsEssential => {
                eval.actor_property(|props| props.protection == ActorProtection::Essential)
            }
            ConditionFunction::IsChild => eval.actor_property(|props| props.is_child),
            ConditionFunction::IsGuard => todo!(),
            ConditionFunction::GetIsGhost => todo!(),
            ConditionFunction::GetValue { actor_value } => {
                eval.actor_value(*actor_value, |value| value.damaged())
            }
            ConditionFunction::GetValuePercent(actor_value) => {
                eval.actor_value(*actor_value, |value| value.damaged() / value.value * 100.0)
            }
            // Weather
            ConditionFunction::IsSnowing => todo!(),
            ConditionFunction::IsPleasant => todo!(),
            ConditionFunction::IsRaining => todo!(),
            ConditionFunction::IsTimespanSunrise => todo!(),
            ConditionFunction::IsTimespanMorning => todo!(),
            ConditionFunction::IsTimespanAfternoon => todo!(),
            ConditionFunction::IsTimespanEvening => todo!(),
            ConditionFunction::IsTimespanSunset => todo!(),
            ConditionFunction::IsTimespanNight => todo!(),
            ConditionFunction::IsTimespanMidnight => todo!(),
            ConditionFunction::IsTimespanAnyDay => todo!(),
            ConditionFunction::IsTimespanAnyNight => todo!(),
            // Object Properties
            ConditionFunction::HasKeyword { keyword } => {
                let keyword = esp_loader.resolve_form(*keyword, record).unwrap();
                match world.get::<&ObjectKeywords>(entity) {
                    Ok(keywords) => Value::Bool(keywords.contains(keyword)),
                    Err(_) => Value::Bool(false),
                }
            }
            // Animation
            ConditionFunction::GetGraphVariableInt(name) => {
                let name = name.as_str().unwrap();
                match world.get::<&AnimationDriver>(entity) {
                    Ok(driver) => driver
                        .get_variable(name)
                        .and_then(|var| var.to_i32())
                        .map(Value::I32)
                        .unwrap_or(Value::I32(0)),
                    Err(_) => Value::I32(0),
                }
            }
            ConditionFunction::GetGraphVariableFloat { name } => {
                let name = name.as_str().unwrap();
                match world.get::<&AnimationDriver>(entity) {
                    Ok(driver) => driver
                        .get_variable(name)
                        .and_then(|var| var.to_f32())
                        .map(Value::F32)
                        .unwrap_or(Value::F32(0.0)),
                    Err(_) => Value::F32(0.0),
                }
            }

            x => todo!("{:?}", x),
        };

        let lhs = match lhs {
            // A boolean value is represented as exactly 1.0.
            Value::Bool(v) => f32::from(v),
            Value::I32(v) => v as f32,
            Value::F32(v) => v,
        };

        let rhs = match condition.value {
            ComparisonValue::Const(rhs) => rhs,
            ComparisonValue::Global(global) => {
                let global = esp_loader.resolve_form(global, record).unwrap();
                *globals.variables.lock().get(&global).unwrap()
            }
        };

        let is_ok = match condition.comparator {
            Comparator::Equal => lhs == rhs,
            Comparator::NotEqual => lhs != rhs,
            Comparator::LessThan => lhs < rhs,
            Comparator::LessThanOrEqual => lhs <= rhs,
            Comparator::GreaterThan => lhs > rhs,
            Comparator::GreaterThanOrEqual => lhs >= rhs,
        };

        dbg!(record, result, result_or, is_ok, lhs, rhs, condition);

        // OR has a higher operator precedence than AND.
        // i.e. A AND B OR C is evaulated as A AND (B OR C).
        result_or |= is_ok;
        if condition.combinator == Combinator::And {
            // Collapse the last chain of OR expressions into the top
            // level AND expression.
            result &= result_or;
            result_or = false;

            // Since AND operators always have the lowest precedence
            // it is not possible for a value to become true again.
            if !result {
                return false;
            }
        }

        last_op_is_or = condition.combinator == Combinator::Or;
    }

    // Collapse the last OR expression if the conditions end with an OR,
    // i.e. A OR B OR <EOF>.
    if last_op_is_or {
        result &= result_or;
    }

    result
}

struct FuncEvaluator<'a> {
    world: &'a World,
    esp_loader: &'a EspLoader,
    entity: Entity,
    record: RecordId,
}

impl FuncEvaluator<'_> {
    fn movement_state<F, R>(&self, f: F) -> Value
    where
        F: FnOnce(&MovementState) -> R,
        R: Into<Value>,
    {
        self.world
            .get::<&MovementState>(self.entity)
            .map(|v| f(&v).into())
            .unwrap_or(Value::Bool(false))
    }

    fn actor_property<F, R>(&self, f: F) -> Value
    where
        F: FnOnce(&ActorProperties) -> R,
        R: Into<Value>,
    {
        self.world
            .get::<&ActorProperties>(self.entity)
            .map(|v| f(&v).into())
            .unwrap_or(Value::Bool(false))
    }

    fn actor_value<F>(&self, actor_value: FormId, f: F) -> Value
    where
        F: FnOnce(&ActorValue) -> f32,
    {
        let actor_value = self
            .esp_loader
            .resolve_form(actor_value, self.record)
            .unwrap();

        match self.world.get::<&ActorValueProperties>(self.entity) {
            Ok(values) => match values.get(actor_value) {
                Some(value) => Value::F32(f(&value)),
                None => Value::F32(0.0),
            },
            Err(_) => Value::F32(0.0),
        }
    }

    fn inventory_bool<F>(&self, f: F) -> Value
    where
        F: FnOnce(&Inventory) -> bool,
    {
        match self.world.get::<&Inventory>(self.entity) {
            Ok(inventory) => Value::Bool(f(&inventory)),
            Err(_) => Value::Bool(false),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum Value {
    Bool(bool),
    I32(i32),
    F32(f32),
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}
