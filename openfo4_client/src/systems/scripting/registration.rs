use std::num::NonZeroU32;
use std::str::FromStr;

use hashbrown::{HashMap, HashSet};
use openfo4_script::runtime::ObjectAddr;

#[derive(Clone, Debug, Default)]
pub struct RegistrationSet {
    events: Vec<(ObjectAddr, Event)>,
    // events: HashMap<EventKind, HashSet<ObjectAddr>>,
}

impl RegistrationSet {
    pub fn register(&mut self, object: ObjectAddr, event: Event) {
        // Only register for event once.
        self.unregister(object, event.clone());
        self.events.push((object, event));
    }

    pub fn unregister(&mut self, object: ObjectAddr, event: Event) {
        self.events
            .retain(|(k, v)| *k != object || v.kind == event.kind || v.sender == event.sender);
    }

    pub fn unregister_all(&mut self, object: ObjectAddr) {
        self.events.retain(|(v, _)| *v != object);
    }

    pub fn unregister_matching<F>(&mut self, object: ObjectAddr, mut f: F)
    where
        F: FnMut(&Event) -> bool,
    {
        self.events
            .retain(|(k, v)| if *k == object { !f(v) } else { true });
    }

    pub fn get(
        &mut self,
        kind: &EventKind,
        sender: &EventSender,
    ) -> impl Iterator<Item = &(ObjectAddr, Event)> {
        self.events
            .iter()
            .filter(|v| &v.1.kind == kind && &v.1.sender == sender)
            .collect::<Vec<_>>()
            .into_iter()
    }
}

#[derive(Clone, Debug)]
pub struct Event {
    pub kind: EventKind,
    pub sender: EventSender,
    /// Number of events send before the event is unregistered automatically.
    ///
    /// `None` means unlimited.
    pub limit: Option<NonZeroU32>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EventKind {
    Animation {
        event_name: String,
    },
    Custom(String),
    DetectionLosGain {
        viewer: ObjectAddr,
        target: ObjectAddr,
    },
    DetectionLosLost {
        viewer: ObjectAddr,
        target: ObjectAddr,
    },
    DirectLosGain {
        viewer: ObjectAddr,
        target: ObjectAddr,
        viewer_node: String,
        target_node: String,
    },
    DirectLosLost {
        viewer: ObjectAddr,
        target: ObjectAddr,
        viewer_node: String,
        target_node: String,
    },
    DistanceGreaterThan {
        object1: ObjectAddr,
        object2: ObjectAddr,
        distance: f32,
    },
    DistanceLessThan {
        object1: ObjectAddr,
        object2: ObjectAddr,
        distance: f32,
    },
    Hit {
        aggressor_filter: Option<ObjectAddr>,
        source_filter: Option<ObjectAddr>,
        projectile_filter: Option<ObjectAddr>,
        power_filter: i32,
        sneak_filter: i32,
        bash_filter: i32,
        block_filter: i32,
        filters_must_match: bool,
    },
    LooksMenu,
    MagicEffectApply {
        caster_filter: Option<ObjectAddr>,
        effect_filter: Option<ObjectAddr>,
        filters_must_match: bool,
    },
    MenuOpenClose(String),
    PlayerSleep,
    PlayerTeleport,
    PlayerWait,
    RadiationDamage,
    TrackedStats {
        stat: String,
        threshold: i32,
    },
    TutorialEvent {
        name: String,
    },

    DifficultyChanged,
    PlayerLoadGame,
    Kill,
    CombatStateChanged,
    ItemEquipped,
    PlayerSwimming,
    CompanionChange,
    EnterBleedout,
    PlayerHealTeammate,
    PipboyMenu,
    LocationChange,
    ItemAdded,
}

impl EventKind {
    pub fn func_name(&self) -> &'static str {
        match self {
            Self::ItemAdded => "OnItemAdded",
            _ => todo!(),
        }
    }

    /// Returns the function name for remote functions.
    pub fn remote_func_name(&self) -> &'static str {
        match self {
            Self::ItemAdded => "::remote_ObjectReference_OnItemAdded",
            _ => todo!(),
        }
    }

    pub fn remote_ref_alias_func_name(&self) -> &'static str {
        todo!()
    }
}

#[derive(Clone, Debug)]
pub struct UnknownEvent;

impl FromStr for EventKind {
    type Err = UnknownEvent;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "OnDifficultyChanged" => Ok(Self::DifficultyChanged),
            "OnPlayerLoadGame" => Ok(Self::PlayerLoadGame),
            "OnKill" => Ok(Self::Kill),
            "OnCombatStateChanged" => Ok(Self::CombatStateChanged),
            "OnItemEquipped" => Ok(Self::ItemEquipped),
            "OnPlayerSwimming" => Ok(Self::PlayerSwimming),
            "OnCompanionChange" => Ok(Self::CompanionChange),
            "OnEnterBleedout" => Ok(Self::EnterBleedout),
            "OnPlayerHealTeammate" => Ok(Self::PlayerHealTeammate),
            "OnPipboyMenu" => Ok(Self::PipboyMenu),
            "OnLocationChange" => Ok(Self::LocationChange),
            "OnItemAdded" => Ok(Self::ItemAdded),
            _ => Err(UnknownEvent),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum EventSender {
    /// Only receive events when the sender matches the given object.
    Object(ObjectAddr),
    /// Receive events from any sender.
    Any,
}
