use glam::{Quat, Vec3};

use crate::{Decode, Error, Reader};

#[derive(Clone, Debug)]
pub struct BSConnectPointChildren {
    pub name: String,
    pub skinned: bool,
    pub points: Vec<String>,
}

impl Decode for BSConnectPointChildren {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let name = reader.get_u32()?;
        let name = reader.string(name)?.to_owned();
        let skinned = reader.get_bool()?;

        let points_len = reader.get_u32()?;
        let points = (0..points_len)
            .map(|_| String::decode(reader))
            .collect::<Result<_, _>>()?;

        Ok(Self {
            name,
            skinned,
            points,
        })
    }
}

#[derive(Clone, Debug)]
pub struct BSConnectPointParents {
    pub name: String,
    pub points: Vec<ConnectPoint>,
}

impl Decode for BSConnectPointParents {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let name = reader.get_u32()?;
        let name = reader.string(name)?.to_owned();

        let points_len = reader.get_u32()?;
        let points = (0..points_len)
            .map(|_| ConnectPoint::decode(reader))
            .collect::<Result<_, _>>()?;

        Ok(Self { name, points })
    }
}

#[derive(Clone, Debug)]
pub struct ConnectPoint {
    /// The name of the node where the attachment is bound to.
    pub parent: String,
    pub name: String,
    /// Rotation offset from the parent node.
    pub rotation: Quat,
    /// Translation offset from the parent node.
    pub translation: Vec3,
    pub scale: f32,
}

impl Decode for ConnectPoint {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, Error> {
        let parent = String::decode(reader)?;
        let name = String::decode(reader)?;
        let rotation = Quat::decode(reader)?;
        let translation = Vec3::decode(reader)?;
        let scale = reader.get_f32()?;

        Ok(Self {
            parent,
            name,
            rotation,
            translation,
            scale,
        })
    }
}
