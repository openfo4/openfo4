use hashbrown::HashMap;
use openfo4_esp::FormId;
use openfo4_esp::records::Property;

const ACTOR_VALUE_STRENGTH: FormId = FormId(0x2c2);
const ACTOR_VALUE_PERCEPTION: FormId = FormId(0x2c3);
const ACTOR_VALUE_ENDURANCE: FormId = FormId(0x2c4);
const ACTOR_VALUE_CHARISMA: FormId = FormId(0x2c5);
const ACTOR_VALUE_INTELLIGENCE: FormId = FormId(0x2c6);
const ACTOR_VALUE_AGILITY: FormId = FormId(0x2c7);
const ACTOR_VALUE_LUCK: FormId = FormId(0x2c8);

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
