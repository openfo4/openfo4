use std::str::FromStr;

use hashbrown::{HashMap, HashSet};
use openfo4_script::runtime::ObjectAddr;

#[derive(Clone, Debug, Default)]
pub struct RegistrationSet {
    objects: HashMap<ObjectAddr, Vec<Event>>,
    events: HashMap<EventKind, HashSet<ObjectAddr>>,
}

impl RegistrationSet {
    pub fn register(&mut self, object: ObjectAddr, event: Event) {
        self.events.entry(event.kind).or_default().insert(object);
        self.objects.entry(object).or_default().push(event);
    }

    pub fn unregister(&mut self, object: ObjectAddr, event: Event) {
        todo!()
    }

    pub fn get(&self) {}
}

#[derive(Clone, Debug)]
pub struct Event {
    pub kind: EventKind,
    pub sender: EventSender,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum EventKind {
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

#[derive(Copy, Clone, Debug)]
pub enum EventSender {
    /// Only receive events when the sender matches the given object.
    Object(ObjectAddr),
    /// Receive events from any sender.
    Any,
}
