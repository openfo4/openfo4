use openfo4_esp_derive::{BinaryStruct, ConditionFunction};

use crate::fields::{FieldMap, Read};
use crate::{Error, FormId, Text, ZString};

#[derive(Clone, Debug)]
pub struct Conditions {
    pub conditions: Vec<Condition>,
}

impl Read for Conditions {
    type Error = Error;

    fn read(map: &mut FieldMap, _key: Text<4>) -> Result<Self, Self::Error> {
        let mut conditions = Vec::new();

        while let Some(data) = map.consume_next(Text::new(b"CTDA"))? {
            let data = data.value;
            let cis1 = map.consume_next(Text::new(b"CIS1"))?.map(|v| v.value);
            let cis2 = map.consume_next(Text::new(b"CIS2"))?.map(|v| v.value);

            let function = ConditionFunction::parse(data, [cis1, cis2])?;

            let comparator = match data.comparator >> 5 {
                0 => Comparator::Equal,
                1 => Comparator::NotEqual,
                2 => Comparator::GreaterThan,
                3 => Comparator::GreaterThanOrEqual,
                4 => Comparator::LessThan,
                5 => Comparator::LessThanOrEqual,
                val => return Err(Error::BadEnumValue("Comparator", val.into())),
            };

            let combinator = if data.comparator & 1 != 0 {
                Combinator::Or
            } else {
                Combinator::And
            };

            let receiver = if data.comparator & 2 != 0 {
                FunctionReceiver::Target
            } else {
                FunctionReceiver::Subject
            };

            let value = if data.comparator & 4 != 0 {
                ComparisonValue::Global(FormId(data.value))
            } else {
                ComparisonValue::Const(f32::from_bits(data.value))
            };

            conditions.push(Condition {
                object: data.object,
                receiver,
                function,
                comparator,
                value,
                combinator,
            });
        }

        Ok(Self { conditions })
    }

    fn read_next(map: &mut FieldMap, key: Text<4>) -> Result<Self, Self::Error> {
        Self::read(map, key)
    }
}

#[derive(Clone, Debug)]
pub struct Condition {
    pub object: ConditionDataObject,
    /// Defines which entity to run the function on.
    pub receiver: FunctionReceiver,
    pub function: ConditionFunction,
    pub comparator: Comparator,
    pub value: ComparisonValue,
    pub combinator: Combinator,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
struct RawConditionData {
    comparator: u8,
    _pad0: [u8; 3],
    value: u32,
    function: u16,
    _pad1: [u8; 2],
    parameter0: u32,
    parameter1: u32,
    /// The object on which the function should be run on.
    object: ConditionDataObject,
    _unknown: u32,
    parameter2: u32,
}

#[derive(Copy, Clone, Debug, BinaryStruct)]
#[primitive(u32)]
pub enum ConditionDataObject {
    Subject = 0,
    Target = 1,
    Reference = 2,
    CombatTarget = 3,
    LinkedReference = 4,
    QuestAlias = 5,
    PackageData = 6,
    EventData = 7,
    CommandTarget = 8,
    EventCameraRef = 9,
    MyKiller = 10,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Comparator {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

#[derive(Copy, Clone, Debug)]
pub enum ComparisonValue {
    Const(f32),
    Global(FormId),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Combinator {
    And,
    Or,
}

/// Defines which entity to run the [`ConditionFunction`] on.
///
/// The exact meanings of the variants depends on the form where the [`Condition`] is attached.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum FunctionReceiver {
    /// Run the function on the subject entity.
    Subject,
    /// Run the function on the target entity.
    Target,
}

// See https://github.com/TES5Edit/TES5Edit/blob/dev-4.1.6/Core/wbDefinitionsFO4.pas#L274
#[derive(Clone, Debug, ConditionFunction)]
pub enum ConditionFunction {
    #[tag(0)]
    GetWantBlocking,
    #[tag(1)]
    GetDistance { reference: FormId },
    #[tag(5)]
    GetLocked,
    #[tag(6)]
    GetPos { axis: i32 },
    #[tag(8)]
    GetAngle { axis: i32 },
    #[tag(10)]
    GetStartingPos { axis: i32 },
    #[tag(11)]
    GetStartingAngle { axis: i32 },
    #[tag(12)]
    GetSecondsPassed {},
    #[tag(14)]
    GetValue { actor_value: FormId },
    #[tag(18)]
    GetCurrentTime,
    #[tag(24)]
    GetScale,
    #[tag(25)]
    IsMoving,
    #[tag(26)]
    IsTurning,
    #[tag(27)]
    GetLineOfSight { reference: FormId },
    #[tag(32)]
    GetInSameCell { reference: FormId },
    #[tag(35)]
    GetDisabled,
    #[tag(36)]
    MenuMode { menu: i32 },
    #[tag(39)]
    GetDisease,
    #[tag(41)]
    GetClothingValue,
    #[tag(42)]
    SameFaction { actor: FormId },
    #[tag(43)]
    SameRace { actor: FormId },
    #[tag(44)]
    SameSex { actor: FormId },
    #[tag(45)]
    GetDetected { actor: FormId },
    #[tag(46)]
    GetDead,
    #[tag(47)]
    GetItemCount { base_object: FormId },
    #[tag(48)]
    GetGold,
    #[tag(49)]
    GetSleeping,
    #[tag(50)]
    GetTalkedToPc,
    #[tag(56)]
    GetQuestRunning { quest: FormId },
    #[tag(58)]
    GetStage { quest: FormId },
    #[tag(59)]
    GetStageDone { quest: FormId, stage: u32 },
    #[tag(60)]
    GetFactionRankDifference { faction: FormId, actor: FormId },
    #[tag(61)]
    GetAlarmed,
    #[tag(62)]
    IsRaining,
    #[tag(63)]
    GetAttacked,
    #[tag(64)]
    GetIsCreature,
    #[tag(65)]
    GetLockLevel,
    #[tag(66)]
    GetShouldAttack { actor: FormId },
    #[tag(67)]
    GetInCell { cell: FormId },
    #[tag(68)]
    GetIsClass { class: FormId },
    #[tag(69)]
    GetIsRace { race: FormId },
    #[tag(70)]
    GetIsSex { sex: i32 },
    #[tag(71)]
    GetInFaction { faction: FormId },
    #[tag(72)]
    GetIsId { base_object: FormId },
    #[tag(73)]
    GetFactionRank { faction: FormId },
    #[tag(74)]
    GetGlobalValue { global: FormId },
    #[tag(75)]
    IsSnowing,
    #[tag(77)]
    GetRandomPercent,
    #[tag(79)]
    WouldBeStealing { reference: FormId },
    #[tag(80)]
    GetLevel,
    #[tag(81)]
    IsRotating,
    #[tag(84)]
    GetDeadCount { actor_base: FormId },
    #[tag(91)]
    GetIsAlerted,
    #[tag(98)]
    GetPlayerControlsDisabled(i32, i32, i32),
    #[tag(99)]
    GetHeadingAngle { reference: FormId },
    #[tag(101)]
    IsWeaponMagicOut,
    #[tag(102)]
    IsTorchOut,
    #[tag(103)]
    IsShieldOut,
    #[tag(106)]
    IsFacingUp,
    #[tag(107)]
    GetKockedState,
    #[tag(108)]
    GetWeaponAnimType,
    #[tag(109)]
    IsWeaponSkillType { actor_value: FormId },
    #[tag(110)]
    GetCurrentAiPackage,
    #[tag(111)]
    IsWaiting,
    #[tag(112)]
    IsIdlePlaying,
    #[tag(116)]
    IsIntimidatedByPlayer,
    #[tag(117)]
    IsPlayerInRegion { region: FormId },
    #[tag(118)]
    GetActorAggroRadiusViolated,
    #[tag(122)]
    GetCrime { actor: FormId, crime_type: u32 },
    #[tag(123)]
    IsGreetingPlayer,
    #[tag(125)]
    IsGuard,
    #[tag(127)]
    HasBeenEaten,
    #[tag(128)]
    GetStaminaPercentage,
    #[tag(129)]
    HasBeenRead,
    #[tag(130)]
    GetDying,
    #[tag(131)]
    GetSceneActionPercent { scene: FormId, _unknown: i32 },
    #[tag(132)]
    WouldRefuseCommand { reference: FormId },
    #[tag(133)]
    SameFactionAsPc,
    #[tag(134)]
    SameRaceAsPc,
    #[tag(135)]
    SameSexAsPc,
    #[tag(136)]
    GetIsReference { reference: FormId },
    #[tag(141)]
    IsTalking,
    #[tag(142)]
    GetComponentCount { base_object: FormId },
    #[tag(143)]
    GetCurrentAiProdecure,
    #[tag(144)]
    GetTrespassWarningLevel,
    #[tag(145)]
    IsTrespassing,
    #[tag(146)]
    IsInMyOwnedCell,
    #[tag(147)]
    GetWindSpeed,
    #[tag(148)]
    GetCurrentWeatherPercent,
    #[tag(149)]
    GetIsCurrentWeather { weather: FormId },
    #[tag(150)]
    IsContinuingPackagePcNear,
    #[tag(152)]
    GetIsCrimeFaction { faction: FormId },
    #[tag(153)]
    CanHaveFlames,
    #[tag(154)]
    HasFlames,
    #[tag(157)]
    GetOpenState,
    #[tag(159)]
    GetSitting,
    #[tag(161)]
    GetIsCurrentPackage { package: FormId },
    #[tag(162)]
    IsCurrentFurnitureRef { reference: FormId },
    #[tag(163)]
    IsCurrentFurnitureObj { reference: FormId },
    #[tag(170)]
    GetDayOfWeek,
    #[tag(172)]
    GetTalkedToPcParam { actor: FormId },
    #[tag(175)]
    IsPcSleeping,
    #[tag(176)]
    IsPcAMurderer,
    #[tag(180)]
    HasSameEditorLocationAsRef { reference: FormId, keyword: FormId },
    #[tag(181)]
    HasSameEditorLocationAsRefAlias { alias: FormId, keyword: FormId },
    #[tag(182)]
    GetEquipped,
    #[tag(185)]
    IsSwimming,
    #[tag(190)]
    GetAmountSoldStolen,
    #[tag(192)]
    GetIgnoreCrime,
    #[tag(193)]
    GetPcExpelled { faction: FormId },
    #[tag(195)]
    GetPcFactionMurder { faction: FormId },
    #[tag(197)]
    GetPcEnemyOfFaction { faction: FormId },
    #[tag(199)]
    GetPcFactionAttack { faction: FormId },
    #[tag(203)]
    GetDestroyed,
    #[tag(214)]
    HasMagicEffect { base_effect: FormId },
    #[tag(215)]
    GetDefaultOpen,
    #[tag(223)]
    IsSpellTarget { effect_item: FormId },
    #[tag(224)]
    GetVatsMode,
    #[tag(225)]
    GetPersuasionNumber,
    #[tag(226)]
    GetVampireFeed,
    #[tag(227)]
    GetCannibal,
    #[tag(228)]
    GetIsClassDefault { class: FormId },
    #[tag(229)]
    GetClassDefaultMatch,
    #[tag(230)]
    GetInCellParam { cell: FormId, reference: FormId },
    #[tag(231)]
    GetPlayerDialogueInput,
    #[tag(235)]
    GetVatsTargetHeight,
    #[tag(237)]
    GetIsGhost,
    #[tag(242)]
    GetUnconscious,
    #[tag(244)]
    GetRestrained,
    #[tag(246)]
    GetIsUsedItem { base_object: FormId },
    #[tag(247)]
    GetIsUsedItemType { form_type: FormId },
    #[tag(248)]
    IsScenePlaying { scene: FormId },
    #[tag(249)]
    IsInDialogueWithPlayer,
    #[tag(250)]
    GetLocationCleared { location: FormId },
    #[tag(254)]
    GetIsPlayableRace,
    #[tag(255)]
    GetOffersServiceNow,
    #[tag(258)]
    HasAssociationType {
        actor: FormId,
        association_type: FormId,
    },
    #[tag(259)]
    HasFamilyRelationship { actor: FormId },
    #[tag(261)]
    HasParentRelationship { actor: FormId },
    #[tag(262)]
    IsWarningAbout { form_list: FormId },
    #[tag(263)]
    IsWeaponOut,
    #[tag(264)]
    HasSpell { effect_item: FormId },
    #[tag(265)]
    IsTimePassing,
    #[tag(266)]
    IsPleasant,
    #[tag(267)]
    IsCloudy,
    #[tag(274)]
    IsSmallBump,
    #[tag(277)]
    GetBaseValue { actor_value: FormId },
    #[tag(278)]
    IsOwner { owner: FormId },
    #[tag(280)]
    IsCellOwner { cell: FormId, owner: FormId },
    #[tag(282)]
    IsHorseStolen,
    #[tag(285)]
    IsLeftUp,
    #[tag(286)]
    IsSneaking,
    #[tag(287)]
    IsRunning,
    #[tag(288)]
    GetFriendHit,
    #[tag(289)]
    IsInCombat(i32),
    #[tag(300)]
    IsInInterior,
    #[tag(304)]
    IsWaterObject,
    #[tag(305)]
    GetPlayerAction,
    #[tag(306)]
    IsActorUsingTorch,
    #[tag(309)]
    IsXBox,
    #[tag(310)]
    GetInWorldspace { worldspace: FormId },
    #[tag(312)]
    GetPcMiscStat { misc_stat: FormId },
    #[tag(313)]
    GetPairedAnimation,
    #[tag(314)]
    IsActorVictim,
    #[tag(315)]
    GetTotalPersuasionNumber,
    #[tag(318)]
    GetIdleDoneOnce,
    #[tag(320)]
    GetNoRumors,
    #[tag(323)]
    GetCombatState,
    #[tag(325)]
    GetWithinPackageLocation { package_data: FormId },
    #[tag(327)]
    IsRidingMount,
    #[tag(329)]
    IsFleeing,
    #[tag(332)]
    IsInDangerousWater,
    #[tag(338)]
    GetIgnoreFriendlyHits,
    #[tag(339)]
    IsPlayersLastRiddenMount,
    #[tag(353)]
    IsActor,
    #[tag(354)]
    IsEssential,
    #[tag(358)]
    IsPlayerMovingIntoNewSpace,
    #[tag(359)]
    GetInCurrentLocation { location: FormId },
    #[tag(360)]
    GetInCurrentLocationAlias { alias: FormId },
    #[tag(361)]
    GetTimeDead,
    #[tag(362)]
    HasLinkedRef { keyword: FormId },
    #[tag(365)]
    IsChild,
    #[tag(366)]
    GetStolenItemValueNoCrime { faction: FormId },
    #[tag(367)]
    GetLastPlayerAction,
    #[tag(368)]
    IsPlayerActionActive(i32),
    #[tag(370)]
    IsTalkingActivatorActor { actor: FormId },
    #[tag(372)]
    IsInList { list: FormId },
    #[tag(373)]
    GetStolenItemValue { faction: FormId },
    #[tag(375)]
    GetCrimeGoldViolent { faction: FormId },
    #[tag(376)]
    GetCrimeGoldNonViolent { form: FormId },
    #[tag(378)]
    IsOwnedBy { actor: FormId },
    #[tag(380)]
    GetCommandDistance,
    #[tag(381)]
    GetCommandLocationDistance,
    #[tag(390)]
    GetHitLocation,
    #[tag(391)]
    IsPc1stPerson,
    #[tag(396)]
    GetCauseOfDeath,
    #[tag(397)]
    IsLimbGone { limb: i32 },
    #[tag(398)]
    IsWeaponInList { list: FormId },
    #[tag(402)]
    IsBribedByPlayer,
    #[tag(403)]
    GetRelationshipRank { actor: FormId },
    #[tag(407)]
    GetVatsValue(i32, i32),
    #[tag(408)]
    IsKiller { actor: FormId },
    #[tag(409)]
    IsKillerObject { list: FormId },
    #[tag(410)]
    GetFactionCombatReaction {
        faction: FormId,
        other_faction: FormId,
    },
    #[tag(414)]
    Exists { reference: FormId },
    #[tag(415)]
    GetGroupMemberCount,
    #[tag(416)]
    GetGroupTargetCount,
    #[tag(426)]
    GetIsVoiceType { voice_type: FormId },
    #[tag(427)]
    GetPlantedExplosive,
    #[tag(429)]
    IsScenePackageRunning,
    #[tag(430)]
    GetHealthPercentage,
    #[tag(432)]
    GetIsObjectType { object_type: FormId },
    #[tag(434)]
    PlayerVisualDetection,
    #[tag(435)]
    PlayerAudioDetection,
    #[tag(437)]
    GetIsCreatureType(i32),
    #[tag(438)]
    HasKey { key: FormId },
    #[tag(439)]
    IsFurnitureEntryType { base_object: FormId },
    #[tag(444)]
    GetInCurrentLocationFormList { list: FormId },
    #[tag(445)]
    GetInZone { encounter_zone: FormId },
    #[tag(446)]
    GetVelocity { axis: i32 },
    #[tag(447)]
    GetGraphVariableFloat { name: ZString },
    #[tag(448)]
    HasPerk { perk: FormId },
    #[tag(449)]
    GetFactionRelation { actor: FormId },
    #[tag(450)]
    IsLastIdlePlayed { idle_animation: FormId },
    #[tag(453)]
    GetPlayerTeammate,
    #[tag(454)]
    GetPlayerTemmateCount,
    #[tag(458)]
    GetActorCrimePlayerEnemey,
    #[tag(459)]
    GetCrimeGold { faction: FormId },
    #[tag(463)]
    IsPlayerGrabbedRef { reference: FormId },
    #[tag(465)]
    GetkeywordItemCount { keyword: FormId },
    #[tag(470)]
    GetDestructionStage,
    #[tag(473)]
    GetIsAlignment { alignment: FormId },
    #[tag(476)]
    IsProtected,
    #[tag(477)]
    GetThreatRatio { actor: FormId },
    #[tag(479)]
    GetIsUsedItemEquipType { equip_type: FormId },
    #[tag(483)]
    GetPlayerActivated,
    #[tag(485)]
    GetFullyEnabledActorsInHigh,
    #[tag(487)]
    IsCarryable,
    #[tag(488)]
    GetConcuseed,
    #[tag(491)]
    GetMapMarkerVisible,
    #[tag(493)]
    PlayerKnows { base_object: FormId },
    #[tag(494)]
    GetPermanentValue { actor_value: FormId },
    #[tag(495)]
    GetKillingBlowLimb,
    #[tag(497)]
    CanPayCrimeGold { faction: FormId },
    #[tag(499)]
    GetDaysInJail,
    #[tag(500)]
    EpAlchemyGetMakingPoison,
    #[tag(501)]
    EpAlchemyEffectHasKeyword { keyword: FormId },
    #[tag(503)]
    GetAllowedWorldInteractions,
    #[tag(506)]
    DialogueGetActorValue { actor_value: FormId },
    #[tag(507)]
    DialogueHasPerk { perk: FormId },
    #[tag(508)]
    GetLastHitCritical,
    #[tag(510)]
    DialogueGetItemCount { base_object: FormId },
    #[tag(511)]
    LastCrippledCondition { actor_value: FormId },
    #[tag(512)]
    HasSharedPowerGrid { reference: FormId },
    #[tag(513)]
    IsCombatTarget { actor: FormId },
    #[tag(515)]
    GetVatsRightAreaFree { reference: FormId },
    #[tag(516)]
    GetVatsLeftAreaFree { reference: FormId },
    #[tag(517)]
    GetVatsBackAreaFree { reference: FormId },
    #[tag(518)]
    GetVatsFrontAreaFree { reference: FormId },
    #[tag(519)]
    GetIsLockBroken,
    #[tag(520)]
    IsPS3,
    #[tag(521)]
    IsWindowsPc,
    #[tag(522)]
    GetVatsRightTargetVisible { reference: FormId },
    #[tag(523)]
    GetVatsLefttargetVisible { reference: FormId },
    #[tag(524)]
    GetVatsBackTargetVisible { reference: FormId },
    #[tag(525)]
    GetVatsFrontTargetVisible { reference: FormId },
    #[tag(528)]
    IsInCriticalStage { stage: i32 },
    #[tag(530)]
    GetXpForNextLevel,
    #[tag(533)]
    GetInfamy { faction: FormId },
    #[tag(534)]
    GetInfamyViolent { faction: FormId },
    #[tag(535)]
    GetInfamyNonViolent { faction: FormId },
    #[tag(536)]
    GetTypeCommandPerforming,
    #[tag(543)]
    GetQuestCompleted { quest: FormId },
    #[tag(544)]
    GetSpeechChallengeSuccessLevel,
    #[tag(547)]
    IsGoreDisabled,
    #[tag(550)]
    IsSceneActionComplete { scene: FormId, action: i32 },
    #[tag(552)]
    GetSpellUsageNum { effect_item: FormId },
    #[tag(554)]
    GetActorsInHigh,
    #[tag(555)]
    HasLoaded3D,
    #[tag(560)]
    HasKeyword { keyword: FormId },
    #[tag(561)]
    HasRefType { location_ref_type: FormId },
    #[tag(562)]
    LocationHasKeyword { keyword: FormId },
    #[tag(563)]
    LocationHasRefType { location_ref_type: FormId },
    #[tag(565)]
    GetIsEditorLocation { location: FormId },
    #[tag(566)]
    GetIsAliasRef { alias: u32 },
    #[tag(567)]
    GetIsEditorLocationAlias { alias: u32 },
    #[tag(568)]
    IsSprinting,
    #[tag(569)]
    IsBlocking,
    #[tag(570)]
    HasEquippedSpell(i32),
    #[tag(571)]
    GetCurrentCastingType(i32),
    #[tag(572)]
    GetCurrentDeliveryType(i32),
    #[tag(574)]
    GetAttackState,
    #[tag(576)]
    GetEventData(i32, i32),
    #[tag(577)]
    IsCloserToAThanB {
        reference_a: FormId,
        reference_b: FormId,
    },
    #[tag(578)]
    LevelMinusPcLevel,
    #[tag(580)]
    IsBleedingOut,
    #[tag(584)]
    GetRelativeAngle { reference: FormId, axis: i32 },
    #[tag(589)]
    GetMovementDirection,
    #[tag(590)]
    IsInScene,
    #[tag(591)]
    GetRefTypeDeadCount {
        location: FormId,
        location_ref_type: FormId,
    },
    #[tag(592)]
    GetRefTypeAliveCount {
        location: FormId,
        location_ref_type: FormId,
    },
    #[tag(594)]
    GetIsFlying,
    #[tag(595)]
    IsCurrentSpell(FormId, i32),
    #[tag(596)]
    SpellHasKeyword(FormId, FormId),
    #[tag(597)]
    GetEquippedItemType(i32),
    #[tag(598)]
    GetLocationAliasCleared(FormId),
    #[tag(600)]
    GetLocationAliasRefTypeDeadCount(FormId, i32),
    #[tag(601)]
    GetLocationAliasRefTypeAliveCount(FormId, i32),
    #[tag(602)]
    IsWardState(i32),
    #[tag(603)]
    IsInSameCurrentLocationAsRef(FormId, FormId),
    #[tag(604)]
    IsInSameCurrentLocationAsRefAlias(FormId, FormId),
    #[tag(605)]
    LocationAliasIsLocation(FormId, FormId),
    #[tag(606)]
    GetKeywordDataForLocation(FormId, FormId),
    #[tag(608)]
    GetKeywordDataForAlias(FormId, FormId),
    #[tag(610)]
    LocationAliasHasKeyword(FormId, FormId),
    #[tag(611)]
    IsNullPackageData(FormId),
    #[tag(612)]
    GetNumericPackageData { package_data: u32 },
    #[tag(613)]
    IsPlayerRadioOn,
    #[tag(614)]
    GetPlayerRadioFrequency,
    #[tag(615)]
    GetHighestRelationshipRank,
    #[tag(616)]
    GetLowestRelationshipRank,
    #[tag(617)]
    HasAssociationTypeAny(FormId),
    #[tag(618)]
    HasFamilyRelationshipAny,
    #[tag(619)]
    GetPathingTargetOffset(i32),
    #[tag(620)]
    GetPathingTargetAngleOffset(i32),
    #[tag(621)]
    GetPathingTargetSpeed,
    #[tag(622)]
    GetPathingTargetSpeedAngle(i32),
    #[tag(623)]
    GetMovementSpeed,
    #[tag(624)]
    GetInContainer(FormId),
    #[tag(625)]
    IsLocationLoaded(FormId),
    #[tag(626)]
    IsLocationAliasLoaded(FormId),
    #[tag(627)]
    IsDualCasting,
    #[tag(629)]
    GetVmQuestVariable(FormId, ZString),
    #[tag(630)]
    GetCombatAudioDetection,
    #[tag(631)]
    GetCombatVisualDetection,
    #[tag(632)]
    IsCasting,
    #[tag(633)]
    GetFlyingState,
    #[tag(635)]
    IsInFavorState,
    #[tag(636)]
    HasTwoHandedWeaponEquipped,
    #[tag(637)]
    IsFurnitureExittype(FormId),
    #[tag(638)]
    IsInFrientStateWithPlayer,
    #[tag(639)]
    GetWithinDistance(FormId, f32),
    #[tag(640)]
    GetValuePercent(FormId),
    #[tag(641)]
    IsUnique,
    #[tag(642)]
    GetLastBumpDirection,
    #[tag(644)]
    GetInfoChallengeSuccess,
    #[tag(645)]
    GetIsInjured,
    #[tag(646)]
    GetIsCrashLandRequest,
    #[tag(647)]
    GetIsHastyLandRequest,
    #[tag(650)]
    IsLinkedTo(FormId, FormId),
    #[tag(651)]
    GetKeywordDataForCurrentLocation(FormId),
    #[tag(652)]
    GetInSharedCrimeFaction(FormId),
    #[tag(654)]
    GetBribeSuccess,
    #[tag(655)]
    GetIntimidateSuccess,
    #[tag(656)]
    GetArrestedState,
    #[tag(657)]
    GetArrestingActor,
    #[tag(659)]
    HasVmScript(ZString),
    #[tag(660)]
    GetVmScriptVariable(ZString, ZString),
    #[tag(661)]
    GetWorkshopResourceDamage(FormId),
    #[tag(664)]
    HasValidRumorTopic { quest: FormId },
    #[tag(672)]
    IsAttacking,
    #[tag(673)]
    IsPowerAttacking,
    #[tag(674)]
    IsLastHostileActor,
    #[tag(675)]
    GetGraphVariableInt(ZString),
    #[tag(678)]
    ShouldAttackKill(FormId),
    #[tag(680)]
    GetActivationHeight,
    #[tag(682)]
    WornHasKeyword(FormId),
    #[tag(683)]
    GetPathingCurrentSpeed,
    #[tag(684)]
    GetPathingCurrentSpeedAngle(i32),
    #[tag(691)]
    GetWorkshopObjectCount(FormId),
    #[tag(693)]
    EpMagicSpellHashKeyword(FormId),
    #[tag(694)]
    GetNoBleedoutRecovery,
    #[tag(696)]
    EpMagicSpellHasSkill(FormId),
    #[tag(697)]
    IsAttackType(FormId),
    #[tag(698)]
    IsAllowedToFly,
    #[tag(699)]
    HasMagicEffectKeyword(FormId),
    #[tag(700)]
    IsCommandedActor,
    #[tag(701)]
    IsStaggered,
    #[tag(702)]
    IsRecoiling,
    #[tag(703)]
    HasScopeWeaponEquipped,
    #[tag(704)]
    IsPathing,
    #[tag(705)]
    GetShouldHelp(FormId),
    #[tag(706)]
    HasBoundWeaponEquipped(i32),
    #[tag(707)]
    GetCombatTargetHasKeyword(FormId),
    #[tag(709)]
    GetCombatGroupMemberCount,
    #[tag(710)]
    IsIgnoringCombat,
    #[tag(711)]
    GetLightLevel,
    #[tag(713)]
    SpellHasCastingPerk(FormId),
    #[tag(714)]
    IsBeingRidden,
    #[tag(715)]
    IsUndead,
    #[tag(716)]
    GetRealHoursPassed,
    #[tag(718)]
    IsUnlockedDoor,
    #[tag(719)]
    IsHostileToActor(FormId),
    #[tag(720)]
    GetTargetHeight(FormId),
    #[tag(7212)]
    IsPoison,
    #[tag(722)]
    WornApparelHasKeywordCount(FormId),
    #[tag(723)]
    GetItemHealthPercent,
    #[tag(724)]
    EffectWasDualCast,
    #[tag(725)]
    GetKnockStateEnum,
    #[tag(726)]
    DoesNotExist,
    #[tag(728)]
    GetPlayerWalkAwayFromDialogueScene,
    #[tag(729)]
    GetActorStance,
    #[tag(734)]
    CanProduceForWorkshop,
    #[tag(735)]
    CanFlyHere,
    #[tag(736)]
    EpIsDamage(FormId),
    #[tag(738)]
    GetActorGunState,
    #[tag(739)]
    GetVoiceLineLength,
    #[tag(741)]
    ObjectTemplateItemHasKeyword(FormId),
    #[tag(742)]
    ObjectTemplateItemhasUniqueKeyword(FormId),
    #[tag(743)]
    ObjectTemplateItemGetLevel,
    #[tag(744)]
    MovementIdleMatches(i32, i32),
    #[tag(745)]
    GetActionData,
    #[tag(746)]
    GetActionDataShort(i32),
    #[tag(747)]
    GetActionDataByte(i32),
    #[tag(748)]
    GetActionDataFlag(i32),
    #[tag(749)]
    ModdedItemHasKeyword(FormId),
    #[tag(750)]
    GetAngryWithPlayer,
    #[tag(751)]
    IsCameraUnderWater,
    #[tag(753)]
    IsActorRefOwner(FormId),
    #[tag(754)]
    HasActorRefOwner(FormId),
    #[tag(756)]
    GetLoadedAmmoCount,
    #[tag(757)]
    IsTimespanSunrise,
    #[tag(758)]
    IsTimespanMorning,
    #[tag(759)]
    IsTimespanAfternoon,
    #[tag(760)]
    IsTimespanEvening,
    #[tag(761)]
    IsTimespanSunset,
    #[tag(762)]
    IsTimespanNight,
    #[tag(763)]
    IsTimespanMidnight,
    #[tag(764)]
    IsTimespanAnyDay,
    #[tag(765)]
    IsTimespanAnyNight,
    #[tag(766)]
    CurrentFurnitureHasKeyword(FormId),
    #[tag(767)]
    GetWeaponEquipIndex,
    #[tag(769)]
    IsOverEncumbered,
    #[tag(770)]
    IsPackageRequestingBlockedIdles,
    #[tag(771)]
    GetActionDataInt,
    #[tag(772)]
    GetVatsRightMinusLeftAreaFree(FormId),
    #[tag(773)]
    GetInIronSights(FormId),
    #[tag(774)]
    GetActorStaggerDirection,
    #[tag(775)]
    GetActorStaggerMagnitude,
    #[tag(776)]
    WornCoversBipedSlot(i32),
    #[tag(777)]
    GetInventoryValue,
    #[tag(778)]
    IsPlayerInConversation,
    #[tag(779)]
    IsInDialogueCamera,
    #[tag(780)]
    IsMyDialogueTargetPlayer,
    #[tag(781)]
    IsMyDialogueTargetActor,
    #[tag(782)]
    GetMyDialogueTargetDistance,
    #[tag(783)]
    IsSeatOccupied(FormId),
    #[tag(784)]
    IsPlayerRiding,
    #[tag(785)]
    IsTryingEventCamera,
    #[tag(786)]
    UseLeftSideCamera,
    #[tag(787)]
    GetNoteType,
    #[tag(788)]
    LocationHasPlayerOwnedWorkshop,
    #[tag(789)]
    IsStartinAction,
    #[tag(790)]
    IsMidAction,
    #[tag(791)]
    IsWeaponChargeAttack,
    #[tag(792)]
    IsInWorkshopMode,
    #[tag(793)]
    IsWeaponCharginhgoldAttack,
    #[tag(794)]
    IsEncounterAbovePlayerLevel,
    #[tag(795)]
    IsMeleeAttacking,
    #[tag(796)]
    GetVatsQueuedTargetsUnique,
    #[tag(797)]
    GetCurrentLocationCleared,
    #[tag(798)]
    IsPowered,
    #[tag(799)]
    GetTransmitterDistance,
    #[tag(800)]
    GetCameraPlaybackTime,
    #[tag(801)]
    IsInWater,
    #[tag(802)]
    GetWithinActivateDistance(FormId),
    #[tag(803)]
    IsUnderWater,
    #[tag(804)]
    IsInSameSpace(FormId),
    #[tag(805)]
    LocationAllowsReset,
    #[tag(806)]
    GetVatsBackRightAreaFree(FormId),
    #[tag(807)]
    GetVatsBackLeftAreaFree(FormId),
    #[tag(808)]
    GetVatsBackRightTargetVisible(FormId),
    #[tag(809)]
    GetVatsBackLeftTargetVisible(FormId),
    #[tag(810)]
    GetVatsTargetLimbVisible(FormId),
    #[tag(811)]
    IsPlayerListening(f32),
    #[tag(812)]
    GetPathingRequestedQuickTurn,
    #[tag(813)]
    EpIsCalculatingBaseDamage,
    #[tag(814)]
    GetReanimating,
    #[tag(817)]
    IsInRobotWorkbench,
}

trait ConditionParameter: Sized {
    fn parse(raw: u32, cis: Option<ZString>) -> Result<Self, Error>;
}

impl ConditionParameter for u32 {
    fn parse(raw: u32, _cis: Option<ZString>) -> Result<Self, Error> {
        Ok(raw)
    }
}

impl ConditionParameter for i32 {
    fn parse(raw: u32, _cis: Option<ZString>) -> Result<Self, Error> {
        Ok(raw.cast_signed())
    }
}

impl ConditionParameter for f32 {
    fn parse(raw: u32, _cis: Option<ZString>) -> Result<Self, Error> {
        Ok(f32::from_bits(raw))
    }
}

impl ConditionParameter for FormId {
    fn parse(raw: u32, _cis: Option<ZString>) -> Result<Self, Error> {
        Ok(FormId(raw))
    }
}

impl ConditionParameter for ZString {
    fn parse(_raw: u32, cis: Option<ZString>) -> Result<Self, Error> {
        match cis {
            Some(s) => Ok(s),
            None => Err(Error::Other(
                "CIS missing to decode ZString for ConditionData".to_owned(),
            )),
        }
    }
}
