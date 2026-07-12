use std::num::NonZeroU32;
use std::str::FromStr;

use hashbrown::hash_map::Entry;
use hashbrown::{HashMap, HashSet};
use openfo4_script::runtime::ObjectAddr;
use parking_lot::Mutex;
use slab::Slab;

#[derive(Debug, Default)]
pub struct RegistrationSet {
    events: Mutex<Slab<(ObjectAddr, Event)>>,
    by_object: Mutex<HashMap<ObjectAddr, HashSet<usize>>>,
    by_sender: Mutex<HashMap<EventSender, HashSet<usize>>>,
}

impl RegistrationSet {
    pub fn register(&self, object: ObjectAddr, event: Event) {
        let mut events = self.events.lock();
        let mut by_object = self.by_object.lock();
        let mut by_sender = self.by_sender.lock();

        // Only register for event once.
        if let Entry::Occupied(mut indices) = by_object.entry(object) {
            indices.get_mut().retain(|index| {
                let e = &events.get(*index).unwrap();

                if e.1.kind == event.kind && e.1.sender == event.sender {
                    let (_, event) = events.remove(*index);

                    let events_of_sender = by_sender.get_mut(&event.sender).unwrap();
                    events_of_sender.remove(index);
                    if events_of_sender.is_empty() {
                        by_sender.remove(&event.sender);
                    }

                    false
                } else {
                    true
                }
            });

            if indices.get().is_empty() {
                indices.remove();
            }
        };

        let sender = event.sender;

        let index = events.insert((object, event));
        by_object.entry(object).or_default().insert(index);
        by_sender.entry(sender).or_default().insert(index);
    }

    pub fn unregister(&self, object: ObjectAddr, event: Event) {
        self.unregister_matching(object, |e| e.kind == event.kind && e.sender == event.sender);
    }

    pub fn unregister_all(&self, object: ObjectAddr) {
        let mut events = self.events.lock();
        let mut by_object = self.by_object.lock();
        let mut by_sender = self.by_sender.lock();

        let Some(indices) = by_object.remove(&object) else {
            return;
        };

        for index in indices {
            let (_, event) = events.remove(index);

            let events_of_sender = by_sender.get_mut(&event.sender).unwrap();
            events_of_sender.remove(&index);
            if events_of_sender.is_empty() {
                by_sender.remove(&event.sender);
            }
        }
    }

    pub fn unregister_matching<F>(&self, object: ObjectAddr, mut f: F)
    where
        F: FnMut(&Event) -> bool,
    {
        let mut events = self.events.lock();
        let mut by_object = self.by_object.lock();
        let mut by_sender = self.by_sender.lock();

        let Entry::Occupied(mut indices) = by_object.entry(object) else {
            return;
        };

        indices.get_mut().retain(|index| {
            let e = &events.get(*index).unwrap();

            if f(&e.1) {
                let (_, event) = events.remove(*index);

                let events_of_sender = by_sender.get_mut(&event.sender).unwrap();
                events_of_sender.remove(index);
                if events_of_sender.is_empty() {
                    by_sender.remove(&event.sender);
                }

                false
            } else {
                true
            }
        });

        if indices.get().is_empty() {
            indices.remove();
        }
    }

    pub fn for_each<F>(&mut self, kind: &EventKind, sender: &EventSender, mut f: F)
    where
        F: FnMut(ObjectAddr, &Event),
    {
        let events = self.events.lock();
        let by_sender = self.by_sender.lock();

        let Some(indices) = by_sender.get(sender) else {
            return;
        };

        for index in indices {
            let (object, event) = events.get(*index).unwrap();

            debug_assert_eq!(event.sender, *sender);

            if &event.kind != kind {
                continue;
            }

            f(*object, event);
        }
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
    /// The action of a scene is run.
    SceneAction,
    /// The object is activated.
    Activate,
    /// The alias of a quest has been selected.
    AliasInit,
    /// The alias of a quest has been reset.
    AliasReset,
    /// The alias of a quest is being destroyed.
    AliasShutdown,
    Animation {
        event_name: String,
    },
    /// A scene begins.
    SceneBegin,
    /// A topic info begins.
    TopcInfoBegin,
    /// The parent cell of the object attaches.
    CellAttach,
    /// The parent cell of the object deatches.
    CellDetach,
    /// The parent cell of the object loads.
    CellLoad,
    /// Called on package when it is ends.
    PackageChange,
    /// Called when an objects "close" animation has finished.
    Close,
    /// Called when an objects "open" animation has finished.
    Open,
    /// Called when an actor limb is crippled or "uncrippled".
    Cripple,
    /// Called when an actor stops dying.
    Death,
    /// Called when an actor starts dying.
    Dying,
    /// Called when the actor enters bleedout state.
    EnterBleedout,
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
    ItemUnequipped,
    PlayerSwimming,
    CompanionChange,
    PlayerHealTeammate,
    PipboyMenu,
    LocationChange,
    ItemAdded,
    ItemRemoved,
    TriggerEnter,
    TriggerLeave,
    /// `OnEquipped`
    ///
    /// An item was equipped by an actor.
    Equipped,
    /// `OnUnequipped`
    ///
    /// An item was unequipped by an actor.
    Unequipped,
    /// Called when a quest is started.
    QuestInit,
    /// Called when a quest is stopped.
    QuestShutdown,
    QuestStageSet,
}

impl EventKind {
    pub fn func_name(&self) -> &'static str {
        match self {
            Self::Activate => "OnActivate",
            // Alias
            Self::AliasInit => "OnAliasInit",
            Self::AliasReset => "OnAliasReset",
            Self::AliasShutdown => "OnAliasShutdown",
            Self::Animation { event_name: _ } => "OnAnimationEvent",
            // Inventory
            Self::ItemAdded => "OnItemAdded",
            Self::ItemRemoved => "OnItemRemoved",
            Self::ItemEquipped => "OnItemEquipped",
            Self::ItemUnequipped => "OnItemUnequipped",
            Self::Equipped => "OnEquipped",
            Self::Unequipped => "OnUnequipped",
            // Trigger
            Self::TriggerEnter => "OnTriggerEnter",
            Self::TriggerLeave => "OnTriggerLeave",
            // Scene
            Self::SceneAction => "OnAction",
            Self::SceneBegin => "OnBegin",
            Self::TopcInfoBegin => "OnBegin",
            // Cell
            Self::CellAttach => "OnCellAttach",
            Self::CellDetach => "OnCellDeatch",
            Self::CellLoad => "OnCellLoad",
            Self::PackageChange => "OnChange",
            Self::Open => "OnOpen",
            Self::Close => "OnClose",
            // Quest
            Self::QuestInit => "OnQuestInit",
            Self::QuestShutdown => "OnQuestShutdown",
            Self::QuestStageSet => "OnStageSet",
            // Actor "Health"
            Self::Cripple => "OnCripple",
            Self::Death => "OnDeath",
            Self::Dying => "OnDying",
            Self::EnterBleedout => "OnEnterBleedout",
            _ => todo!(),
        }
    }

    /// Returns the function name for remote functions.
    pub fn remote_func_name(&self) -> &'static str {
        match self {
            Self::ItemAdded => "::remote_ObjectReference_OnItemAdded",
            Self::TriggerEnter => "::remote_ObjectReference_OnTriggerEnter",
            Self::TriggerLeave => "::remote_ObjectReference_OnTriggerLeave",
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
