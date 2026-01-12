use std::collections::{HashMap, HashSet};
use std::mem::transmute;
use std::sync::{Arc, LazyLock, RwLock};

use openfo4_jit::Type;
use openfo4_pex::{Class, Module};

use crate::compiler::{CompilationResult, Context, NativeFunction, TypeLayout};
use crate::runtime::{
    Func, HostState, IntoFunc, NativeFnPtr, ObjectPool, RawVal, call_host_trampoline,
};

pub type NativeFuncPtr = unsafe extern "C" fn(*const HostState, usize, *mut RawVal);

#[derive(Clone, Debug)]
pub enum Error {
    UndeclaredType(String),
    DefaultStateMissing(String),
    UndeclaredMethod { class: String, function: String },
}

pub struct Engine {
    registry: Registry,
    pub(crate) shared: Arc<EngineShared>,
    layouts: HashMap<String, TypeLayout>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            registry: Registry::new(),
            shared: Arc::new(EngineShared::default()),
            layouts: HashMap::new(),
        }
    }

    pub fn register(&mut self, module: &Module) {
        self.registry.register(module).unwrap();
    }

    pub fn register_native_function<F, T, Params, Returns>(&mut self, class: &str, func: &str, f: F)
    where
        F: IntoFunc<T, Params, Returns>,
    {
        let _lock = self.shared.call_table_lock.write();
        let mut call_table = unsafe { self.shared.state.call_table.borrow_mut() };

        let ptr = call_host_trampoline as *const u8;
        let index = f.into_func(&mut call_table);

        let mut signature = F::signature();
        // Our VM context pointer.
        signature.params.insert(0, Type::Pointer);
        // VM error
        signature.returns.push(Type::I32);

        self.registry.define_native_function(
            class,
            func,
            NativeFunction {
                ptr,
                index,
                signature,
            },
        );
    }

    pub fn compile(&mut self, class: &str, options: &Options) -> Instance {
        let (instance, layouts) = self
            .registry
            .compile(self.shared.clone(), class, options)
            .unwrap();
        self.layouts.extend(layouts);

        instance
    }

    pub fn get_class_layout(&self, class: &str) -> Option<&TypeLayout> {
        self.layouts.get(&class.to_lowercase())
    }

    pub fn object_pool(&self) -> &ObjectPool {
        &self.shared.state.pool
    }
}

#[derive(Clone, Debug, Default)]
pub struct Options {
    /// Skip compilation of native functions.
    pub skip_native: bool,
    /// Skip compliation of empty functions.
    pub skip_empty: bool,
    /// Only compile functions with predefined event signatures.
    pub only_events: bool,
}

#[derive(Default)]
pub(crate) struct EngineShared {
    pub(crate) call_table_lock: RwLock<()>,
    pub(crate) state: HostState,
}

#[derive(Clone, Debug)]
pub struct ClassId(pub String);

pub struct Registry {
    ctx: Context,
    native_functions: Vec<(String, String, NativeFunction)>,
    classes: HashMap<String, Class>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            classes: HashMap::new(),
            native_functions: Vec::new(),
            ctx: Context::new(),
        }
    }

    pub fn register(&mut self, module: &Module) -> Result<(), Error> {
        self.ctx.register_module(module).unwrap();

        for class in &module.classes {
            self.classes
                .insert(class.name.to_lowercase(), class.clone());
        }

        Ok(())
    }

    pub fn define_native_function(&mut self, class: &str, func: &str, f: NativeFunction) {
        // self.native_functions
        //     .push((class.to_lowercase(), func.to_lowercase(), f));

        self.ctx.define_native_function(class, func, f);
    }

    fn compile(
        &mut self,
        engine: Arc<EngineShared>,
        class: &str,
        options: &Options,
    ) -> Result<(Instance, HashMap<String, TypeLayout>), Error> {
        let mut functions = HashMap::new();

        let class = self.classes.get(&class.to_lowercase()).unwrap();

        for s in &class.states {
            for f in &s.functions {
                if functions.contains_key(&f.name.to_lowercase()) {
                    continue;
                }

                if options.skip_native && f.is_native {
                    continue;
                }

                if options.skip_empty && f.instructions.is_empty() {
                    continue;
                }

                if options.only_events && !is_event(f) {
                    continue;
                }

                let res = self.ctx.compile(&class.name, &f.name).unwrap();

                let func = unsafe { Func::new(&res) };

                functions.insert(f.name.to_lowercase(), (res, func));
            }
        }

        let layouts = self.ctx.compute_class_layouts(&class.name).unwrap();

        Ok((
            Instance {
                engine,
                functions,
                events_only: options.only_events,
            },
            layouts,
        ))
    }
}

/// An compiled instance of a script.
#[derive(Clone)]
pub struct Instance {
    engine: Arc<EngineShared>,
    functions: HashMap<String, (CompilationResult, Func)>,
    events_only: bool,
}

impl Instance {
    pub fn get_func(&self, name: &str) -> Option<&Func> {
        if self.events_only {
            debug_assert!(
                EVENT_NAMES.contains(name),
                "{:?} not registered as engine event name",
                name
            );
        }

        let (_, func) = self.functions.get(&name.to_lowercase())?;
        Some(func)
    }
}

impl std::fmt::Debug for Instance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            engine: _,
            functions,
            events_only,
        } = self;

        f.debug_struct("Instance")
            .field("functions", functions)
            .field("events_only", events_only)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug)]
pub struct ScriptInstance {
    functions: HashMap<String, CompilationResult>,
}

impl ScriptInstance {
    pub fn get_func(&self, name: &str) -> Option<Func> {
        debug_assert!(EVENT_NAMES.contains(name));

        let res = self.functions.get(&name.to_lowercase())?;

        Some(Func {
            ptr: unsafe { transmute::<*const u8, NativeFnPtr>(res.function.as_ptr()) },
            signature: res.function.signature().clone(),
            stack_size: res.stack_size,
        })
    }
}

#[derive(Clone, Debug)]
pub struct InstanceField {
    pub class: String,
    pub name: String,
    pub offset: usize,
}

fn is_event(f: &openfo4_pex::Function) -> bool {
    EVENT_NAMES.contains(&*f.name)
}

static EVENT_NAMES: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "OnAction",
        "OnActivate",
        "OnAliasInit",
        "OnAliasReset",
        "OnAliasShutdown",
        "OnAnimationEvent",
        "OnAninmationEventUnregistered",
        "OnBegin",
        "OnBeginState",
        "OnCellAttach",
        "OnCellDetach",
        "OnCellLoad",
        "OnChange",
        "OnClose",
        "OnCombatStateChanged",
        "OnCommandModeCompleteCommand",
        "OnCommandModeEnter",
        "OnCommandModeExit",
        "OnCommandModeGiveCommand",
        "OnCompanionDismiss",
        "OnConsciousnessStateChanged",
        "OnContainerChanged",
        "OnControlDown",
        "OnControlUp",
        "OnCripple",
        "OnDeath",
        "OnDeferredKill",
        "OnDestructionStageChanged",
        "OnDifficultyChanged",
        "OnDistanceGreaterThan",
        "OnDistanceLessThan",
        "OnDying",
        "OnEffectFinish",
        "OnEffectStart",
        "OnEnd",
        "OnEndState",
        "OnEnterBleedout",
        "OnEnterSneaking",
        "OnEntryRun",
        "OnEquipped",
        "OnEscortWaitStart",
        "OnEscortWaitStop",
        "OnExitFurniture",
        "OnFurnitureEvent",
        "OnGainLOS",
        "OnGetUp",
        "OnGrab",
        "OnHit",
        "OnHolotapeChatter",
        "OnHolotapePlay",
        "OnInit",
        "OnItemAdded",
        "OnItemEquipped",
        "OnItemRemoved",
        "OnItemUnequipped",
        "OnKeyDown",
        "OnKeyUp",
        "OnKill",
        "InLoad",
        "OnLocationChange",
        "OnLocationCleared",
        "OnLocationLoaded",
        "OnLockStateChanged",
        "OnLooksMenuEvent",
        "OnLostLOS",
        "OnMagicEffectApply",
        "OnMenuItemRun",
        "OnMenuOpenCloseEvent",
        "OnOpen",
        "OnPackageChange",
        "OnPackageEnd",
        "OnPackageStart",
        "OnPartialCripple",
        "OnPhaseBegin",
        "OnPhaseEnd",
        "OnPickpocketFailed",
        "OnPipboyRadioDetection",
        "OnPlayerCameraState",
        "OnPlayerCreateRobot",
        "OnPlayerDialogueTarget",
        "OnPlayerEnterVertibird",
        "OnPLayerFallLongDistance",
        "OnPlayerFireWeapon",
        "OnPlyerHealTeammate",
        "OnPlayerLoadGame",
        "OnPlayerModArmorWeapon",
        "OnPlayerModRobot",
        "OnPlayerSleepStart",
        "OnPlayerSleepStop",
        "OnPlayerSwimming",
        "OnPlayerTeleport",
        "OnPlayerUseWorkBench",
        "OnPlayerWaitStart",
        "OnPlayerWaitStop",
        "OnPowerOff",
        "OnPowerOn",
        "OnQuestInit",
        "OnQuestShutdown",
        "OnRaceSwitchComplete",
        "OnRadiationDamage",
        "OnRead",
        "OnRelease",
        "OnReset",
        "OnSell",
        "OnSit",
        "OnSpeechChallengeAvailable",
        "OnSpellCast",
        "OnStageSet",
        "OnStart",
        "OnStoryIncreaseLevel",
        "OnTimer",
        "OnTimerGameTime",
        "OnTrackedStatsEvent",
        "OnTranslationAlmostComplete",
        "OnTranslationComplte",
        "OnTranslationFailed",
        "OnTrapHitStart",
        "OnTrapHitStop",
        "OnTriggerEnter",
        "OnTriggerLeave",
        "OnTutorialEvent",
        "OnUnequipped",
        "OnUnload",
        "OnWorkshopMode",
        "OnWorkshopNPCTransfer",
        "OnWorkshopObjectDestroyed",
        "OnWorkshopObjectGrabbed",
        "OnWorkshopObjectMoved",
        "OnWorkshopObjectPlaced",
        "OnWorkshopObjectRepaired",
    ]
    .into_iter()
    .collect()
});
