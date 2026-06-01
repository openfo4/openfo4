use bytes::Buf;
use openfo4_esp_derive::{BinaryStruct, Record};

use crate::records::{Array, Model, SoundLevel};
use crate::{Decode, Error, FormId, LString, ZString};

#[derive(Clone, Debug, Record)]
pub struct ObjectModification {
    #[record("EDID")]
    pub id: Option<ZString>,
    #[record("FULL")]
    pub full_name: Option<LString>,
    #[record("DESC")]
    pub description: Option<ZString>,
    #[record("MODL")]
    pub model: Model,
    #[record("DATA")]
    pub data: ObjectModificationData,
    #[record("MNAM")]
    pub target_keywords: Option<Array<FormId>>,
    #[record("FNAM")]
    pub filter_keywords: Option<Array<FormId>>,
    #[record("LNAM")]
    pub loose_mod: Option<FormId>,
    #[record("NAM1")]
    pub priority: Option<u8>,
}

#[derive(Clone, Debug)]
pub struct ObjectModificationData {
    pub form_type: ObjectModificationFormType,
    pub max_rank: u8,
    pub level_tier_scaled_offset: u8,
    pub attach_point: FormId,
    pub attach_parent_slots: Vec<FormId>,
    pub includes: Vec<ObjectModificationInclude>,
    pub properties: Vec<ObjectModificationProperty>,
}

impl Decode for ObjectModificationData {
    type Error = Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let include_count = u32::decode(&mut buf)?;
        let property_count = u32::decode(&mut buf)?;
        u16::decode(&mut buf)?;
        let form_type = ObjectModificationFormType::decode(&mut buf)?;

        let max_rank = u8::decode(&mut buf)?;
        let level_tier_scaled_offset = u8::decode(&mut buf)?;
        let attach_point = FormId::decode(&mut buf)?;

        let attach_parent_slots_count = u32::decode(&mut buf)?;
        let attach_parent_slots = (0..attach_parent_slots_count)
            .map(|_| FormId::decode(&mut buf))
            .collect::<Result<_, _>>()?;

        for _ in 0..u32::decode(&mut buf)? {
            u32::decode(&mut buf)?;
            u32::decode(&mut buf)?;
        }

        let includes = (0..include_count)
            .map(|_| ObjectModificationInclude::decode(&mut buf))
            .collect::<Result<_, _>>()?;

        let properties = (0..property_count)
            .map(|_| ObjectModificationProperty::decode(&mut buf, form_type))
            .collect::<Result<_, _>>()?;

        Ok(Self {
            form_type,
            max_rank,
            level_tier_scaled_offset,
            attach_point,
            attach_parent_slots,
            includes,
            properties,
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ObjectModificationFormType {
    Armor,
    Npc,
    Weapon,
    None,
}

impl Decode for ObjectModificationFormType {
    type Error = Error;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let bytes = <[u8; 4]>::decode(buf)?;
        match &bytes {
            b"ARMO" => Ok(Self::Armor),
            b"NPC_" => Ok(Self::Npc),
            b"WEAP" => Ok(Self::Weapon),
            b"NONE" => Ok(Self::None),
            _ => Err(Error::BadEnumValue(
                stringify!(ObjectModificationFormType),
                u32::from_le_bytes(bytes).into(),
            )),
        }
    }
}

#[derive(Clone, Debug, BinaryStruct)]
pub struct ObjectModificationInclude {
    pub mod_: FormId,
    pub min_level: u8,
    pub optional: bool,
    pub dont_use_all: bool,
}

#[derive(Clone, Debug)]
pub struct ObjectModificationProperty {
    pub ty: ObjectModificationPropertyType,
    pub function_type: ObjectModificationFunctionType,
    pub step: f32,
}

impl ObjectModificationProperty {
    fn decode<B>(mut buf: B, form_type: ObjectModificationFormType) -> Result<Self, Error>
    where
        B: Buf,
    {
        let value_type = ObjectModificationValueType::decode(&mut buf)?;
        <[u8; 3]>::decode(&mut buf)?;

        let function_type = u8::decode(&mut buf)?;
        <[u8; 3]>::decode(&mut buf)?;
        let function_type = match value_type {
            ObjectModificationValueType::Float => match function_type {
                0 => ObjectModificationFunctionType::Set,
                1 => ObjectModificationFunctionType::MulAdd,
                2 => ObjectModificationFunctionType::Add,
                _ => {
                    return Err(Error::BadEnumValue(
                        stringify!(ObjectModificationFunctionType),
                        function_type.into(),
                    ));
                }
            },
            ObjectModificationValueType::Bool => match function_type {
                0 => ObjectModificationFunctionType::Set,
                1 => ObjectModificationFunctionType::And,
                2 => ObjectModificationFunctionType::Or,
                _ => {
                    return Err(Error::BadEnumValue(
                        stringify!(ObjectModificationFunctionType),
                        function_type.into(),
                    ));
                }
            },
            ObjectModificationValueType::String => {
                return Err(Error::BadEnumValue(
                    stringify!(ObjectModificationFunctionType),
                    function_type.into(),
                ));
            }
            ObjectModificationValueType::Enum => match function_type {
                0 => ObjectModificationFunctionType::Set,
                _ => {
                    return Err(Error::BadEnumValue(
                        stringify!(ObjectModificationFunctionType),
                        function_type.into(),
                    ));
                }
            },
            ObjectModificationValueType::FormIdFloat | ObjectModificationValueType::FormIdInt => {
                match function_type {
                    0 => ObjectModificationFunctionType::Set,
                    1 => ObjectModificationFunctionType::Rem,
                    2 => ObjectModificationFunctionType::Add,
                    _ => {
                        return Err(Error::BadEnumValue(
                            stringify!(ObjectModificationFunctionType),
                            function_type.into(),
                        ));
                    }
                }
            }
            ObjectModificationValueType::Int => match function_type {
                0 => ObjectModificationFunctionType::Set,
                2 => ObjectModificationFunctionType::Add,
                _ => {
                    return Err(Error::BadEnumValue(
                        stringify!(ObjectModificationFunctionType),
                        function_type.into(),
                    ));
                }
            },
        };

        let property = u16::decode(&mut buf)?;
        <[u8; 2]>::decode(&mut buf)?;
        let property = match form_type {
            ObjectModificationFormType::Armor => match property {
                0 => ObjectModificationPropertyProperty::Enchantments,
                1 => ObjectModificationPropertyProperty::BashImpactDataSet,
                2 => ObjectModificationPropertyProperty::BlockMaterial,
                3 => ObjectModificationPropertyProperty::Keywords,
                4 => ObjectModificationPropertyProperty::Weight,
                5 => ObjectModificationPropertyProperty::Value,
                6 => ObjectModificationPropertyProperty::Rating,
                7 => ObjectModificationPropertyProperty::AddonIndex,
                8 => ObjectModificationPropertyProperty::BodyPart,
                9 => ObjectModificationPropertyProperty::DamageTypeValue,
                10 => ObjectModificationPropertyProperty::ActorValues,
                11 => ObjectModificationPropertyProperty::Health,
                12 => ObjectModificationPropertyProperty::ColorRemappingIndex,
                13 => ObjectModificationPropertyProperty::MaterialSwaps,
                _ => {
                    return Err(Error::BadEnumValue(
                        stringify!(ObjectModificationPropertyProperty),
                        property.into(),
                    ));
                }
            },
            ObjectModificationFormType::Npc => match property {
                0 => ObjectModificationPropertyProperty::Keywords,
                1 => ObjectModificationPropertyProperty::ForcedInventory,
                2 => ObjectModificationPropertyProperty::XpOffset,
                3 => ObjectModificationPropertyProperty::Enchantments,
                4 => ObjectModificationPropertyProperty::ColorRemappingIndex,
                5 => ObjectModificationPropertyProperty::MaterialSwaps,
                _ => {
                    return Err(Error::BadEnumValue(
                        stringify!(ObjectModificationPropertyProperty),
                        property.into(),
                    ));
                }
            },
            ObjectModificationFormType::Weapon => match property {
                0 => ObjectModificationPropertyProperty::Speed,
                1 => ObjectModificationPropertyProperty::Reach,
                2 => ObjectModificationPropertyProperty::MinRange,
                3 => ObjectModificationPropertyProperty::MaxRange,
                4 => ObjectModificationPropertyProperty::AttackDelaySec,
                5 => ObjectModificationPropertyProperty::Unknown,
                6 => ObjectModificationPropertyProperty::OutOfRangeDamageMult,
                7 => ObjectModificationPropertyProperty::SecondaryDamage,
                8 => ObjectModificationPropertyProperty::CriticalChargeBonus,
                9 => ObjectModificationPropertyProperty::HitBehavior,
                10 => ObjectModificationPropertyProperty::Rank,
                11 => ObjectModificationPropertyProperty::Unknown,
                12 => ObjectModificationPropertyProperty::AmmoCapacity,
                13 => ObjectModificationPropertyProperty::Unknown,
                14 => ObjectModificationPropertyProperty::Unknown,
                15 => ObjectModificationPropertyProperty::Type,
                16 => ObjectModificationPropertyProperty::IsPlayerOnly,
                17 => ObjectModificationPropertyProperty::NpcsUseAmmo,
                18 => ObjectModificationPropertyProperty::HasChargingReload,
                19 => ObjectModificationPropertyProperty::IsMinorCrime,
                20 => ObjectModificationPropertyProperty::IsFixedRange,
                21 => ObjectModificationPropertyProperty::HasEffectOnDeath,
                22 => ObjectModificationPropertyProperty::HasAlternateRumble,
                23 => ObjectModificationPropertyProperty::IsNonHostile,
                24 => ObjectModificationPropertyProperty::IgnoreResist,
                25 => ObjectModificationPropertyProperty::IsAutomatic,
                26 => ObjectModificationPropertyProperty::CantDrop,
                27 => ObjectModificationPropertyProperty::IsNonPlayable,
                28 => ObjectModificationPropertyProperty::AttackDamage,
                29 => ObjectModificationPropertyProperty::Value,
                30 => ObjectModificationPropertyProperty::Weight,
                31 => ObjectModificationPropertyProperty::Keywords,
                32 => ObjectModificationPropertyProperty::AimModel,
                33 => ObjectModificationPropertyProperty::AimModelMinConeDegrees,
                34 => ObjectModificationPropertyProperty::AimModelMaxConeDegrees,
                35 => ObjectModificationPropertyProperty::AimModelConeIncreasePerShot,
                36 => ObjectModificationPropertyProperty::AimModelConeDecreasePerSec,
                37 => ObjectModificationPropertyProperty::AimModelConeDecreaseDelayMs,
                38 => ObjectModificationPropertyProperty::AimModelConeSneakMultiplier,
                39 => ObjectModificationPropertyProperty::AimModelRecoilDiminishSpringForce,
                40 => ObjectModificationPropertyProperty::AimModelRecoilDiminishSightsMult,
                41 => ObjectModificationPropertyProperty::AimModelRecoilMaxDegPerShot,
                42 => ObjectModificationPropertyProperty::AimModelRecoilMinDegPerShot,
                43 => ObjectModificationPropertyProperty::AimModelRecoilHipMult,
                44 => ObjectModificationPropertyProperty::AimModelRecoilShotsForRunaway,
                45 => ObjectModificationPropertyProperty::AimModelRecoilArcDeg,
                46 => ObjectModificationPropertyProperty::AimModelRecoilArcRotateDeg,
                47 => ObjectModificationPropertyProperty::AimModelConeIronSightsMultiplier,
                48 => ObjectModificationPropertyProperty::HasScope,
                49 => ObjectModificationPropertyProperty::ZoomDataFovMult,
                50 => ObjectModificationPropertyProperty::FireSeconds,
                51 => ObjectModificationPropertyProperty::NumProjectiles,
                52 => ObjectModificationPropertyProperty::AttackSound,
                53 => ObjectModificationPropertyProperty::AttackSound2D,
                54 => ObjectModificationPropertyProperty::AttackLoop,
                55 => ObjectModificationPropertyProperty::AttackFailSound,
                56 => ObjectModificationPropertyProperty::IdleSound,
                57 => ObjectModificationPropertyProperty::EquipSound,
                58 => ObjectModificationPropertyProperty::UnequipSound,
                59 => ObjectModificationPropertyProperty::SoundLevel,
                60 => ObjectModificationPropertyProperty::ImpactDataSet,
                61 => ObjectModificationPropertyProperty::Ammo,
                62 => ObjectModificationPropertyProperty::CritEffect,
                63 => ObjectModificationPropertyProperty::BashImpactDataSet,
                64 => ObjectModificationPropertyProperty::BlockMaterial,
                65 => ObjectModificationPropertyProperty::Enchantments,
                66 => ObjectModificationPropertyProperty::AimModelBaseStability,
                67 => ObjectModificationPropertyProperty::ZoomData,
                68 => ObjectModificationPropertyProperty::ZoomDataOverlay,
                69 => ObjectModificationPropertyProperty::ZoomDataImageSpace,
                70 => ObjectModificationPropertyProperty::ZoomDataCameraOffsetX,
                71 => ObjectModificationPropertyProperty::ZoomDataCameraOffsetY,
                72 => ObjectModificationPropertyProperty::ZoomDataCameraOffsetZ,
                73 => ObjectModificationPropertyProperty::EquipSlot,
                74 => ObjectModificationPropertyProperty::SoundLevelMult,
                75 => ObjectModificationPropertyProperty::NpcAmmoList,
                76 => ObjectModificationPropertyProperty::ReloadSpeed,
                77 => ObjectModificationPropertyProperty::DamageTypeValues,
                78 => ObjectModificationPropertyProperty::AccuracyBonus,
                79 => ObjectModificationPropertyProperty::AttackActionPointCost,
                80 => ObjectModificationPropertyProperty::OverrideProjectile,
                81 => ObjectModificationPropertyProperty::HasBoltAction,
                82 => ObjectModificationPropertyProperty::StaggerValue,
                83 => ObjectModificationPropertyProperty::SightedTransitionSeconds,
                84 => ObjectModificationPropertyProperty::FullPowerSeconds,
                85 => ObjectModificationPropertyProperty::HoldInputToPower,
                86 => ObjectModificationPropertyProperty::HasRepeatableSingleFire,
                87 => ObjectModificationPropertyProperty::MinPowerPerShot,
                88 => ObjectModificationPropertyProperty::ColorRemappingIndex,
                89 => ObjectModificationPropertyProperty::MaterialSwaps,
                90 => ObjectModificationPropertyProperty::CriticalDamageMult,
                91 => ObjectModificationPropertyProperty::FastEquipSound,
                92 => ObjectModificationPropertyProperty::DisableShells,
                93 => ObjectModificationPropertyProperty::HasChargingAttack,
                94 => ObjectModificationPropertyProperty::ActorValues,
                _ => {
                    return Err(Error::BadEnumValue(
                        stringify!(ObjectModificationPropertyProperty),
                        property.into(),
                    ));
                }
            },
            ObjectModificationFormType::None => {
                return Err(Error::BadEnumValue(
                    stringify!(ObjectModificationPropertyProperty),
                    property.into(),
                ));
            }
        };

        let value1 = u32::decode(&mut buf)?;
        let value2 = u32::decode(&mut buf)?;
        let step = f32::decode(&mut buf)?;

        let ty = match property {
            ObjectModificationPropertyProperty::Enchantments => {
                let enchantment = FormId(value1);
                ObjectModificationPropertyType::Enchantments(enchantment)
            }
            ObjectModificationPropertyProperty::BashImpactDataSet => {
                ObjectModificationPropertyType::BashImpactDataSet
            }
            ObjectModificationPropertyProperty::BlockMaterial => {
                ObjectModificationPropertyType::BlockMaterial
            }
            ObjectModificationPropertyProperty::Keywords => {
                let keyword = FormId(value1);
                ObjectModificationPropertyType::Keywords(keyword)
            }
            ObjectModificationPropertyProperty::Weight => {
                let weight = f32::from_bits(value1);
                ObjectModificationPropertyType::Weight(weight)
            }
            ObjectModificationPropertyProperty::Value => {
                let value = f32::from_bits(value1);
                ObjectModificationPropertyType::Value(value)
            }
            ObjectModificationPropertyProperty::Rating => {
                let rating = value1;
                ObjectModificationPropertyType::Rating(rating)
            }
            ObjectModificationPropertyProperty::AddonIndex => {
                let index = value1;
                ObjectModificationPropertyType::AddonIndex(index)
            }
            ObjectModificationPropertyProperty::BodyPart => {
                ObjectModificationPropertyType::BodyPart
            }
            ObjectModificationPropertyProperty::DamageTypeValue => {
                let damage_type = FormId(value1);
                let value = value2;
                ObjectModificationPropertyType::DamageTypeValue(damage_type, value)
            }
            ObjectModificationPropertyProperty::ActorValues => {
                let actor_value = FormId(value1);
                let value = f32::from_bits(value2);
                ObjectModificationPropertyType::ActorValues(actor_value, value)
            }
            ObjectModificationPropertyProperty::Health => {
                let value = if value_type == ObjectModificationValueType::Int {
                    value1.cast_signed() as f32
                } else {
                    f32::from_bits(value1)
                };

                ObjectModificationPropertyType::Health(value)
            }
            ObjectModificationPropertyProperty::ColorRemappingIndex => {
                let value = f32::from_bits(value1);
                ObjectModificationPropertyType::ColorRemappingIndex(value)
            }
            ObjectModificationPropertyProperty::MaterialSwaps => {
                let mswap = FormId(value1);
                ObjectModificationPropertyType::MaterialSwaps(mswap)
            }
            ObjectModificationPropertyProperty::ForcedInventory => {
                let item = FormId(value1);
                ObjectModificationPropertyType::ForcedInventory(item)
            }
            ObjectModificationPropertyProperty::XpOffset => {
                ObjectModificationPropertyType::XpOffset
            }
            ObjectModificationPropertyProperty::Speed => {
                let speed = f32::from_bits(value1);
                ObjectModificationPropertyType::Speed(speed)
            }
            ObjectModificationPropertyProperty::Reach => ObjectModificationPropertyType::Reach,
            ObjectModificationPropertyProperty::MinRange => {
                let value = f32::from_bits(value1);
                ObjectModificationPropertyType::MinRange(value)
            }
            ObjectModificationPropertyProperty::MaxRange => {
                let value = f32::from_bits(value1);
                ObjectModificationPropertyType::MaxRange(value)
            }
            ObjectModificationPropertyProperty::AttackDelaySec => {
                let value = f32::from_bits(value1);
                ObjectModificationPropertyType::AttackDelaySec(value)
            }
            ObjectModificationPropertyProperty::OutOfRangeDamageMult => {
                let value = f32::from_bits(value1);
                ObjectModificationPropertyType::OutOfRangeDamageMult(value)
            }
            ObjectModificationPropertyProperty::SecondaryDamage => {
                let value = f32::from_bits(value1);
                ObjectModificationPropertyType::SecondaryDamage(value)
            }
            ObjectModificationPropertyProperty::CriticalChargeBonus => {
                let value = f32::from_bits(value1);
                ObjectModificationPropertyType::CriticalChargeBonus(value)
            }
            ObjectModificationPropertyProperty::HitBehavior => {
                ObjectModificationPropertyType::HitBehavior
            }
            ObjectModificationPropertyProperty::Rank => ObjectModificationPropertyType::Rank,
            ObjectModificationPropertyProperty::AmmoCapacity => {
                let value = value1;
                ObjectModificationPropertyType::AmmoCapacity(value)
            }
            ObjectModificationPropertyProperty::Type => ObjectModificationPropertyType::Type,
            ObjectModificationPropertyProperty::IsPlayerOnly => {
                let value = value1 != 0;
                ObjectModificationPropertyType::IsPlayerOnly(value)
            }
            ObjectModificationPropertyProperty::NpcsUseAmmo => {
                let value = value1 != 0;
                ObjectModificationPropertyType::NpcsUseAmmo(value)
            }
            ObjectModificationPropertyProperty::HasChargingReload => {
                let value = value1 != 0;
                ObjectModificationPropertyType::HasChargingReload(value)
            }
            ObjectModificationPropertyProperty::IsMinorCrime => {
                let value = value1 != 0;
                ObjectModificationPropertyType::IsMinorCrime(value)
            }
            ObjectModificationPropertyProperty::IsFixedRange => {
                let value = value1 != 0;
                ObjectModificationPropertyType::IsFixedRange(value)
            }
            ObjectModificationPropertyProperty::HasEffectOnDeath => {
                let value = value1 != 0;
                ObjectModificationPropertyType::HasEffectOnDeath(value)
            }
            ObjectModificationPropertyProperty::HasAlternateRumble => {
                let value = value1 != 0;
                ObjectModificationPropertyType::HasAlternateRumble(value)
            }
            ObjectModificationPropertyProperty::IsNonHostile => {
                let value = value1 != 0;
                ObjectModificationPropertyType::IsNonHostile(value)
            }
            ObjectModificationPropertyProperty::IgnoreResist => {
                let value = value1 != 0;
                ObjectModificationPropertyType::IgnoreResist(value)
            }
            ObjectModificationPropertyProperty::IsAutomatic => {
                let value = value1 != 0;
                ObjectModificationPropertyType::IsAutomatic(value)
            }
            ObjectModificationPropertyProperty::CantDrop => {
                let value = value1 != 0;
                ObjectModificationPropertyType::CantDrop(value)
            }
            ObjectModificationPropertyProperty::IsNonPlayable => {
                let value = value1 != 0;
                ObjectModificationPropertyType::IsNonPlayable(value)
            }
            ObjectModificationPropertyProperty::AttackDamage => {
                let value = f32::from_bits(value1);
                ObjectModificationPropertyType::AttackDamage(value)
            }
            ObjectModificationPropertyProperty::AimModel => {
                let model = FormId(value1);
                ObjectModificationPropertyType::AimModel(model)
            }
            ObjectModificationPropertyProperty::AimModelMinConeDegrees => {
                let value = f32::from_bits(value1);
                ObjectModificationPropertyType::AimModelMinConeDegrees(value)
            }
            ObjectModificationPropertyProperty::AimModelMaxConeDegrees => {
                let value = f32::from_bits(value1);
                ObjectModificationPropertyType::AimModelMaxConeDegrees(value)
            }
            ObjectModificationPropertyProperty::AimModelConeIncreasePerShot => {
                let value = f32::from_bits(value1);
                ObjectModificationPropertyType::AimModelConeIncreasePerShot(value)
            }
            ObjectModificationPropertyProperty::AimModelConeDecreasePerSec => {
                let value = f32::from_bits(value1);
                ObjectModificationPropertyType::AimModelConeDecreasePerSec(value)
            }
            ObjectModificationPropertyProperty::AimModelConeDecreaseDelayMs => {
                let value = value1;
                ObjectModificationPropertyType::AimModelConeDecreaseDelayMs(value)
            }
            ObjectModificationPropertyProperty::AimModelConeSneakMultiplier => {
                let value = f32::from_bits(value1);
                ObjectModificationPropertyType::AimModelConeSneakMultiplier(value)
            }
            ObjectModificationPropertyProperty::AimModelRecoilDiminishSpringForce => {
                let value = f32::from_bits(value1);
                ObjectModificationPropertyType::AimModelRecoilDiminishSpringForce(value)
            }
            ObjectModificationPropertyProperty::AimModelRecoilDiminishSightsMult => {
                let value = f32::from_bits(value1);
                ObjectModificationPropertyType::AimModelRecoilDiminishSightsMult(value)
            }
            ObjectModificationPropertyProperty::AimModelRecoilMaxDegPerShot => {
                let value = f32::from_bits(value1);
                ObjectModificationPropertyType::AimModelRecoilMaxDegPerShot(value)
            }
            ObjectModificationPropertyProperty::AimModelRecoilMinDegPerShot => {
                let value = f32::from_bits(value1);
                ObjectModificationPropertyType::AimModelRecoilMinDegPerShot(value)
            }
            ObjectModificationPropertyProperty::AimModelRecoilHipMult => {
                let value = f32::from_bits(value1);
                ObjectModificationPropertyType::AimModelRecoilHipMult(value)
            }
            ObjectModificationPropertyProperty::AimModelRecoilShotsForRunaway => {
                let value = f32::from_bits(value1);
                ObjectModificationPropertyType::AimModelRecoilShotsForRunaway(value)
            }
            ObjectModificationPropertyProperty::AimModelRecoilArcDeg => {
                let value = f32::from_bits(value1);
                ObjectModificationPropertyType::AimModelRecoilArcDeg(value)
            }
            ObjectModificationPropertyProperty::AimModelRecoilArcRotateDeg => {
                let value = f32::from_bits(value1);
                ObjectModificationPropertyType::AimModelRecoilArcRotateDeg(value)
            }
            ObjectModificationPropertyProperty::AimModelConeIronSightsMultiplier => {
                let value = f32::from_bits(value1);
                ObjectModificationPropertyType::AimModelConeIronSightsMultiplier(value)
            }
            ObjectModificationPropertyProperty::HasScope => {
                let value = value1 != 0;
                ObjectModificationPropertyType::HasScope(value)
            }
            ObjectModificationPropertyProperty::ZoomDataFovMult => {
                let value = f32::from_bits(value1);
                ObjectModificationPropertyType::ZoomDataFovMult(value)
            }
            ObjectModificationPropertyProperty::FireSeconds => {
                let value = f32::from_bits(value1);
                ObjectModificationPropertyType::FireSeconds(value)
            }
            ObjectModificationPropertyProperty::NumProjectiles => {
                let value = f32::from_bits(value1);
                ObjectModificationPropertyType::NumProjectiles(value)
            }
            ObjectModificationPropertyProperty::AttackSound => {
                let sound = FormId(value1);
                ObjectModificationPropertyType::AttackSound(sound)
            }
            ObjectModificationPropertyProperty::AttackSound2D => {
                let sound = FormId(value1);
                ObjectModificationPropertyType::AttackSound2D(sound)
            }
            ObjectModificationPropertyProperty::AttackLoop => {
                ObjectModificationPropertyType::AttackLoop
            }
            ObjectModificationPropertyProperty::AttackFailSound => {
                let sound = FormId(value1);
                ObjectModificationPropertyType::AttackFailSound(sound)
            }
            ObjectModificationPropertyProperty::IdleSound => {
                let sound = FormId(value1);
                ObjectModificationPropertyType::IdleSound(sound)
            }
            ObjectModificationPropertyProperty::EquipSound => {
                let sound = FormId(value1);
                ObjectModificationPropertyType::EquipSound(sound)
            }
            ObjectModificationPropertyProperty::UnequipSound => {
                let sound = FormId(value1);
                ObjectModificationPropertyType::UnequipSound(sound)
            }
            ObjectModificationPropertyProperty::SoundLevel => {
                let level = SoundLevel::from_u32(value1)?;
                ObjectModificationPropertyType::SoundLevel(level)
            }
            ObjectModificationPropertyProperty::ImpactDataSet => {
                let set = FormId(value1);
                ObjectModificationPropertyType::ImpactDataSet(set)
            }
            ObjectModificationPropertyProperty::Ammo => {
                let ammo = FormId(value1);
                ObjectModificationPropertyType::Ammo(ammo)
            }
            ObjectModificationPropertyProperty::CritEffect => {
                let effect = FormId(value1);
                ObjectModificationPropertyType::CritEffect(effect)
            }
            ObjectModificationPropertyProperty::AimModelBaseStability => {
                let value = f32::from_bits(value1);
                ObjectModificationPropertyType::AimModelBaseStability(value)
            }
            ObjectModificationPropertyProperty::ZoomData => {
                let zoom = FormId(value1);
                ObjectModificationPropertyType::ZoomData(zoom)
            }
            ObjectModificationPropertyProperty::ZoomDataOverlay => {
                ObjectModificationPropertyType::ZoomDataOverlay
            }
            ObjectModificationPropertyProperty::ZoomDataImageSpace => {
                ObjectModificationPropertyType::ZoomDataImageSpace
            }
            ObjectModificationPropertyProperty::ZoomDataCameraOffsetX => {
                let value = f32::from_bits(value1);
                ObjectModificationPropertyType::ZoomDataCameraOffsetX(value)
            }
            ObjectModificationPropertyProperty::ZoomDataCameraOffsetY => {
                let value = f32::from_bits(value1);
                ObjectModificationPropertyType::ZoomDataCameraOffsetY(value)
            }
            ObjectModificationPropertyProperty::ZoomDataCameraOffsetZ => {
                let value = f32::from_bits(value1);
                ObjectModificationPropertyType::ZoomDataCameraOffsetZ(value)
            }
            ObjectModificationPropertyProperty::EquipSlot => {
                let slot = FormId(value1);
                ObjectModificationPropertyType::EquipSlot(slot)
            }
            ObjectModificationPropertyProperty::SoundLevelMult => {
                ObjectModificationPropertyType::SoundLevelMult
            }
            ObjectModificationPropertyProperty::NpcAmmoList => {
                let list = FormId(value1);
                ObjectModificationPropertyType::NpcAmmoList(list)
            }
            ObjectModificationPropertyProperty::ReloadSpeed => {
                let value = f32::from_bits(value1);
                ObjectModificationPropertyType::ReloadSpeed(value)
            }
            ObjectModificationPropertyProperty::DamageTypeValues => {
                let dmg_type = FormId(value1);
                let value = f32::from_bits(value2);
                ObjectModificationPropertyType::DamageTypeValues(dmg_type, value)
            }
            ObjectModificationPropertyProperty::AccuracyBonus => {
                ObjectModificationPropertyType::AccuracyBonus
            }
            ObjectModificationPropertyProperty::AttackActionPointCost => {
                let value = f32::from_bits(value1);
                ObjectModificationPropertyType::AttackActionPointCost(value)
            }
            ObjectModificationPropertyProperty::OverrideProjectile => {
                let projectile = FormId(value1);
                ObjectModificationPropertyType::OverrideProjectile(projectile)
            }
            ObjectModificationPropertyProperty::HasBoltAction => {
                let value = value1 != 0;
                ObjectModificationPropertyType::HasBoltAction(value)
            }
            ObjectModificationPropertyProperty::StaggerValue => {
                ObjectModificationPropertyType::StaggerValue
            }
            ObjectModificationPropertyProperty::SightedTransitionSeconds => {
                let value = f32::from_bits(value1);
                ObjectModificationPropertyType::SightedTransitionSeconds(value)
            }
            ObjectModificationPropertyProperty::FullPowerSeconds => {
                let value = f32::from_bits(value1);
                ObjectModificationPropertyType::FullPowerSeconds(value)
            }
            ObjectModificationPropertyProperty::HoldInputToPower => {
                let value = value1 != 0;
                ObjectModificationPropertyType::HoldInputToPower(value)
            }
            ObjectModificationPropertyProperty::HasRepeatableSingleFire => {
                let value = value1 != 0;
                ObjectModificationPropertyType::HasRepeatableSingleFire(value)
            }
            ObjectModificationPropertyProperty::MinPowerPerShot => {
                let value = f32::from_bits(value1);
                ObjectModificationPropertyType::MinPowerPerShot(value)
            }
            ObjectModificationPropertyProperty::CriticalDamageMult => {
                let value = f32::from_bits(value1);
                ObjectModificationPropertyType::CriticalDamageMult(value)
            }
            ObjectModificationPropertyProperty::FastEquipSound => {
                let sound = FormId(value1);
                ObjectModificationPropertyType::FastEquipSound(sound)
            }
            ObjectModificationPropertyProperty::DisableShells => {
                let value = value1 != 0;
                ObjectModificationPropertyType::DisableShells(value)
            }
            ObjectModificationPropertyProperty::HasChargingAttack => {
                let value = value1 != 0;
                ObjectModificationPropertyType::HasChargingAttack(value)
            }
            ObjectModificationPropertyProperty::Unknown => ObjectModificationPropertyType::Unknown,
        };

        Ok(Self {
            ty,
            function_type,
            step,
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, BinaryStruct)]
#[primitive(u8)]
enum ObjectModificationValueType {
    Int = 0,
    Float = 1,
    Bool = 2,
    String = 3,
    FormIdInt = 4,
    Enum = 5,
    FormIdFloat = 6,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ObjectModificationFunctionType {
    Set,
    MulAdd,
    Add,
    And,
    Or,
    Rem,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum ObjectModificationPropertyProperty {
    Enchantments,
    BashImpactDataSet,
    BlockMaterial,
    Keywords,
    Weight,
    Value,
    Rating,
    AddonIndex,
    BodyPart,
    DamageTypeValue,
    ActorValues,
    Health,
    ColorRemappingIndex,
    MaterialSwaps,
    ForcedInventory,
    XpOffset,
    Speed,
    Reach,
    MinRange,
    MaxRange,
    AttackDelaySec,
    OutOfRangeDamageMult,
    SecondaryDamage,
    CriticalChargeBonus,
    HitBehavior,
    Rank,
    AmmoCapacity,
    Type,
    IsPlayerOnly,
    NpcsUseAmmo,
    HasChargingReload,
    IsMinorCrime,
    IsFixedRange,
    HasEffectOnDeath,
    HasAlternateRumble,
    IsNonHostile,
    IgnoreResist,
    IsAutomatic,
    CantDrop,
    IsNonPlayable,
    AttackDamage,
    AimModel,
    AimModelMinConeDegrees,
    AimModelMaxConeDegrees,
    AimModelConeIncreasePerShot,
    AimModelConeDecreasePerSec,
    AimModelConeDecreaseDelayMs,
    AimModelConeSneakMultiplier,
    AimModelRecoilDiminishSpringForce,
    AimModelRecoilDiminishSightsMult,
    AimModelRecoilMaxDegPerShot,
    AimModelRecoilMinDegPerShot,
    AimModelRecoilHipMult,
    AimModelRecoilShotsForRunaway,
    AimModelRecoilArcDeg,
    AimModelRecoilArcRotateDeg,
    AimModelConeIronSightsMultiplier,
    HasScope,
    ZoomDataFovMult,
    FireSeconds,
    NumProjectiles,
    AttackSound,
    AttackSound2D,
    AttackLoop,
    AttackFailSound,
    IdleSound,
    EquipSound,
    UnequipSound,
    SoundLevel,
    ImpactDataSet,
    Ammo,
    CritEffect,
    AimModelBaseStability,
    ZoomData,
    ZoomDataOverlay,
    ZoomDataImageSpace,
    ZoomDataCameraOffsetX,
    ZoomDataCameraOffsetY,
    ZoomDataCameraOffsetZ,
    EquipSlot,
    SoundLevelMult,
    NpcAmmoList,
    ReloadSpeed,
    DamageTypeValues,
    AccuracyBonus,
    AttackActionPointCost,
    OverrideProjectile,
    HasBoltAction,
    StaggerValue,
    SightedTransitionSeconds,
    FullPowerSeconds,
    HoldInputToPower,
    HasRepeatableSingleFire,
    MinPowerPerShot,
    CriticalDamageMult,
    FastEquipSound,
    DisableShells,
    HasChargingAttack,
    Unknown,
}

/// The type of property that is being modified.
#[derive(Copy, Clone, Debug)]
pub enum ObjectModificationPropertyType {
    Enchantments(FormId),
    BashImpactDataSet,
    BlockMaterial,
    Keywords(FormId),
    Weight(f32),
    Value(f32),
    Rating(u32),
    AddonIndex(u32),
    BodyPart,
    DamageTypeValue(FormId, u32),
    ActorValues(FormId, f32),
    Health(f32),
    ColorRemappingIndex(f32),
    MaterialSwaps(FormId),
    ForcedInventory(FormId),
    XpOffset,
    Speed(f32),
    Reach,
    MinRange(f32),
    MaxRange(f32),
    AttackDelaySec(f32),
    OutOfRangeDamageMult(f32),
    SecondaryDamage(f32),
    CriticalChargeBonus(f32),
    HitBehavior,
    Rank,
    AmmoCapacity(u32),
    Type,
    IsPlayerOnly(bool),
    NpcsUseAmmo(bool),
    HasChargingReload(bool),
    IsMinorCrime(bool),
    IsFixedRange(bool),
    HasEffectOnDeath(bool),
    HasAlternateRumble(bool),
    IsNonHostile(bool),
    IgnoreResist(bool),
    IsAutomatic(bool),
    CantDrop(bool),
    IsNonPlayable(bool),
    AttackDamage(f32),
    AimModel(FormId),
    AimModelMinConeDegrees(f32),
    AimModelMaxConeDegrees(f32),
    AimModelConeIncreasePerShot(f32),
    AimModelConeDecreasePerSec(f32),
    AimModelConeDecreaseDelayMs(u32),
    AimModelConeSneakMultiplier(f32),
    AimModelRecoilDiminishSpringForce(f32),
    AimModelRecoilDiminishSightsMult(f32),
    AimModelRecoilMaxDegPerShot(f32),
    AimModelRecoilMinDegPerShot(f32),
    AimModelRecoilHipMult(f32),
    AimModelRecoilShotsForRunaway(f32),
    AimModelRecoilArcDeg(f32),
    AimModelRecoilArcRotateDeg(f32),
    AimModelConeIronSightsMultiplier(f32),
    HasScope(bool),
    ZoomDataFovMult(f32),
    FireSeconds(f32),
    NumProjectiles(f32),
    AttackSound(FormId),
    AttackSound2D(FormId),
    AttackLoop,
    AttackFailSound(FormId),
    IdleSound(FormId),
    EquipSound(FormId),
    UnequipSound(FormId),
    SoundLevel(SoundLevel),
    ImpactDataSet(FormId),
    Ammo(FormId),
    CritEffect(FormId),
    AimModelBaseStability(f32),
    ZoomData(FormId),
    ZoomDataOverlay,
    ZoomDataImageSpace,
    ZoomDataCameraOffsetX(f32),
    ZoomDataCameraOffsetY(f32),
    ZoomDataCameraOffsetZ(f32),
    EquipSlot(FormId),
    SoundLevelMult,
    NpcAmmoList(FormId),
    ReloadSpeed(f32),
    DamageTypeValues(FormId, f32),
    AccuracyBonus,
    AttackActionPointCost(f32),
    OverrideProjectile(FormId),
    HasBoltAction(bool),
    StaggerValue,
    SightedTransitionSeconds(f32),
    FullPowerSeconds(f32),
    HoldInputToPower(bool),
    HasRepeatableSingleFire(bool),
    MinPowerPerShot(f32),
    CriticalDamageMult(f32),
    FastEquipSound(FormId),
    DisableShells(bool),
    HasChargingAttack(bool),
    Unknown,
}
