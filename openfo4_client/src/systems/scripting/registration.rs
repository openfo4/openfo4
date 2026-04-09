use std::num::NonZeroU32;
use std::str::FromStr;

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
