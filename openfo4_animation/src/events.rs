use std::collections::HashMap;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EventId(usize);

#[derive(Clone, Debug, Default)]
pub struct EventSet {
    next_id: usize,
    by_name: HashMap<String, EventId>,
    by_id: HashMap<EventId, String>,
}

impl EventSet {
    pub fn insert(&mut self, name: String) -> EventId {
        let id = EventId(self.next_id);
        self.next_id += 1;
        assert_ne!(self.next_id, usize::MAX);
        self.by_name.insert(name, id);
        id
    }

    pub fn lookup_name(&self, name: &str) -> Option<EventId> {
        self.by_name.get(name).copied()
    }

    pub fn lookup_id(&self, id: EventId) -> Option<&str> {
        self.by_id.get(&id).map(|v| v.as_str())
    }
}
