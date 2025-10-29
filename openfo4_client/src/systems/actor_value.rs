use hashbrown::HashMap;
use openfo4_esp::FormId;
use openfo4_esp::records::Property;

#[derive(Clone, Debug, Default)]
pub struct ActorValueProperties(HashMap<FormId, f32>);

impl ActorValueProperties {
    pub fn get(&self, id: FormId) -> Option<f32> {
        self.0.get(&id).copied()
    }

    pub fn set(&mut self, id: FormId, value: f32) {
        self.0.insert(id, value);
    }
}

impl FromIterator<Property> for ActorValueProperties {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = Property>,
    {
        Self(
            iter.into_iter()
                .map(|prop| (prop.actor_value, prop.value))
                .collect(),
        )
    }
}
