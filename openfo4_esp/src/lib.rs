pub mod records;
pub mod vmad;

mod fields;

use fields::FieldMap;
use flate2::read::ZlibDecoder;
use glam::Vec3;
use hashbrown::HashMap;
use thiserror::Error;

use std::fmt::Debug;
use std::io::{self, ErrorKind, SeekFrom};
use std::io::{Read, Seek};
use std::mem::MaybeUninit;

use bytes::Buf;
use records::{
    AcousticSpace, Action, Activator, Ammo, Armor, Book, CMPO, Cell, Class, Climate, Container,
    DMGT, DOOR, ENCH, Empty, FACT, FLOR, Furniture, GLOB, GRAS, GameSetting, HDPT, Hazard, INGR,
    IdleMarker, Ingestible, Key, Keyword, Land, LandTexture, LeveledActor, LeveledItem, Light,
    LocationReferenceType, MSTT, MagicEffect, MiscObject, Note, Npc, PlacedGrenade, PlacedHazard,
    PlacedNpc, PlacedObject, Projectile, Race, SOUN, SPEL, STAT, ShaderParticleGeometry,
    StaticCollection, TACT, TRNS, Tes4, TextureSet, VisualEffect, Weather, Worldspace,
};

use crate::records::{
    ActorValueInfo, AddonNode, AimModel, AnimatedObject, Animation, AnimationSoundTagSet,
    ArmorAddon, ArtObject, AssocationType, AttractionRule, AudioCategorySnapshot, AudioEffectChain,
    BlendableSpline, BodyPartData, CameraPath, CameraShot, CollisionLayer, Color, CombatStyle,
    ConstructibleObject, Debris, DefaultObject, DefaultObjectManager, DialogueBranch, DialogueInfo,
    DialogueTopic, DialogueView, EffectShader, EncounterZone, EquipType, Explosion, Footstep,
    FootstepSet, FormList, GodRays, ImageSpace, ImageSpaceAdapter, Impact, ImpactDataSet,
    InstanceNamingRules, Layer, LensFlare, LightingTemplate, LoadScreen, Location, MaterialObject,
    MaterialSwap, MaterialType, Message, MovementType, MusicTrack, MusicType, NAVI,
    NavigationMeshObstacleManager, ObjectModification, ObjectVisibilityManager, Outfit, PackIn,
    Package, Perk, PlacedMissile, Quest, ReferenceGroup, Region, Relationship, ReverbParameters,
    Scene, SceneCollection, SoundCategory, SoundDescriptor, SoundKeywordMapping, SoundOutputModel,
    StoryManagerBranchNode, StoryManagerEventNode, StoryManagerQuestNode, Terminal, Tree,
    VoiceType, Water, Weapon, Zoom,
};

const MAX_INDEX_CAP: usize = 2 * 1024 * 1024;

#[derive(Debug, Error)]
#[error("error reading field {field:?}: {error}")]
pub struct RecordError {
    pub field: Text<4>,
    pub error: Error,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("eof")]
    Eof,
    #[error("bad magic")]
    BadMagic,
    #[error("missing required field: {0}")]
    MissingField(&'static str),
    #[error("missing required field: {0:?}")]
    MissingFieldText(Text<4>),
    #[error("bad enum value for {0}: {1}")]
    BadEnumValue(&'static str, u64),
    #[error(transparent)]
    Record(Box<RecordError>),
    #[error("invalid string: {0}")]
    InvalidString(std::string::FromUtf8Error),
    #[error("{0}")]
    Other(String),
    #[error("unknown record type: {0:?}")]
    UnknownRecordType(Text<4>),
    #[error("end of file")]
    EndOfFile,
}

#[derive(Debug)]
pub struct EspFile<R> {
    reader: R,
    /// Index table mapping [`FormId`]s that have been visited to file offsets.
    ///
    /// Linearly scanning the entire file on every lookup takes too long and we don't have
    /// any guarantees on the order of records in the file.
    ///
    /// It may seem that `GRUP` records are useful, however this still requires us to scan the
    /// entire file to find all `GRUP`s, which is still be too costly for good load times.
    ///
    /// Instead on every lookup we test if this table contains the offset for a record and on a
    /// hit directly seek to that location. On a miss we will start from the starting postion
    /// and scan all record headers until we find the record that we are looking for, while
    /// recording all records we find until then in the table.
    ///
    /// To prevent the table from becoming too large we exclude several types of [`RecordKind`]s
    /// that are known to not be accessed individually. In case the file is very very very large
    /// we also have a upper limit of [`MAX_INDEX_CAP`] on the table.
    //
    // Use a `u32` instead of a `u64` here, because hashbrown will store the entries
    // as `(K, V)` internally, meaning we only need 8 bytes per tuple instead of 16 bytes.
    //
    // It is unrealisitc for a file be larger than 4GiB, but in case that happens we
    // will simply not cache the records anymore, however the lookup still works correctly.
    index: HashMap<FormId, u32>,
}

impl<R> EspFile<R>
where
    R: Read + Seek,
{
    pub fn new(reader: R) -> Result<Self, Error> {
        let mut file = Self {
            reader,
            index: HashMap::new(),
        };

        let header = file.read_record_header()?;
        if header.typ != Text::TES4 {
            return Err(Error::BadMagic);
        }

        Ok(file)
    }

    fn read_record_header(&mut self) -> Result<RecordHeader, Error> {
        let mut buf = [0; RecordHeader::SIZE];
        match self.reader.read_exact(&mut buf) {
            Ok(()) => (),
            Err(err) if err.kind() == ErrorKind::UnexpectedEof => return Err(Error::EndOfFile),
            Err(err) => return Err(Error::Io(err)),
        }

        RecordHeader::decode(&buf[..])
    }

    /// Returns an `Iterator` over all [`Record`]s in this `EspFile` with the given `kind`, if any.
    pub fn lookup_all(&mut self, kind: Option<RecordKind>) -> LookupIter<'_, R> {
        LookupIter {
            file: self,
            kind,
            is_init: false,
            position: 0,
        }
    }

    /// Look up a [`Record`] by its [`FormId`].
    ///
    /// A [`RecordKind`] can be provided optionally and may increase seeking performance.
    pub fn lookup(&mut self, kind: Option<RecordKind>, id: FormId) -> Result<Record, Error> {
        let position = self.index.get(&id).copied().unwrap_or_default();
        self.reader.seek(SeekFrom::Start(position.into()))?;

        self.lookup_next_from_position(kind, Some(id), &mut position.into())
    }

    /// Search for the next [`Record`] matching the given criteria starting from the given
    /// position. The `position` value is updated as the cursor is advanced.
    ///
    /// When this function returns an [`Error`] the value of `position` is not specified.
    fn lookup_next_from_position(
        &mut self,
        kind: Option<RecordKind>,
        id: Option<FormId>,
        position: &mut u64,
    ) -> Result<Record, Error> {
        let mut do_seek = true;

        loop {
            let header_position = *position;

            // Seeking is quite expensive so we only do it when we actually
            // are skipping data.
            if do_seek {
                self.reader.seek(SeekFrom::Start(header_position))?;
                do_seek = false;
            }

            let header = self.read_record_header()?;
            *position += RecordHeader::SIZE as u64;

            if header.typ == Text::GRUP {
                let header = header.into_group();

                if let Some(kind) = kind {
                    // If the group does not match our record we can skip over the group.
                    // Note however that some records may appear as records in groups that
                    // are nested in other record. If this is the case we cannot skip the
                    // group.
                    // The `group_type` if 0 indicates that the group header includes the
                    // label.
                    if header.group_type == 0
                        && header.label() != kind.label()
                        && kind
                            .possible_parents()
                            .iter()
                            .all(|kind| kind.label() != header.label())
                    {
                        *position += u64::from(header.size);
                        do_seek = true;
                    }
                }

                continue;
            }

            if RecordKind::from_label(header.typ).is_some_and(|v| v.is_cachable())
                && self.index.len() < MAX_INDEX_CAP
            {
                if let Ok(header_position) = u32::try_from(header_position) {
                    self.index.insert(header.form_id, header_position);
                }
            }

            if id.is_some_and(|id| header.form_id != id)
                || kind.is_some_and(|kind| header.typ != kind.label())
            {
                *position += u64::from(header.size);
                do_seek = true;
                continue;
            }

            let data = self.read_record_data(header)?;
            *position += u64::from(header.size);

            return Ok(Record {
                id: header.form_id,
                flags: header.flags,
                data,
            });
        }
    }

    pub fn children(&mut self, kind: RecordChildren, id: FormId) -> Result<Vec<Record>, Error> {
        // Lookup will scan exactly to the end of the record.
        // The next record will be a GRUP with the children if
        // the record has children.
        let mut start_pos = self.index.get(&id).copied().unwrap_or_default().into();
        self.lookup_next_from_position(Some(kind.parent_kind()), Some(id), &mut start_pos)?;
        self.reader.seek(SeekFrom::Start(start_pos))?;

        let header = self.read_record_header()?;
        if header.typ != Text::GRUP {
            return Ok(Vec::new());
        }

        loop {
            match kind {
                RecordChildren::Cells => {
                    let mut cells = Vec::new();

                    let mut bytes_remaining = header.into_group().size;
                    while bytes_remaining > 0 {
                        let header = self.read_record_header()?;
                        bytes_remaining = bytes_remaining.saturating_sub(RecordHeader::SIZE as u32);

                        // Skip over the children of the cell.
                        if header.typ == Text::GRUP {
                            let header = header.into_group();
                            if header.group_type == 8 || header.group_type == 9 {
                                self.reader.seek(SeekFrom::Current(header.size.into()))?;
                                bytes_remaining = bytes_remaining.saturating_sub(header.size);
                            }

                            continue;
                        }

                        let data = self.read_record_data(header)?;
                        cells.push(Record {
                            id: header.form_id,
                            flags: header.flags,
                            data,
                        });

                        bytes_remaining = bytes_remaining.saturating_sub(header.size);
                    }

                    return Ok(cells);
                }
                RecordChildren::PersistentEntities | RecordChildren::TemporaryEntities => {
                    let header = match self.read_record_header() {
                        Ok(header) => header,
                        Err(Error::EndOfFile) => return Ok(Vec::new()),
                        Err(err) => return Err(err),
                    };

                    // It is not clear if GRUPs are required or optional.
                    // There are some records with only a temporary group
                    // but there are no cells with no groups whatsoever.
                    if header.typ != Text::GRUP {
                        return Ok(Vec::new());
                    }

                    let group_type = match kind {
                        RecordChildren::PersistentEntities => 8,
                        RecordChildren::TemporaryEntities => 9,
                        _ => unreachable!(),
                    };

                    let header = header.into_group();
                    if header.group_type != group_type {
                        self.reader.seek(SeekFrom::Current(header.size.into()))?;
                        continue;
                    }

                    return self.read_records(header);
                }
            }
        }
    }

    fn read_records(&mut self, header: GroupHeader) -> Result<Vec<Record>, Error> {
        let mut bytes_remaining = header.size;

        let mut records = Vec::new();

        while bytes_remaining > 0 {
            let header = self.read_record_header()?;

            let data = self.read_record_data(header)?;
            records.push(Record {
                id: header.form_id,
                flags: header.flags,
                data,
            });

            bytes_remaining = bytes_remaining.saturating_sub(RecordHeader::SIZE as u32);
            bytes_remaining = bytes_remaining.saturating_sub(header.size);
        }

        Ok(records)
    }

    fn read_record_data(&mut self, header: RecordHeader) -> Result<RecordData, Error> {
        debug_assert_ne!(header.typ, Text::GRUP);

        let mut buf = vec![0; header.size as usize];
        self.reader.read_exact(&mut buf)?;

        if header.flags.contains(RecordHeaderFlags::DATA_COMPRESSED) {
            let mut input = &buf[..];
            let decompressed_size = u32::decode(&mut input)?;
            let mut decoder = ZlibDecoder::new(input);

            let mut output = Vec::with_capacity(decompressed_size as usize);
            decoder.read_to_end(&mut output)?;
            buf = output;
        }

        let mut map = FieldMap::decode(&buf[..])?;
        macro_rules! read_data {
            ($($kind:tt => $data:tt),*$(,)?) => {
                match RecordKind::from_label(header.typ) {
                    $(
                        Some(RecordKind::$kind) => fields::Read::read(&mut map, Text([0; 4])).map(RecordData::$data),
                    )*
                    None => return Err(Error::UnknownRecordType(header.typ)),
                }
            };
        }

        read_data! {
            Tes4 => Tes4,
            GameSetting => GameSetting,
            Keyword => Keyword,
            LocationReferenceType => LocationReferenceType,
            Action => Action,
            Transform => TRNS,
            Component => CMPO,
            TextureSet => TextureSet,
            GlobalVariable => GLOB,
            DamageType => DMGT,
            Class => Class,
            Faction => FACT,
            HeadPart => HDPT,
            Race => Race,
            Sound => SOUN,
            AcousticSpace => AcousticSpace,
            MagicEffect => MagicEffect,
            LandTexture => LandTexture,
            Enchantment => ENCH,
            Spell => SPEL,
            Activator => Activator,
            TalkingActivator => TACT,
            Armor => Armor,
            Book => Book,
            Container => Container,
            Door => DOOR,
            Ingredient => INGR,
            Light => Light,
            MiscObject => MiscObject,
            Static => STAT,
            StaticCollection => StaticCollection,
            MovableStatic => MSTT,
            Grass => GRAS,
            Flora => FLOR,
            Furniture => Furniture,
            Weapon => Weapon,
            Ammo => Ammo,
            Npc => Npc,
            LeveledActor => LeveledActor,
            Key => Key,
            Ingestible => Ingestible,
            IdleMarker => IdleMarker,
            Note => Note,
            Projectile => Projectile,
            Hazard => Hazard,
            BlendableSpline => BlendableSpline,
            Terminal => Terminal,
            Tree => Tree,
            LeveledItem => LeveledItem,
            Weather => Weather,
            Climate => Climate,
            ShaderParticleGeometry => ShaderParticleGeometry,
            VisualEffect => VisualEffect,
            Region => Region,
            NavMeshInfo => NavMeshInfo,
            Cell => Cell,
            PlacedNpc => PlacedNpc,
            PlacedObject => PlacedObject,
            NavMesh => NavMesh,
            PlacedGrenade => PlacedGrenade,
            PlacedHazard => PlacedHazard,
            Worldspace => Worldspace,
            Land => Land,
            PlacedMissile => PlacedMissile,
            Quest => Quest,
            DialogueBranch => DialogueBranch,
            DialogueTopic => DialogueTopic,
            Scene => Scene,
            DialogueInfo => DialogueInfo,
            Animation => Animation,
            Package => Package,
            CombatStyle => CombatStyle,
            LoadScreen => LoadScreen,
            AnimatedObject => AnimatedObject,
            Water => Water,
            EffectShader => EffectShader,
            Explosion => Explosion,
            Debris => Debris,
            ImageSpace => ImageSpace,
            ImageSpaceAdapter => ImageSpaceAdapter,
            FormList => FormList,
            Perk => Perk,
            BodyPartData => BodyPartData,
            AddonNode => AddonNode,
            ActorValueInfo => ActorValueInfo,
            CameraShot => CameraShot,
            CameraPath => CameraPath,
            VoiceType => VoiceType,
            MaterialType => MaterialType,
            Impact => Impact,
            ImpactDataSet => ImpactDataSet,
            ArmorAddon => ArmorAddon,
            EncounterZone => EncounterZone,
            Location => Location,
            Message => Message,
            DefaultObjectManager => DefaultObjectManager,
            DefaultObject => DefaultObject,
            LightingTemplate => LightingTemplate,
            MusicType => MusicType,
            Footstep => Footstep,
            FootstepSet => FootstepSet,
            StoryManagerBranchNode => StoryManagerBranchNode,
            StoryManagerQuestNode => StoryManagerQuestNode,
            StoryManagerEventNode => StoryManagerEventNode,
            MusicTrack => MusicTrack,
            DialogueView => DialogueView,
            EquipType => EquipType,
            Relationship => Relationship,
            AssocationType => AssocationType,
            Outfit => Outfit,
            ArtObject => ArtObject,
            MaterialObject => MaterialObject,
            MovementType => MovementType,
            SoundDescriptor => SoundDescriptor,
            SoundCategory => SoundCategory,
            SoundOutputModel => SoundOutputModel,
            CollisionLayer => CollisionLayer,
            Color => Color,
            ReverbParameters => ReverbParameters,
            PackIn => PackIn,
            ReferenceGroup => ReferenceGroup,
            AimModel => AimModel,
            Layer => Layer,
            ConstructibleObject => ConstructibleObject,
            ObjectModification => ObjectModification,
            MaterialSwap => MaterialSwap,
            Zoom => Zoom,
            InstanceNamingRules => InstanceNamingRules,
            SoundKeywordMapping => SoundKeywordMapping,
            AudioEffectChain => AudioEffectChain,
            SceneCollection => SceneCollection,
            AttractionRule => AttractionRule,
            AudioCategorySnapshot => AudioCategorySnapshot,
            AnimationSoundTagSet => AnimationSoundTagSet,
            NavigationMeshObstacleManager => NavigationMeshObstacleManager,
            LensFlare => LensFlare,
            GodRays => GodRays,
            ObjectVisibilityManager => ObjectVisibilityManager,
        }
    }
}

#[derive(Debug)]
pub struct LookupIter<'a, R> {
    file: &'a mut EspFile<R>,
    /// Filter for the searched records.
    kind: Option<RecordKind>,
    is_init: bool,
    position: u64,
}

impl<'a, R> Iterator for LookupIter<'a, R>
where
    R: Read + Seek,
{
    type Item = Result<Record, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.is_init {
            self.is_init = true;

            // Seek to the start of the file.
            // We always need to do a deep scan of the entire file if we
            // want to be sure that we have found all records of a given type.
            if let Err(err) = self.file.reader.seek(SeekFrom::Start(0)) {
                return Some(Err(Error::Io(err)));
            }
        }

        match self
            .file
            .lookup_next_from_position(self.kind, None, &mut self.position)
        {
            Ok(value) => Some(Ok(value)),
            Err(Error::EndOfFile) => None,
            Err(err) => Some(Err(err)),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum RecordChildren {
    /// [`Cell`]s of a [`Worldspace`].
    Cells,
    PersistentEntities,
    TemporaryEntities,
}

impl RecordChildren {
    fn parent_kind(&self) -> RecordKind {
        match self {
            Self::Cells => RecordKind::Worldspace,
            Self::PersistentEntities => RecordKind::Cell,
            Self::TemporaryEntities => RecordKind::Cell,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Record {
    pub id: FormId,
    pub flags: RecordHeaderFlags,
    pub data: RecordData,
}

#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum RecordData {
    Tes4(Tes4),
    GameSetting(GameSetting),
    Keyword(Keyword),
    LocationReferenceType(LocationReferenceType),
    Action(Action),
    TRNS(TRNS),
    CMPO(CMPO),
    TextureSet(TextureSet),
    GLOB(GLOB),
    DMGT(DMGT),
    Class(Class),
    FACT(FACT),
    HDPT(HDPT),
    Race(Race),
    SOUN(SOUN),
    AcousticSpace(AcousticSpace),
    MagicEffect(MagicEffect),
    LandTexture(LandTexture),
    ENCH(ENCH),
    SPEL(SPEL),
    Activator(Activator),
    TACT(TACT),
    Armor(Armor),
    Book(Book),
    Container(Container),
    DOOR(DOOR),
    INGR(INGR),
    Light(Light),
    MiscObject(MiscObject),
    STAT(STAT),
    StaticCollection(StaticCollection),
    MSTT(MSTT),
    GRAS(GRAS),
    FLOR(FLOR),
    Furniture(Furniture),
    Weapon(Weapon),
    Ammo(Ammo),
    Npc(Npc),
    LeveledActor(LeveledActor),
    Key(Key),
    Ingestible(Ingestible),
    IdleMarker(IdleMarker),
    Note(Note),
    Projectile(Projectile),
    Hazard(Hazard),
    BlendableSpline(BlendableSpline),
    Terminal(Terminal),
    Tree(Tree),
    LeveledItem(LeveledItem),
    Weather(Weather),
    Climate(Climate),
    ShaderParticleGeometry(ShaderParticleGeometry),
    VisualEffect(VisualEffect),
    Region(Region),
    NavMeshInfo(NAVI),
    Cell(Cell),
    PlacedNpc(PlacedNpc),
    PlacedObject(PlacedObject),
    NavMesh(Empty),
    PlacedGrenade(PlacedGrenade),
    PlacedHazard(PlacedHazard),
    Worldspace(Worldspace),
    Land(Land),
    PlacedMissile(PlacedMissile),
    Quest(Quest),
    DialogueBranch(DialogueBranch),
    DialogueTopic(DialogueTopic),
    Scene(Scene),
    DialogueInfo(DialogueInfo),
    Animation(Animation),
    Package(Package),
    CombatStyle(CombatStyle),
    LoadScreen(LoadScreen),
    AnimatedObject(AnimatedObject),
    Water(Water),
    EffectShader(EffectShader),
    Explosion(Explosion),
    Debris(Debris),
    ImageSpace(ImageSpace),
    ImageSpaceAdapter(ImageSpaceAdapter),
    FormList(FormList),
    Perk(Perk),
    BodyPartData(BodyPartData),
    AddonNode(AddonNode),
    ActorValueInfo(ActorValueInfo),
    CameraShot(CameraShot),
    CameraPath(CameraPath),
    VoiceType(VoiceType),
    MaterialType(MaterialType),
    Impact(Impact),
    ImpactDataSet(ImpactDataSet),
    ArmorAddon(ArmorAddon),
    EncounterZone(EncounterZone),
    Location(Location),
    Message(Message),
    DefaultObjectManager(DefaultObjectManager),
    DefaultObject(DefaultObject),
    LightingTemplate(LightingTemplate),
    MusicType(MusicType),
    Footstep(Footstep),
    FootstepSet(FootstepSet),
    StoryManagerBranchNode(StoryManagerBranchNode),
    StoryManagerQuestNode(StoryManagerQuestNode),
    StoryManagerEventNode(StoryManagerEventNode),
    MusicTrack(MusicTrack),
    DialogueView(DialogueView),
    EquipType(EquipType),
    Relationship(Relationship),
    AssocationType(AssocationType),
    Outfit(Outfit),
    ArtObject(ArtObject),
    MaterialObject(MaterialObject),
    MovementType(MovementType),
    SoundDescriptor(SoundDescriptor),
    SoundCategory(SoundCategory),
    SoundOutputModel(SoundOutputModel),
    CollisionLayer(CollisionLayer),
    Color(Color),
    ReverbParameters(ReverbParameters),
    PackIn(PackIn),
    ReferenceGroup(ReferenceGroup),
    AimModel(AimModel),
    Layer(Layer),
    ConstructibleObject(ConstructibleObject),
    ObjectModification(ObjectModification),
    MaterialSwap(MaterialSwap),
    Zoom(Zoom),
    InstanceNamingRules(InstanceNamingRules),
    SoundKeywordMapping(SoundKeywordMapping),
    AudioEffectChain(AudioEffectChain),
    SceneCollection(SceneCollection),
    AttractionRule(AttractionRule),
    AudioCategorySnapshot(AudioCategorySnapshot),
    AnimationSoundTagSet(AnimationSoundTagSet),
    NavigationMeshObstacleManager(NavigationMeshObstacleManager),
    LensFlare(LensFlare),
    GodRays(GodRays),
    ObjectVisibilityManager(ObjectVisibilityManager),
}

impl RecordData {
    pub fn as_variant<T>(&self) -> Option<&T>
    where
        T: AsRecordKind,
    {
        T::from_data(self)
    }

    pub fn into_variant<T>(self) -> Option<T>
    where
        T: AsRecordKind,
    {
        T::from_data_owned(self)
    }
}

pub trait AsRecordKind: Sized {
    const KIND: RecordKind;

    #[doc(hidden)]
    fn from_data(data: &RecordData) -> Option<&Self>;

    #[doc(hidden)]
    fn from_data_owned(data: RecordData) -> Option<Self>;
}

macro_rules! as_record_kind_impl {
    ($($t:tt => $kind:expr),*,) => {
        $(
            impl AsRecordKind for $t {
                const KIND: RecordKind = $kind;

                fn from_data(data: &RecordData) -> Option<&Self> {
                    match data {
                        RecordData::$t(v) => Some(v),
                        _ => None,
                    }
                }

                fn from_data_owned(data: RecordData) -> Option<Self> {
                    match data {
                        RecordData::$t(v) => Some(v),
                        _ => None,
                    }
                }
            }
        )*
    };
}

as_record_kind_impl! {
    Tes4 => RecordKind::Tes4,
    GameSetting => RecordKind::GameSetting,
    Keyword => RecordKind::Keyword,
    LocationReferenceType => RecordKind::LocationReferenceType,
    Action => RecordKind::Action,
    TRNS => RecordKind::Transform,
    CMPO => RecordKind::Component,
    TextureSet => RecordKind::TextureSet,
    GLOB => RecordKind::GlobalVariable,
    DMGT => RecordKind::DamageType,
    Class => RecordKind::Class,
    FACT => RecordKind::Faction,
    HDPT => RecordKind::HeadPart,
    Race => RecordKind::Race,
    SOUN => RecordKind::Sound,
    AcousticSpace => RecordKind::AcousticSpace,
    MagicEffect => RecordKind::MagicEffect,
    LandTexture => RecordKind::LandTexture,
    ENCH => RecordKind::Enchantment,
    SPEL => RecordKind::Spell,
    Activator => RecordKind::Activator,
    TACT => RecordKind::TalkingActivator,
    Armor => RecordKind::Armor,
    Book => RecordKind::Book,
    Container => RecordKind::Container,
    INGR => RecordKind::Ingredient,
    Light => RecordKind::Light,
    MiscObject => RecordKind::MiscObject,
    STAT => RecordKind::Static,
    StaticCollection => RecordKind::StaticCollection,
    MSTT => RecordKind::MovableStatic,
    GRAS => RecordKind::Grass,
    FLOR => RecordKind::Flora,
    Furniture => RecordKind::Furniture,
    Ammo => RecordKind::Ammo,
    Npc => RecordKind::Npc,
    LeveledActor => RecordKind::LeveledActor,
    Key => RecordKind::Key,
    Ingestible => RecordKind::Ingestible,
    IdleMarker => RecordKind::IdleMarker,
    Note => RecordKind::Note,
    Projectile => RecordKind::Projectile,
    Hazard => RecordKind::Hazard,
    LeveledItem => RecordKind::LeveledItem,
    Weather => RecordKind::Weather,
    Climate => RecordKind::Climate,
    ShaderParticleGeometry => RecordKind::ShaderParticleGeometry,
    VisualEffect => RecordKind::VisualEffect,
    Cell => RecordKind::Cell,
    PlacedNpc => RecordKind::PlacedNpc,
    PlacedObject => RecordKind::PlacedObject,
    // NAVMESH
    PlacedGrenade => RecordKind::PlacedGrenade,
    PlacedHazard => RecordKind::PlacedHazard,
    Worldspace => RecordKind::Worldspace,
    Land => RecordKind::Land,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RecordKind {
    Tes4,
    GameSetting,
    Keyword,
    LocationReferenceType,
    Action,
    Transform,
    Component,
    TextureSet,
    GlobalVariable,
    DamageType,
    Class,
    Faction,
    HeadPart,
    Race,
    Sound,
    AcousticSpace,
    MagicEffect,
    LandTexture,
    Enchantment,
    Spell,
    Activator,
    TalkingActivator,
    Armor,
    Book,
    Container,
    Door,
    Ingredient,
    Light,
    MiscObject,
    Static,
    StaticCollection,
    MovableStatic,
    Grass,
    Flora,
    Furniture,
    Weapon,
    Ammo,
    Npc,
    LeveledActor,
    Key,
    Ingestible,
    IdleMarker,
    Note,
    Projectile,
    Hazard,
    BlendableSpline,
    Terminal,
    Tree,
    LeveledItem,
    Weather,
    Climate,
    ShaderParticleGeometry,
    VisualEffect,
    Region,
    NavMeshInfo,
    Cell,
    PlacedNpc,
    PlacedObject,
    NavMesh,
    PlacedGrenade,
    PlacedHazard,
    Worldspace,
    Land,
    PlacedMissile,
    Quest,
    DialogueBranch,
    DialogueTopic,
    Scene,
    DialogueInfo,
    Animation,
    Package,
    CombatStyle,
    LoadScreen,
    AnimatedObject,
    Water,
    EffectShader,
    Explosion,
    Debris,
    ImageSpace,
    ImageSpaceAdapter,
    FormList,
    Perk,
    BodyPartData,
    AddonNode,
    ActorValueInfo,
    CameraShot,
    CameraPath,
    VoiceType,
    MaterialType,
    Impact,
    ImpactDataSet,
    ArmorAddon,
    EncounterZone,
    Location,
    Message,
    DefaultObjectManager,
    DefaultObject,
    LightingTemplate,
    MusicType,
    Footstep,
    FootstepSet,
    StoryManagerBranchNode,
    StoryManagerQuestNode,
    StoryManagerEventNode,
    MusicTrack,
    DialogueView,
    EquipType,
    Relationship,
    AssocationType,
    Outfit,
    ArtObject,
    MaterialObject,
    MovementType,
    SoundDescriptor,
    SoundCategory,
    SoundOutputModel,
    CollisionLayer,
    Color,
    ReverbParameters,
    PackIn,
    ReferenceGroup,
    AimModel,
    Layer,
    ConstructibleObject,
    ObjectModification,
    MaterialSwap,
    Zoom,
    InstanceNamingRules,
    SoundKeywordMapping,
    AudioEffectChain,
    SceneCollection,
    AttractionRule,
    AudioCategorySnapshot,
    AnimationSoundTagSet,
    NavigationMeshObstacleManager,
    LensFlare,
    GodRays,
    ObjectVisibilityManager,
}

impl RecordKind {
    fn possible_parents(&self) -> &'static [Self] {
        match self {
            Self::Cell => &[Self::Worldspace],
            Self::PlacedNpc | Self::PlacedObject | Self::PlacedHazard | Self::PlacedGrenade => {
                &[Self::Worldspace, Self::Cell]
            }
            Self::DialogueBranch | Self::DialogueTopic | Self::Scene => &[Self::Quest],
            Self::DialogueInfo => &[Self::DialogueTopic],
            _ => &[],
        }
    }

    fn possible_children(&self) -> &'static [Self] {
        match self {
            Self::Worldspace => &[
                Self::Cell,
                Self::PlacedNpc,
                Self::PlacedObject,
                Self::PlacedHazard,
                Self::PlacedGrenade,
                Self::Land,
                Self::NavMesh,
            ],
            Self::Cell => &[
                Self::PlacedNpc,
                Self::PlacedObject,
                Self::PlacedHazard,
                Self::PlacedGrenade,
                Self::Land,
                Self::NavMesh,
            ],
            Self::Quest => &[Self::DialogueBranch, Self::DialogueTopic, Self::Scene],
            Self::DialogueTopic => &[Self::DialogueInfo],
            _ => &[],
        }
    }

    /// Returns `true` if a record with the given key MAY be cached.
    fn is_cachable(&self) -> bool {
        match self {
            // These records are most the majority of records
            // in most big ESP files.
            // In additional these records are children of CELL
            // records, which are always loaded in batch.
            // Placed records are usually not loaded individually,
            // so not caching them has little effect on access time.
            Self::PlacedNpc => false,
            Self::PlacedObject => false,
            Self::PlacedGrenade => false,
            Self::PlacedHazard => false,
            Self::PlacedMissile => false,
            _ => true,
        }
    }
}

macro_rules! record_kind_label {
    ($($a:tt => $b:expr),* $(,)?) => {
        impl RecordKind {
            fn label(&self) -> Text<4> {
                match self {
                    $(
                        Self::$a => {
                            const { assert!($b.as_bytes().len() == 4); }
                            Text($b.as_bytes().try_into().unwrap())
                        }
                    )*
                }
            }

            fn from_label(label: Text<4>) -> Option<Self> {
                match &label.0 {
                    $(
                        val if val == $b.as_bytes() => Some(Self::$a),
                    )*
                    _ => None,
                }
            }
        }
    };
}

record_kind_label! {
    Tes4 => "TES4",
    GameSetting => "GMST",
    Keyword => "KYWD",
    LocationReferenceType => "LCRT",
    Action => "AACT",
    Transform => "TRNS",
    Component => "CMPO",
    TextureSet => "TXST",
    GlobalVariable => "GLOB",
    DamageType => "DMGT",
    Class => "CLAS",
    Faction => "FACT",
    HeadPart => "HDPT",
    Race => "RACE",
    Sound => "SOUN",
    AcousticSpace => "ASPC",
    MagicEffect => "MGEF",
    LandTexture => "LTEX",
    Enchantment => "ENCH",
    Spell => "SPEL",
    Activator => "ACTI",
    TalkingActivator => "TACT",
    Armor => "ARMO",
    Book => "BOOK",
    Container => "CONT",
    Door => "DOOR",
    Ingredient => "INGR",
    Light => "LIGH",
    MiscObject => "MISC",
    Static => "STAT",
    StaticCollection => "SCOL",
    MovableStatic => "MSTT",
    Grass => "GRAS",
    Flora => "FLOR",
    Furniture => "FURN",
    Weapon => "WEAP",
    Ammo => "AMMO",
    Npc => "NPC_",
    LeveledActor => "LVLN",
    Key => "KEYM",
    Ingestible => "ALCH",
    IdleMarker =>  "IDLM",
    Note => "NOTE",
    Projectile => "PROJ",
    Hazard => "HAZD",
    BlendableSpline => "BNDS",
    Terminal => "TERM",
    Tree => "TREE",
    LeveledItem => "LVLI",
    Weather => "WTHR",
    Climate => "CLMT",
    ShaderParticleGeometry => "SPGD",
    VisualEffect => "RFCT",
    Region => "REGN",
    NavMeshInfo => "NAVI",
    Cell => "CELL",
    PlacedNpc => "ACHR",
    PlacedObject => "REFR",
    NavMesh => "NAVM",
    PlacedGrenade => "PGRE",
    PlacedHazard => "PHZD",
    Worldspace => "WRLD",
    Land => "LAND",
    PlacedMissile => "PMIS",
    Quest => "QUST",
    DialogueBranch => "DLBR",
    DialogueTopic => "DIAL",
    Scene => "SCEN",
    DialogueInfo => "INFO",
    Animation => "IDLE",
    Package => "PACK",
    CombatStyle => "CSTY",
    LoadScreen => "LSCR",
    AnimatedObject => "ANIO",
    Water => "WATR",
    EffectShader => "EFSH",
    Explosion => "EXPL",
    Debris => "DEBR",
    ImageSpace => "IMGS",
    ImageSpaceAdapter => "IMAD",
    FormList => "FLST",
    Perk => "PERK",
    BodyPartData => "BPTD",
    AddonNode => "ADDN",
    ActorValueInfo => "AVIF",
    CameraShot => "CAMS",
    CameraPath => "CPTH",
    VoiceType => "VTYP",
    MaterialType => "MATT",
    Impact => "IPCT",
    ImpactDataSet => "IPDS",
    ArmorAddon => "ARMA",
    EncounterZone => "ECZN",
    Location => "LCTN",
    Message => "MESG",
    DefaultObjectManager => "DOBJ",
    DefaultObject => "DFOB",
    LightingTemplate => "LGTM",
    MusicType => "MUSC",
    Footstep => "FSTP",
    FootstepSet => "FSTS",
    StoryManagerBranchNode => "SMBN",
    StoryManagerQuestNode => "SMQN",
    StoryManagerEventNode => "SMEN",
    MusicTrack => "MUST",
    DialogueView => "DLVW",
    EquipType => "EQUP",
    Relationship => "RELA",
    AssocationType => "ASTP",
    Outfit => "OTFT",
    ArtObject => "ARTO",
    MaterialObject => "MATO",
    MovementType => "MOVT",
    SoundDescriptor => "SNDR",
    SoundCategory => "SNCT",
    SoundOutputModel => "SOPM",
    CollisionLayer => "COLL",
    Color => "CLFM",
    ReverbParameters => "REVB",
    PackIn => "PKIN",
    ReferenceGroup => "RFGP",
    AimModel => "AMDL",
    Layer => "LAYR",
    ConstructibleObject => "COBJ",
    ObjectModification => "OMOD",
    MaterialSwap => "MSWP",
    Zoom => "ZOOM",
    InstanceNamingRules => "INNR",
    SoundKeywordMapping => "KSSM",
    AudioEffectChain => "AECH",
    SceneCollection => "SCCO",
    AttractionRule => "AORU",
    AudioCategorySnapshot => "SCSN",
    AnimationSoundTagSet => "STAG",
    NavigationMeshObstacleManager => "NOCM",
    LensFlare => "LENS",
    GodRays => "GDRY",
    ObjectVisibilityManager => "OVIS",
}

enum Group {
    ACCT,
    ACHR,
    ACTI,
    ADDN,
    ALCH,
    AMMO,
    ANIO,
    APPA,
    ARMA,
    ARMO,
    ARTO,
    ASPC,
    ASTP,
    AVIF,
    BOOK,
    BPTD,
    CAMS,
    CELL,
    CLAS,
    CLFM,
    CLMT,
    COBJ,
    COLL,
    CONT,
    CPTH,
    CSTY,
    DEBR,
    DIAL,
    DLBR,
    VLVW,
    DOBJ,
}

pub trait Decode: Sized {
    type Error;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf;
}

impl Decode for () {
    type Error = Error;

    fn decode<B>(_buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        Ok(())
    }
}

macro_rules! impl_primitive {
    ($($t:ty),*$(,)?) => {
        $(

            impl Decode for $t {
                type Error = Error;

                fn decode<B>(buf: B) -> Result<Self, Self::Error>
                where
                    B: Buf,
                {
                    <[u8; size_of::<Self>()]>::decode(buf).map(Self::from_le_bytes)
                }
            }
        )*
    };
}

impl Decode for u8 {
    type Error = Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        if buf.has_remaining() {
            Ok(buf.get_u8())
        } else {
            Err(Error::Eof)
        }
    }
}

impl<T, const N: usize> Decode for [T; N]
where
    T: Decode,
    Error: From<T::Error>,
{
    type Error = Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        struct DropGuard<T, const N: usize>([MaybeUninit<T>; N], usize);
        impl<T, const N: usize> Drop for DropGuard<T, N> {
            fn drop(&mut self) {
                for index in 0..self.1 {
                    unsafe {
                        self.0[index].assume_init_drop();
                    }
                }
            }
        }

        let mut arr = DropGuard([const { MaybeUninit::uninit() }; N], 0);
        for index in 0..N {
            let elem = T::decode(&mut buf)?;
            arr.0[index].write(elem);
            arr.1 += 1;
        }

        // Reset len, so that no elements will be dropped.
        arr.1 = 0;

        Ok(unsafe { core::mem::transmute_copy::<[MaybeUninit<T>; N], [T; N]>(&arr.0) })
    }
}

impl Decode for bool {
    type Error = Error;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        u8::decode(buf).map(|v| v != 0)
    }
}

impl_primitive! {
    u16,
    u32,
    u64,
    i8,
    i16,
    i32,
    i64,
    f32,
    f64,
}

macro_rules! impl_struct {
    (
        $(#[$($attrs:tt)*])*
        struct $struct_ident:ident {
        $(
            $field_name:ident: $field_ty:ty,
        )*
    }) => {
        $(
            #[$($attrs)*]
        )*
        struct $struct_ident {
            $(
                $field_name: $field_ty,
            )*
        }

        impl Decode for $struct_ident {
            type Error = ();

            fn decode<R>(mut reader: R) -> Result<Self, Self::Error>
            where
                R: Reader,
            {
                Ok(Self {
                $(
                    $field_name: <$field_ty as Decode>::decode(&mut reader)?,
                )*
                })
            }
        }
    };
}

#[derive(Copy, Clone, Debug)]
struct GroupHeader {
    size: u32,
    label: [u8; 4],
    group_type: u32,
}

impl GroupHeader {
    fn label(&self) -> Text<4> {
        Text(self.label)
        // match self.group_type {
        //     0 => Text(self.label),
        //     2 | 3 | 4 | 5 => Text::CELL,
        // }
    }
}

#[derive(Copy, Clone, Debug)]
struct RecordHeader {
    typ: Text<4>,
    size: u32,
    flags: RecordHeaderFlags,
    form_id: FormId,
    timestamp: u16,
    vcs: u16,
    version: u16,
    unknown0: u16,
}

impl RecordHeader {
    /// Fixed size of the header.
    const SIZE: usize = 24;

    fn into_group(self) -> GroupHeader {
        debug_assert_eq!(self.typ, Text::GRUP);

        GroupHeader {
            // For GRUP records the size field includes
            // the size of the header.
            size: self.size.saturating_sub(Self::SIZE as u32),
            label: self.flags.bits().to_le_bytes(),
            group_type: self.form_id.0,
        }
    }
}

impl Decode for RecordHeader {
    type Error = Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        Ok(Self {
            typ: Text::<4>::decode(&mut buf)?,
            size: u32::decode(&mut buf)?,
            flags: u32::decode(&mut buf).map(RecordHeaderFlags::from_bits_retain)?,
            form_id: FormId::decode(&mut buf)?,
            timestamp: u16::decode(&mut buf)?,
            vcs: u16::decode(&mut buf)?,
            version: u16::decode(&mut buf)?,
            unknown0: u16::decode(&mut buf)?,
        })
    }
}

bitflags::bitflags! {
    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
    pub struct RecordHeaderFlags: u32 {
        const TES4_MASTER_FILE = 0x00000001;
        const DELETED_GROUP = 0x00000010;
        const DELETED_RECORD = 0x00000020;
        const GLOB_CONSTANT = 0x00000040;
        const REFR_HIDDEN_FROM_LOCAL_MAP = 0x00000040;
        const LOCALIZED = 0x00000080;
        const MUST_UPDATE_ANIMS = 0x00000100;
        const REFR_INACESSIBLE = 0x00000100;
        const TES4_LIGHT_MASTER_FILE = 0x00000200;
        const REFR_HIDDEN_FROM_LOCAL_MAP2 = 0x00000200;
        const QUEST_ITEM = 0x00000400;
        const PERSISTENT_REFERENCE = 0x00000400;
        const LSCR_DISPLAYS_IN_MAIN_MENU = 0x00000400;
        const INITIALLY_DISABLED = 0x00000800;
        const IGNORED = 0x00001000;
        const VISIBLE_WHEN_DISTANT = 0x00008000;
        const ACTI_RANDOM_ANIMATION_START = 0x00010000;
        const ACTI_DANGEROUS = 0x00020000;
        const OFF_LIMITS = 0x00020000;
        const DATA_COMPRESSED = 0x00040000;
        const CANT_WAIT = 0x00080000;
        const ACTI_IGNORE_OBJECT_INTERACTION = 0x00100000;
        const IS_MARKER = 0x00800000;
        const ACTI_OBSTACLE = 0x02000000;
        const REFR_NO_AI_ACQUIRE = 0x02000000;
        const NAV_MESH_GEN_FILTER = 0x04000000;
        const NAV_MESH_GEN_BOUNDING_BOX = 0x08000000;
        const FURN_MUST_EXIT_TO_TALK = 0x10000000;
        const REFR_REFLECTED_BY_AUTO_WATER = 0x10000000;
        const FURN_CHILD_CAN_USE = 0x20000000;
        const REFR_DONT_HAVOK_SETTLE = 0x20000000;
        const NAV_MESH_GEN_GROUND = 0x40000000;
        const REFR_NO_RESPAWN = 0x40000000;
        const REFR_MUTI_BOUND = 0x80000000;

        /// CELL
        const PERSISTENT = 0x400;
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ZString(Vec<u8>);

impl ZString {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn as_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.0).ok()
    }
}

impl Debug for ZString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&String::from_utf8_lossy(&self.0), f)
    }
}

impl Decode for ZString {
    type Error = Error;

    fn decode<B>(mut buf: B) -> Result<Self, Error>
    where
        B: Buf,
    {
        let mut s = Vec::new();
        loop {
            if !buf.has_remaining() {
                return Err(Error::Eof);
            }

            let char = buf.get_u8();
            if char == 0 {
                break;
            }
            s.push(char);
        }

        Ok(Self(s))
    }
}

#[derive(Copy, Clone, Debug)]
struct FieldHeader {
    key: Text<4>,
    size: u32,
}

impl FieldHeader {
    fn read_value<B>(&self, mut buf: B) -> Result<GenericField, Error>
    where
        B: Buf,
    {
        if buf.remaining() < self.size as usize {
            return Err(Error::Eof);
        }

        let value = buf.copy_to_bytes(self.size as usize).to_vec();
        Ok(GenericField {
            key: self.key,
            size: self.size,
            value,
        })
    }
}

impl Decode for FieldHeader {
    type Error = Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let key = Text::<4>::decode(&mut buf)?;
        let size = u16::decode(&mut buf)?;
        Ok(Self {
            key,
            size: size.into(),
        })
    }
}

#[derive(Clone)]
struct GenericField {
    key: Text<4>,
    size: u32,
    value: Vec<u8>,
}

impl GenericField {
    fn to_field<T>(&self) -> Result<Field<T>, T::Error>
    where
        T: Decode,
    {
        let value = T::decode(&mut &self.value[..])?;
        Ok(Field {
            key: self.key,
            size: self.size,
            value,
        })
    }
}

impl Debug for GenericField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(GenericField))
            .field("key", &self.key)
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}

#[derive(Copy, Clone, Debug)]
struct Field<T> {
    key: Text<4>,
    size: u32,
    value: T,
}

impl<T> Decode for Field<T>
where
    T: Decode,
    Error: From<T::Error>,
{
    type Error = Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let key = Text::<4>::decode(&mut buf)?;
        let size = u16::decode(&mut buf)?;
        let value = T::decode(&mut buf)?;
        Ok(Self {
            key,
            size: size.into(),
            value,
        })
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Text<const N: usize>([u8; N]);

impl Text<4> {
    const TES4: Self = Self([b'T', b'E', b'S', b'4']);
    const GRUP: Self = Self([b'G', b'R', b'U', b'P']);

    const DATA: Self = Self([b'D', b'A', b'T', b'A']);
    const EDID: Self = Self([b'E', b'D', b'I', b'D']);
}

impl<const N: usize> Decode for Text<N> {
    type Error = Error;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        <[u8; N]>::decode(buf).map(Self)
    }
}

impl<const N: usize> Debug for Text<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&String::from_utf8_lossy(&self.0), f)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct LString(u32);

impl Decode for LString {
    type Error = Error;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        u32::decode(buf).map(Self)
    }
}

/// A three channel RGB color, padded to four channel width.
///
/// The only difference between `RgbPadded` and [`Rgb`] is the [`Decode`] implementation.
#[derive(Copy, Clone, Debug)]
pub struct RgbPadded<T> {
    /// The red channel.
    pub r: T,
    /// The green channel.
    pub g: T,
    /// The blue channel.
    pub b: T,
}

impl<T> Decode for RgbPadded<T>
where
    T: Decode,
    Error: From<T::Error>,
{
    type Error = Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let r = T::decode(&mut buf)?;
        let g = T::decode(&mut buf)?;
        let b = T::decode(&mut buf)?;
        T::decode(&mut buf)?;

        Ok(Self { r, g, b })
    }
}

impl<T> From<RgbPadded<T>> for Rgb<T> {
    fn from(value: RgbPadded<T>) -> Self {
        Rgb {
            r: value.r,
            g: value.g,
            b: value.b,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct FormId(pub u32);

impl FormId {
    pub const NULL: Self = Self(0);

    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl std::fmt::Debug for FormId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FormId({:X})", self.0)
    }
}

impl Decode for FormId {
    type Error = Error;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        u32::decode(buf).map(Self)
    }
}
/// A three channel RGB color.
#[derive(Copy, Clone, Debug)]
pub struct Rgb<T> {
    /// The red color channel.
    pub r: T,
    /// The green color channel.
    pub g: T,
    /// The blue color channel.
    pub b: T,
}

impl<T> Decode for Rgb<T>
where
    T: Decode,
    Error: From<T::Error>,
{
    type Error = Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let r = T::decode(&mut buf)?;
        let g = T::decode(&mut buf)?;
        let b = T::decode(&mut buf)?;

        Ok(Self { r, g, b })
    }
}

impl Decode for Vec3 {
    type Error = Error;

    fn decode<B>(buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        <[f32; 3]>::decode(buf).map(Self::from_array)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct U16String(pub String);

impl Decode for U16String {
    type Error = Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let len = u16::decode(&mut buf)?;
        let buf = (0..len)
            .map(|_| u8::decode(&mut buf))
            .collect::<Result<_, _>>()?;
        String::from_utf8(buf)
            .map(Self)
            .map_err(Error::InvalidString)
    }
}
