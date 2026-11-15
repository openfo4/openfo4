use hashbrown::HashMap;
use openfo4_input::keyboard::{KeyCode, KeyboardInput};

use crate::state::inputs::ActorInputEvent;

#[derive(Clone, Debug, Default)]
pub struct KeyMap {
    keys: HashMap<KeyCode, KeyState>,
    events: Vec<ActorInputEvent>,
}

impl KeyMap {
    pub fn insert(&mut self, key: KeyCode, mode: EventMode, event: ActorInputEvent) {
        self.keys.insert(
            key,
            KeyState {
                is_pressed: false,
                event,
                mode,
            },
        );
    }

    pub fn process<I>(&mut self, events: I) -> impl Iterator<Item = ActorInputEvent>
    where
        I: Iterator<Item = KeyboardInput>,
    {
        for event in events {
            self.process_event(event);
        }

        self.process_continous();
        self.events.drain(..)
    }

    fn process_event(&mut self, event: KeyboardInput) {
        let Some(key) = event.key_code else {
            return;
        };

        if let Some(state) = self.keys.get_mut(&key) {
            if state.mode == EventMode::OnKeyDown && !state.is_pressed && event.state.is_pressed() {
                self.events.push(state.event);
            }

            state.is_pressed = event.state.is_pressed();
        }
    }

    fn process_continous(&mut self) {
        for state in self.keys.values() {
            if state.mode == EventMode::Continous && state.is_pressed {
                self.events.push(state.event);
            }
        }
    }
}

#[derive(Clone, Debug)]
struct KeyState {
    is_pressed: bool,
    event: ActorInputEvent,
    mode: EventMode,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum EventMode {
    /// Dispatch a single event when the key is down.
    OnKeyDown,
    /// Dispatch an event continously while the key is down.
    Continous,
}
