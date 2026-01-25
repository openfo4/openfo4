use std::num::NonZeroU32;
use std::str::FromStr;

use hashbrown::{HashMap, HashSet};
use openfo4_script::runtime::ObjectAddr;

#[derive(Clone, Debug, Default)]
pub struct RegistrationSet {
    objects: HashMap<ObjectAddr, Vec<Event>>,
    // events: HashMap<EventKind, HashSet<ObjectAddr>>,
}

impl RegistrationSet {
    pub fn register(&mut self, object: ObjectAddr, event: Event) {
        // self.events
        //     .entry(event.kind.clone())
        //     .or_default()
        //     .insert(object);
        self.objects.entry(object).or_default().push(event);
    }

    pub fn unregister(&mut self, object: ObjectAddr, event: Event) {
        todo!()
    }

    pub fn unregister_all(&mut self, object: ObjectAddr) {
        todo!()
    }

    pub fn unregister_matching<F>(&mut self, object: ObjectAddr, f: F)
    where
        F: FnMut(&Event) -> bool,
    {
        todo!()
    }

    pub fn get(&self) {}
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
