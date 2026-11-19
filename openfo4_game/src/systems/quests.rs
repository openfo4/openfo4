pub mod dialogue;
mod loader;
mod scene;

use std::collections::VecDeque;
use std::sync::Arc;

use crossbeam::queue::SegQueue;
use event_listener::IntoNotification;
use hashbrown::{HashMap, HashSet};
use indexmap::IndexMap;
use openfo4_asset::AssetLoader;
use openfo4_audio::AudioManager;
use openfo4_core::ecs::{Entity, World};
use openfo4_core::events::EventBus;
use openfo4_core::record::RecordId;
use openfo4_esp::records::{QuestAliasFlags, QuestReferenceAlias};
use openfo4_esp::{RecordData, RecordKind};
use openfo4_tasks::{Task, TaskPool};
use openfo4_tracing::trace_span;

use crate::events::quest::{StartQuestScene, StopQuestScene};
use crate::state::esp::EspLoader;
use crate::state::level::loader::{EntityLoader, EntityRef};
use crate::state::level::partial::PartialEntityFlags;
use crate::systems::conditions::ConditionCompiler;
use crate::systems::global::{self, Alias, AliasData, AliasKey, GlobalProperties, QuestEvent};
use crate::systems::quests::loader::QuestLoader;
use crate::systems::quests::scene::{DialogueTopicAction, SceneState};
use crate::systems::scripting::invocations::{
    AliasInit, AliasShutdown, Init, QuestFragmentSetStage, QuestInit, QuestSetStage, QuestShutdown,
};
use crate::systems::scripting::loader::ScriptObject;
use crate::systems::scripting::{Executor, Invocation};

#[derive(Debug)]
pub struct QuestManager {
    pool: Arc<TaskPool>,
    esp_loader: Arc<EspLoader>,
    entity_loader: Arc<EntityLoader>,
    events: Arc<SegQueue<Event>>,
    quests: HashMap<RecordId, Quest>,
    scenes_to_quest: HashMap<RecordId, RecordId>,
    loader: AssetLoader,
    topics: DialogueTopics,
    quest_loader: Arc<QuestLoader>,
    wakers: Wakers,
    quests_loading: HashMap<RecordId, Task<()>>,
    deferred_actions: HashMap<RecordId, Vec<QuestAction>>,
}

impl QuestManager {
    pub fn new(
        pool: Arc<TaskPool>,
        esp_loader: &Arc<EspLoader>,
        executor: &mut Executor,
        loader: &Arc<EntityLoader>,
        condition_compiler: Arc<ConditionCompiler>,
        asset_loader: AssetLoader,
    ) -> Self {
        Self {
            pool,
            quests: HashMap::new(),
            esp_loader: esp_loader.clone(),
            entity_loader: loader.clone(),
            events: Arc::default(),
            scenes_to_quest: HashMap::new(),
            quest_loader: Arc::new(QuestLoader::new(
                esp_loader.clone(),
                condition_compiler,
                executor.loader().clone(),
            )),
            loader: asset_loader,
            topics: DialogueTopics::default(),
            wakers: Wakers::default(),
            quests_loading: HashMap::new(),
            deferred_actions: HashMap::new(),
        }
    }

    pub fn update(
        &mut self,
        world: &mut World,
        events: &mut EventBus,
        executor: &mut Executor,
        audio: &mut AudioManager,
        globals: &GlobalProperties,
    ) {
        let _span = trace_span!("QuestManager::update").entered();

        while let Some(event) = globals.quests.events.pop() {
            match event {
                QuestEvent::Start { quest, waker } => {
                    let quest_id = quest;

                    if let Some(quest) = self.quests.get_mut(&quest) {
                        if matches!(quest.state, QuestState::Running) {
                            waker.notify(waker.total_listeners().tag(true));
                            continue;
                        }

                        self.wakers
                            .on_quest_start
                            .entry(quest_id)
                            .or_default()
                            .push(waker);

                        // Start the quest if it is not already starting.
                        if !matches!(quest.state, QuestState::Starting(_)) {
                            quest.actions.push_back(QuestAction::Start);
                        }

                        continue;
                    }

                    self.wakers
                        .on_quest_start
                        .entry(quest_id)
                        .or_default()
                        .push(waker);

                    if self.quests_loading.contains_key(&quest_id) {
                        continue;
                    }

                    let loader = self.quest_loader.clone();
                    let events = self.events.clone();
                    let task = self.pool.spawn(async move {
                        let quest = match loader.load(quest).await {
                            Ok(mut quest) => {
                                quest.actions.push_back(QuestAction::Start);
                                Some(quest)
                            }
                            Err(err) => {
                                tracing::error!("failed to load quest {quest}: {err}");
                                None
                            }
                        };

                        events.push(Event::Discover(quest_id, quest));
                    });

                    debug_assert!(!self.quests_loading.contains_key(&quest_id));
                    self.quests_loading.insert(quest_id, task);
                }
                QuestEvent::Stop { quest, waker } => {
                    let quest_id = quest;

                    // A quest that is currently loading is queued to be started.
                    if self.quests_loading.contains_key(&quest) {
                        self.wakers
                            .on_quest_stop
                            .entry(quest_id)
                            .or_default()
                            .push(waker);

                        self.deferred_actions
                            .entry(quest)
                            .or_default()
                            .push(QuestAction::Stop);

                        continue;
                    }

                    // A quest that was not loaded yet is stopped.
                    let Some(quest) = self.quests.get_mut(&quest) else {
                        waker.notify(waker.total_listeners());
                        continue;
                    };

                    if matches!(quest.state, QuestState::Stopped) {
                        waker.notify(waker.total_listeners());
                        continue;
                    }

                    self.wakers
                        .on_quest_stop
                        .entry(quest_id)
                        .or_default()
                        .push(waker);

                    if !matches!(quest.state, QuestState::Stopping(_)) {
                        quest.actions.push_back(QuestAction::Stop);
                    }
                }
                QuestEvent::SetStage {
                    quest: quest_id,
                    stage,
                    waker,
                } => {
                    self.wakers
                        .on_quest_set_stage
                        .entry((quest_id, stage))
                        .or_default()
                        .push(waker);

                    if let Some(quest) = self.quests.get_mut(&quest_id) {
                        // We always need to ensure the quest is running before
                        // setting the stage.
                        quest.actions.push_back(QuestAction::Start);
                        quest.actions.push_back(QuestAction::SetStage(stage));
                        continue;
                    }

                    if self.quests_loading.contains_key(&quest_id) {
                        self.deferred_actions
                            .entry(quest_id)
                            .or_default()
                            .extend([QuestAction::Start, QuestAction::SetStage(stage)]);
                        continue;
                    }

                    let loader = self.quest_loader.clone();
                    let events = self.events.clone();
                    let task = self.pool.spawn(async move {
                        let quest = match loader.load(quest_id).await {
                            Ok(mut quest) => {
                                quest.actions.push_back(QuestAction::Start);
                                quest.actions.push_back(QuestAction::SetStage(stage));
                                Some(quest)
                            }
                            Err(err) => {
                                tracing::error!("failed to load quest {quest_id}: {err}");
                                None
                            }
                        };

                        events.push(Event::Discover(quest_id, quest));
                    });

                    debug_assert!(!self.quests_loading.contains_key(&quest_id));
                    self.quests_loading.insert(quest_id, task);
                }
            }
        }

        while let Some(event) = self.events.pop() {
            match event {
                Event::Discover(id, quest) => {
                    self.quests_loading.remove(&id);
                    let actions = self.deferred_actions.remove(&id).unwrap_or_default();

                    if let Some(mut quest) = quest {
                        for scene in quest.scenes.keys() {
                            self.scenes_to_quest.insert(*scene, id);
                        }

                        quest.actions.extend(actions);
                        self.quests.insert(id, quest);
                        continue;
                    }

                    // Quest failed to load.
                    for waker in self.wakers.on_quest_start.remove(&id).unwrap_or_default() {
                        waker.notify(waker.total_listeners().tag(false));
                    }

                    for waker in self.wakers.on_quest_stop.remove(&id).unwrap_or_default() {
                        waker.notify(waker.total_listeners());
                    }

                    for action in actions {
                        match action {
                            QuestAction::SetStage(stage) => {
                                for waker in self
                                    .wakers
                                    .on_quest_set_stage
                                    .remove(&(id, stage))
                                    .unwrap_or_default()
                                {
                                    waker.notify(waker.total_listeners().tag(false));
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        for event in events.read::<StartQuestScene>() {
            let Some(quest) = self.scenes_to_quest.get(&event.scene) else {
                continue;
            };

            let quest = self.quests.get_mut(quest).unwrap();
            quest
                .actions
                .push_back(QuestAction::StartScene(event.scene));
        }

        for event in events.read::<StopQuestScene>() {
            let Some(quest) = self.scenes_to_quest.get(&event.scene) else {
                continue;
            };

            let quest = self.quests.get_mut(quest).unwrap();
            quest.actions.push_back(QuestAction::StopScene(event.scene));
        }

        self.advance_quests(executor, globals);
        self.advance_scenes(world, events, executor, globals);
        dialogue::play_actor_dialogue(world, audio, globals, self);
        dialogue::initiate_player_npc_interactions(world, events, self, executor);
    }

    fn advance_quests(&mut self, executor: &mut Executor, globals: &GlobalProperties) {
        for (quest_id, quest) in &mut self.quests {
            let mut quests = globals.quests.quests.lock();
            let global = quests.entry(*quest_id).or_insert_with(|| global::Quest {
                cur_stage: 0,
                state: global::QuestState::Stopped,
                is_active: false,
                stages_done: HashSet::new(),
            });

            loop {
                match &mut quest.state {
                    QuestState::PreInit => {
                        if let Some(script) = &quest.script {
                            let id = executor.invoke(Init {
                                object: script.clone(),
                            });

                            quest.state = QuestState::Init(id);
                        } else {
                            quest.state = QuestState::Stopped;
                        }
                    }
                    QuestState::Init(invocation) => {
                        if !invocation.is_done() {
                            break;
                        }

                        quest.state = QuestState::Stopped;
                    }
                    QuestState::Stopped => {
                        for waker in self
                            .wakers
                            .on_quest_stop
                            .remove(quest_id)
                            .unwrap_or_default()
                        {
                            waker.notify(waker.total_listeners());
                        }

                        if quest
                            .actions
                            .pop_front_if(|action| *action == QuestAction::Start)
                            .is_none()
                        {
                            break;
                        }

                        tracing::debug!("starting quest {:?}", quest_id);

                        let id = *quest_id;
                        let esp_loader = self.esp_loader.clone();
                        let entity_loader = self.entity_loader.clone();

                        // Let's load some aliases!
                        let task = self.pool.spawn(async move {
                            let quest = esp_loader
                                .lookup(Some(RecordKind::Quest), id)
                                .await
                                .unwrap();
                            let RecordData::Quest(data) = &quest.local.data else {
                                unreachable!();
                            };

                            load_aliases(&esp_loader, id, data, &entity_loader).await
                        });

                        quest.state = QuestState::Starting(QuestStarting::Load(task));
                    }
                    QuestState::Starting(QuestStarting::Load(task)) => {
                        let Some(aliases) = task.get_output() else {
                            break;
                        };

                        let mut global_aliases = globals.quests.aliases.lock();
                        for (id, entity) in aliases {
                            let Some(alias) = quest.aliases.get(&id) else {
                                continue;
                            };

                            self.topics
                                .hello
                                .entry(entity.entity())
                                .or_default()
                                .push(*quest_id);

                            global_aliases.insert(
                                AliasKey {
                                    quest: *quest_id,
                                    alias: id,
                                },
                                Alias {
                                    is_optional: alias.is_optional,
                                    data: AliasData::Ref(Some(entity)),
                                },
                            );
                        }

                        let mut invocations = Vec::new();
                        for alias in quest.aliases.values() {
                            if let Some(script) = &alias.script {
                                let id = executor.invoke(AliasInit {
                                    object: script.clone(),
                                });
                                invocations.push(id);
                            }
                        }

                        quest.state = QuestState::Starting(QuestStarting::AliasInit(invocations));
                    }
                    QuestState::Starting(QuestStarting::AliasInit(invocations)) => {
                        invocations.retain_mut(|invoc| !invoc.is_done());
                        if !invocations.is_empty() {
                            break;
                        }

                        if let Some(script) = &quest.script {
                            let id = executor.invoke(QuestInit {
                                quest: script.clone(),
                            });

                            quest.state = QuestState::Starting(QuestStarting::QuestInit(id));
                        } else {
                            quest.state = QuestState::Running;
                        }
                    }
                    QuestState::Starting(QuestStarting::QuestInit(invoc)) => {
                        if !invoc.is_done() {
                            break;
                        }

                        quest.state = QuestState::Running;
                    }
                    QuestState::Running => {
                        for waker in self
                            .wakers
                            .on_quest_start
                            .remove(quest_id)
                            .unwrap_or_default()
                        {
                            waker.notify(waker.total_listeners().tag(true));
                        }

                        let Some(action) = quest.actions.pop_front() else {
                            break;
                        };

                        if action == QuestAction::Start {
                            continue;
                        }

                        match action {
                            QuestAction::Start => {}
                            QuestAction::Stop => {
                                tracing::debug!("stopping quest {:?}", quest_id);

                                if let Some(script) = &quest.script {
                                    let id = executor.invoke(QuestShutdown {
                                        object: script.clone(),
                                    });

                                    quest.state =
                                        QuestState::Stopping(QuestStopping::QuestShutdown(id));
                                } else {
                                    let mut invocations = Vec::new();
                                    for alias in quest.aliases.values() {
                                        if let Some(script) = &alias.script {
                                            let id = executor.invoke(AliasShutdown {
                                                object: script.clone(),
                                            });
                                            invocations.push(id);
                                        }
                                    }

                                    quest.state = QuestState::Stopping(
                                        QuestStopping::AliasShutdown(invocations),
                                    );
                                }
                            }
                            QuestAction::SetStage(stage_id) => {
                                let Some(stage) = quest.stages.get_mut(&stage_id) else {
                                    for waker in self
                                        .wakers
                                        .on_quest_set_stage
                                        .remove(&(*quest_id, stage_id))
                                        .unwrap_or_default()
                                    {
                                        waker.notify(waker.total_listeners().tag(false));
                                    }
                                    continue;
                                };

                                if stage.is_done {
                                    for waker in self
                                        .wakers
                                        .on_quest_set_stage
                                        .remove(&(*quest_id, stage_id))
                                        .unwrap_or_default()
                                    {
                                        waker.notify(waker.total_listeners().tag(false));
                                    }

                                    continue;
                                }

                                global.stages_done.insert(stage_id);
                                stage.is_done = true;
                                quest.active_stage = stage_id;

                                for waker in self
                                    .wakers
                                    .on_quest_set_stage
                                    .remove(&(*quest_id, stage_id))
                                    .unwrap_or_default()
                                {
                                    waker.notify(waker.total_listeners().tag(true));
                                }

                                tracing::debug!(
                                    "advancing quest {:?} from stage {:?} to {}",
                                    stage_id,
                                    quest.active_stage,
                                    stage_id
                                );

                                if let Some(script) = &quest.script {
                                    executor.invoke(QuestSetStage {
                                        quest: script.clone(),
                                        stage: stage_id,
                                        item: 0,
                                    });

                                    for name in &stage.fragment_functions {
                                        executor.invoke(QuestFragmentSetStage {
                                            quest: script.clone(),
                                            func_name: name.clone(),
                                        });
                                    }
                                }
                            }
                            QuestAction::StartScene(scene) => {
                                tracing::debug!(
                                    "starting scene {:?} of quest {:?}",
                                    scene,
                                    quest_id
                                );

                                if let Some(scene) = quest.scenes.get_mut(&scene) {
                                    if let Some(stage) = scene.info.set_stage_begin
                                        && !scene.is_playing
                                    {
                                        quest.actions.push_back(QuestAction::SetStage(stage));
                                    }

                                    scene.is_playing = true;
                                }
                            }
                            QuestAction::StopScene(scene) => {
                                tracing::debug!(
                                    "stopping scene {:?} of quest {:?}",
                                    scene,
                                    quest_id
                                );

                                if let Some(scene) = quest.scenes.get_mut(&scene) {
                                    scene.is_playing = false;
                                }
                            }
                        }
                    }
                    QuestState::Stopping(QuestStopping::QuestShutdown(invoc)) => {
                        if !invoc.is_done() {
                            break;
                        }

                        let mut invocations = Vec::new();
                        for alias in quest.aliases.values() {
                            if let Some(script) = &alias.script {
                                let id = executor.invoke(AliasShutdown {
                                    object: script.clone(),
                                });
                                invocations.push(id);
                            }
                        }

                        quest.state =
                            QuestState::Stopping(QuestStopping::AliasShutdown(invocations));
                    }
                    QuestState::Stopping(QuestStopping::AliasShutdown(invocations)) => {
                        invocations.retain_mut(|invoc| !invoc.is_done());
                        if !invocations.is_empty() {
                            break;
                        }

                        let mut aliases = globals.quests.aliases.lock();
                        for id in quest.aliases.keys() {
                            if let Some(alias) = aliases.get_mut(&global::AliasKey {
                                quest: *quest_id,
                                alias: *id,
                            }) {
                                alias.data.clear();
                            }
                        }

                        quest.state = QuestState::Stopped;
                    }
                }
            }

            global.cur_stage = quest.active_stage;
            global.state = match quest.state {
                QuestState::Init(_) | QuestState::PreInit | QuestState::Stopped => {
                    global::QuestState::Stopped
                }
                QuestState::Running => global::QuestState::Running,
                QuestState::Starting(_) => global::QuestState::Starting,
                QuestState::Stopping(_) => global::QuestState::Stopping,
            }
        }
    }

    fn advance_scenes(
        &mut self,
        world: &mut World,
        events: &mut EventBus,
        executor: &mut Executor,
        globals: &GlobalProperties,
    ) {
        let _span = trace_span!("QuestManager::advance_scenes").entered();

        let mut set_stages = Vec::new();

        for (quest_id, quest) in &mut self.quests {
            for (scene_id, scene) in &mut quest.scenes {
                if !scene.is_playing {
                    continue;
                }

                let is_done = scene.advance(world, events, executor, *quest_id, globals);
                if is_done {
                    tracing::debug!("scene {scene_id} of quest {quest_id} is done");
                    scene.is_playing = false;
                }

                if let Some(stage) = scene.info.set_stage_end
                    && is_done
                {
                    set_stages.push((*quest_id, stage));
                }
            }
        }

        for (quest, stage) in set_stages {
            self.set_quest_stage(quest, stage);
        }
    }

    pub fn get_scene(&self, scene: RecordId) -> Option<&SceneState> {
        let quest = self.scenes_to_quest.get(&scene)?;
        let quest = self.quests.get(quest)?;
        quest.scenes.get(&scene)
    }

    pub fn get(&self, quest: RecordId) -> Option<&Quest> {
        self.quests.get(&quest)
    }

    pub fn start_scene(&mut self, scene: RecordId) {
        let Some(quest) = self.scenes_to_quest.get(&scene) else {
            return;
        };

        let quest = self.quests.get_mut(quest).unwrap();
        quest.actions.push_back(QuestAction::StartScene(scene));
    }

    fn set_quest_stage(&mut self, quest: RecordId, id: StageId) {
        let Some(quest) = self.quests.get_mut(&quest) else {
            return;
        };

        quest.actions.push_back(QuestAction::SetStage(id));
    }
}

#[derive(Debug)]
pub struct Quest {
    pub state: QuestState,
    pub stages: IndexMap<StageId, QuestStage>,
    pub active_stage: StageId,
    script: Option<ScriptObject>,
    aliases: HashMap<u32, AliasState>,
    actions: VecDeque<QuestAction>,
    scenes: HashMap<RecordId, SceneState>,
    /// The topics when the player initiates dialogue with an actor.
    hello_topics: Vec<DialogueTopicAction>,
}

#[derive(Debug)]
pub enum QuestState {
    /// The quest is loaded, but `OnInit` was not called yet.
    PreInit,
    /// `OnInit` is being called on the quest.
    Init(Invocation),
    Starting(QuestStarting),
    /// The quest is currently active.
    Running,
    Stopping(QuestStopping),
    Stopped,
}

#[derive(Debug)]
pub enum QuestStarting {
    /// The quest is loading some data before being started.
    Load(Task<HashMap<u32, EntityRef>>),
    /// `OnAliasInit` is being called on all aliases.
    AliasInit(Vec<Invocation>),
    /// `OnQuestInit` is being called on the quest.
    QuestInit(Invocation),
}

#[derive(Debug)]
pub enum QuestStopping {
    /// `OnQuest` is being called on the quest.
    QuestShutdown(Invocation),
    /// `OnQuestShutdown` is being called on all aliases.
    AliasShutdown(Vec<Invocation>),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum QuestAction {
    Start,
    Stop,
    SetStage(StageId),
    StartScene(RecordId),
    StopScene(RecordId),
}

#[derive(Debug)]
enum Event {
    Discover(RecordId, Option<Quest>),
}

pub type StageId = u16;

#[derive(Clone, Debug)]
pub struct QuestStage {
    pub run_on_start: bool,
    pub run_on_stop: bool,
    pub is_done: bool,
    pub fragment_functions: HashSet<String>,
}

#[tracing::instrument(skip_all)]
async fn load_aliases(
    esp_loader: &EspLoader,
    quest_id: RecordId,
    quest: &openfo4_esp::records::Quest,
    entity_loader: &EntityLoader,
) -> HashMap<u32, EntityRef> {
    let mut aliases = HashMap::new();

    for alias in &quest.aliases.references {
        let entity = load_reference_alias(esp_loader, entity_loader, quest_id, quest, alias)
            .await
            .unwrap();

        if !alias.flags.contains(QuestAliasFlags::OPTIONAL) && entity.is_none() {
            todo!("alias was not optional, but didn't resolve");
        }

        if let Some(entity) = entity {
            tracing::debug!(
                "alias {} for quest {:?} bound to entity {:?}",
                alias.id,
                quest_id,
                entity.entity()
            );

            aliases.insert(alias.id, entity);
        }
    }

    for _alias in &quest.aliases.locations {
        todo!("TODO: location alias");
    }

    for _alias in &quest.aliases.ref_collections {
        todo!("TODO: ref coll alias");
    }

    aliases
}

async fn load_reference_alias(
    esp_loader: &EspLoader,
    loader: &EntityLoader,
    quest_id: RecordId,
    quest_data: &openfo4_esp::records::Quest,
    alias: &QuestReferenceAlias,
) -> Result<Option<EntityRef>, AliasLoadError> {
    if let Some(form) = alias.forced_reference {
        let id = esp_loader.resolve_form(form, quest_id).unwrap();
        let entity = loader.load(id, PartialEntityFlags::NONE).await.unwrap();
        return Ok(Some(entity));
    }

    // A unique actor is a reference to a NPC record.
    // In this case the Quest record must have a Location and that
    // location links the NPC record to a PlacedActor record.
    if let Some(npc_form) = alias.unique_actor {
        let npc_id = esp_loader.resolve_form(npc_form, quest_id).unwrap();

        let Some(location) = quest_data.location else {
            return Err(AliasLoadError::NoLocation);
        };

        let location_id = esp_loader.resolve_form(location, quest_id).unwrap();
        let RecordData::Location(location) = esp_loader
            .lookup(Some(RecordKind::Location), location_id)
            .await
            .unwrap()
            .local
            .data
        else {
            unreachable!();
        };

        for entry in location.unique_npcs.iter().flatten() {
            let entry_npc = esp_loader.resolve_form(entry.actor, location_id).unwrap();
            let entry_ref = esp_loader
                .resolve_form(entry.reference, location_id)
                .unwrap();

            if entry_npc == npc_id {
                let entity = loader
                    .load(entry_ref, PartialEntityFlags::NONE)
                    .await
                    .unwrap();
                return Ok(Some(entity));
            }
        }

        todo!();
    }

    todo!("TODO: {:?}", alias)
}

#[derive(Clone, Debug)]
enum AliasLoadError {
    NoLocation,
}

#[derive(Clone, Debug, Default)]
struct AliasState {
    is_optional: bool,
    script: Option<ScriptObject>,
}

#[derive(Clone, Debug, Default)]
struct DialogueTopics {
    hello: HashMap<Entity, Vec<RecordId>>,
}

#[derive(Debug, Default)]
struct Wakers {
    on_quest_start: HashMap<RecordId, Vec<event_listener::Event<bool>>>,
    on_quest_stop: HashMap<RecordId, Vec<event_listener::Event>>,
    on_quest_set_stage: HashMap<(RecordId, StageId), Vec<event_listener::Event<bool>>>,
}
