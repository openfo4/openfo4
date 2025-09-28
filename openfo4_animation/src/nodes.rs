pub mod assign_variables;
pub mod blend;
pub mod clip;
pub mod event_driven;
pub mod events_from_range;
pub mod is_active;
pub mod layers;
pub mod list;
pub mod manual_selector;
pub mod modify_once;
pub mod state_machine;
pub mod timer;

use std::time::Duration;

use crate::events::EventSet;
use crate::variables::VariableSet;
use crate::{AnimationSet, BoneWeights, Event, NodeId, Value, VariableId};

pub trait GraphNode {
    /// Called once when the node is activated.
    fn activate<T>(&mut self, ctx: T)
    where
        T: Context,
    {
    }

    /// Called once when the node is deactivated.
    fn deactivate<T>(&mut self, ctx: T)
    where
        T: Context,
    {
    }

    fn update<T>(&mut self, ctx: T)
    where
        T: Context,
    {
    }

    fn handle_event<T>(&mut self, ctx: T, event: &Event)
    where
        T: Context,
    {
    }

    fn children(&self) -> Vec<NodeId> {
        Vec::new()
    }
}

pub trait Context {
    fn delta(&self) -> Duration;

    fn activate_node(&mut self, id: NodeId);

    fn deactivate_node(&mut self, id: NodeId);

    fn send_event(&mut self, event: Event);

    fn variables(&self) -> &VariableSet;

    fn events(&self) -> &EventSet;

    fn set_variable(&mut self, id: VariableId, value: Value);

    fn set_property(&mut self, node: NodeId, property: Property, value: Value);

    fn animations(&self) -> &AnimationSet;

    fn set_bone_weights(&mut self, node: NodeId, weights: BoneWeights) {
        let value = match weights {
            BoneWeights::Shared(v) => Value::F32(v),
            BoneWeights::Unique(v) => Value::BoneWeights(v),
        };

        self.set_property(node, Property::Weight, value);
    }

    fn set_playback_speed(&mut self, node: NodeId, speed: f32) {
        self.set_property(node, Property::Speed, Value::F32(speed));
    }
}

#[derive(Clone, Debug)]
pub enum NodeEvent {
    ActivateNode(NodeId),
    DeactivateNode(NodeId),
    Event(Event),
}

#[derive(Clone, Debug)]
pub struct Node {
    pub name: String,
    pub inner: NodeInner,
}

macro_rules! impl_node_inner {
    (
        $(
            #[$attr:meta]
        )*
        pub enum $enum_name:ident {
            $(
                $variant_name:ident($variant_ty:ty),
            )*
        }
    ) => {
        $(
            #[$attr]
        )*
        pub enum $enum_name {
            $(
                $variant_name($variant_ty),
            )*
        }

        impl GraphNode for NodeInner {
            fn activate<T>(&mut self, ctx: T)
            where
                T: Context,
            {
                match self {
                    $(
                        Self::$variant_name(v) => v.activate(ctx),
                    )*
                }
            }

            fn deactivate<T>(&mut self, ctx: T)
            where
                T: Context,
            {
                match self {
                    $(
                        Self::$variant_name(v) => v.deactivate(ctx),
                    )*
                }
            }

            fn update<T>(&mut self, ctx: T)
            where
                T: Context,
            {
                match self {
                    $(
                        Self::$variant_name(v) => v.update(ctx),
                    )*
                }
            }

            fn handle_event<T>(&mut self, ctx: T, event: &Event)
            where
                T: Context,
            {
                match self {
                    $(
                        Self::$variant_name(v) => v.handle_event(ctx, event),
                    )*
                }
            }

            fn children(&self) -> Vec<NodeId> {
                match self {
                    $(
                        Self::$variant_name(v) => v.children(),
                    )*
                }
            }
        }
    };
}

impl_node_inner! {
    #[derive(Clone, Debug)]
    pub enum NodeInner {
        List(list::List),
        StateMachine(state_machine::StateMachine),
        AssignVariables(assign_variables::AssignVariables),
        Timer(timer::Timer),
        IsActive(is_active::IsActive),
        ModifyOnce(modify_once::ModifyOnce),
        Blend(blend::Blend),
        Layers(layers::Layers),
        EventDriven(event_driven::EventDriven),
        Clip(clip::Clip),
        ManualSelector(manual_selector::ManualSelector),
        EventsFromRange(events_from_range::EventsFromRange),
    }
}

impl GraphNode for Node {
    fn activate<T>(&mut self, ctx: T)
    where
        T: Context,
    {
        self.inner.activate(ctx);
    }

    fn deactivate<T>(&mut self, ctx: T)
    where
        T: Context,
    {
        self.inner.deactivate(ctx);
    }

    fn update<T>(&mut self, ctx: T)
    where
        T: Context,
    {
        self.inner.update(ctx);
    }

    fn handle_event<T>(&mut self, ctx: T, event: &Event)
    where
        T: Context,
    {
        self.inner.handle_event(ctx, event);
    }

    fn children(&self) -> Vec<NodeId> {
        self.inner.children()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Property {
    Weight,
    GlobalWeight,
    Speed,
    GlobalSpeed,
}
