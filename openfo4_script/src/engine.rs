use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::mem::transmute;
use std::sync::{Arc, LazyLock, RwLock};

use game_common::string::Lowercase;
use openfo4_jit::Type;
use openfo4_pex::{Class, Module};

use crate::compiler::{CompilationResult, Context, NativeFunction, TypeLayout};
use crate::runtime::host_functions::call_host_trampoline;
use crate::runtime::{Func, HostState, IntoFunc, NativeFnPtr, ObjectPool, RawVal};

pub(crate) type NativeFuncPtr = unsafe extern "C" fn(*const HostState, usize, *mut RawVal);

#[derive(Clone, Debug)]
pub enum Error {
    UndeclaredType(String),
    DefaultStateMissing(String),
    UndeclaredMethod { class: String, function: String },
}

pub struct Engine {
    /// Compilation context.
    ctx: Context,
    pub(crate) shared: Arc<EngineShared>,
    layouts: HashMap<Lowercase<String>, TypeLayout>,
    instances: HashMap<Lowercase<String>, Arc<InstanceShared>>,
    classes: HashMap<Lowercase<String>, Class>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            ctx: Context::new(),
            shared: Arc::new(EngineShared::default()),
            layouts: HashMap::new(),
            instances: HashMap::new(),
            classes: HashMap::new(),
        }
    }

    pub fn register(&mut self, module: &Module) {
        self.ctx.register_module(module).unwrap();

        for class in &module.classes {
            self.classes
                .insert(Lowercase::new(class.name.as_str()), class.clone());
        }
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

        self.ctx.define_native_function(
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
        let class_name = Lowercase::new(class);

        if let Some(shared) = self.instances.get(&class_name) {
            return Instance {
                shared: shared.clone(),
            };
        }

        let class = self.classes.get(&class_name).unwrap();

        let mut funcs = HashMap::new();
        for s in &class.states {
            for f in &s.functions {
                let func_name = Lowercase::new(f.name.as_str());

                if funcs.contains_key(&func_name) {
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

                let res = self.ctx.compile(&class.name, &f.name, options).unwrap();

                let func = unsafe { Func::new(&res) };

                funcs.insert(func_name, (res, func));
            }
        }

        let layouts = self.ctx.compute_class_layouts(&class.name).unwrap();
        self.layouts.extend(layouts);

        let shared = Arc::new(InstanceShared {
            funcs,
            events_only: options.only_events,
        });

        self.instances.insert(class_name, shared.clone());
        Instance { shared }
    }

    pub fn get_class_layout(&self, class: &str) -> Option<&TypeLayout> {
        self.layouts.get(&Lowercase::new(class))
    }

    pub fn object_pool(&self) -> &ObjectPool {
        &self.shared.state.pool
    }
}

impl Debug for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Engine").finish_non_exhaustive()
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
    /// Use stub functions for all undefined native functions.
    ///
    /// Exists purely for compile testing and may result in crashes when calling into functions
    /// which call stub functions.
    ///
    /// Default: `false`
    pub use_native_stubs: bool,
}

#[derive(Default)]
pub(crate) struct EngineShared {
    pub(crate) call_table_lock: RwLock<()>,
    pub(crate) state: HostState,
}

impl Debug for EngineShared {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngineShared").finish_non_exhaustive()
    }
}

#[derive(Clone, Debug)]
pub struct ClassId(pub String);

/// An compiled instance of a script.
#[derive(Clone, Debug)]
pub struct Instance {
    shared: Arc<InstanceShared>,
}

impl Instance {
    pub fn get_func(&self, name: &str) -> Option<&Func> {
        if self.shared.events_only {
            // debug_assert!(
            //     EVENT_NAMES.contains(name),
            //     "{:?} not registered as engine event name",
            //     name
            // );
        }

        let (_, func) = self.shared.funcs.get(&Lowercase::new(name))?;
        Some(func)
    }
}

#[derive(Debug)]
struct InstanceShared {
    funcs: HashMap<Lowercase<String>, (CompilationResult, Func)>,
    events_only: bool,
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
