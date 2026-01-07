use std::path::PathBuf;

use openfo4_script::classes::{self, StateProvider};
use openfo4_script::classes::{CoreActorValues, State};
use openfo4_script::engine::{Engine, Registry};
use openfo4_script::runtime::{Caller, Context, Object, ObjectPool, RawVal, Stack};

fn main() {
    let mut registry = Registry::new();

    let mut files = Vec::new();
    let mut dirs = vec![PathBuf::from("../../archive/raw/Data/scripts")];

    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();

            if entry.file_type().unwrap().is_dir() {
                dirs.push(entry.path());
            } else {
                files.push(entry.path());
            }
        }
    }

    let mut engine = Engine::new();

    for p in
        // "../../archive/raw/Data/scripts/math.pex",
        // "../../archive/raw/Data/scripts/faction.pex",
        // "../../archive/raw/Data/scripts/keyword.pex",
        // "../../archive/raw/Data/scripts/location.pex",
        // "../../archive/raw/Data/scripts/form.pex",
        // "../../archive/raw/Data/scripts/actorbase.pex",
        // "../../archive/raw/Data/scripts/race.pex",
        // "../../archive/raw/Data/scripts/outfit.pex",
        // "../../archive/raw/Data/scripts/outputmodel.pex",
        // "../../archive/raw/Data/scripts/formlist.pex",
        // "../../archive/raw/Data/scripts/message.pex",
        // "../../archive/raw/Data/scripts/actor.pex",
        // "../../archive/raw/Data/scripts/spell.pex",
        // "../../archive/raw/Data/scripts/class.pex",
        // "../../archive/raw/Data/scripts/cell.pex",
        // "../../archive/raw/Data/scripts/worldspace.pex",
        // "../../archive/raw/Data/scripts/locationreftype.pex",
        // "../../archive/raw/Data/scripts/perk.pex",
        // "../../archive/raw/Data/scripts/package.pex",
        // "../../archive/raw/Data/scripts/quest.pex",
        // "../../archive/raw/Data/scripts/alias.pex",
        // "../../archive/raw/Data/scripts/globalvariable.pex",
        // "../../archive/raw/Data/scripts/magiceffect.pex",
        // "../../archive/raw/Data/scripts/objectmod.pex",
        // "../../archive/raw/Data/scripts/key.pex",
        // "../../archive/raw/Data/scripts/topic.pex",
        // "../../archive/raw/Data/scripts/idle.pex",
        // "../../archive/raw/Data/scripts/scene.pex",
        // "../../archive/raw/Data/scripts/voicetype.pex",
        // "../../archive/raw/Data/scripts/encounterzone.pex",
        // "../../archive/raw/Data/scripts/referencealias.pex",
        // "../../archive/raw/Data/scripts/associationtype.pex",
        // "../../archive/raw/Data/scripts/impactdataset.pex",
        // "../../archive/raw/Data/scripts/textureset.pex",
        // "../../archive/raw/Data/scripts/component.pex",
        // "../../archive/raw/Data/scripts/armor.pex",
        // "../../archive/raw/Data/scripts/headpart.pex",
        // "../../archive/raw/Data/scripts/combatstyle.pex",
        // "../../archive/raw/Data/scripts/weapon.pex",
        // "../../archive/raw/Data/scripts/action.pex",
        // "../../archive/raw/Data/scripts/ammo.pex",
        // "../../archive/raw/Data/scripts/scriptobject.pex",
        // "../../archive/raw/Data/scripts/miscobject.pex",
        // "../../archive/raw/Data/scripts/projectile.pex",
        // "../../archive/raw/Data/scripts/objectreference.pex",
        // "../../archive/raw/Data/scripts/actorvalue.pex",
        // "../../archive/raw/Data/scripts/yourespecialscript.pex",
        // "../../archive/raw/Data/scripts/game.pex",
        // "../../archive/raw/Data/scripts/camerashot.pex",
        // "../../archive/raw/Data/scripts/sound.pex",
        // "../../archive/raw/Data/scripts/commonpropertiesscript.pex",
        // "../../archive/raw/Data/scripts/followersscript.pex",
        // "../../archive/raw/Data/scripts/refcollectionalias.pex",
        // "../../archive/raw/Data/scripts/workshopparentscript.pex",
        // "../../archive/raw/Data/scripts/locationalias.pex",
        // "../../archive/raw/Data/scripts/workshopdatascript.pex",
        // "../../archive/raw/Data/scripts/leveleditem.pex",
        // "../../archive/raw/Data/scripts/workshopscript.pex",
        // "../../archive/raw/Data/scripts/topicinfo.pex",
        // "../../archive/raw/Data/scripts/workshopnpcscript.pex",
        // "../../archive/raw/Data/scripts/container.pex",
        // "../../archive/raw/Data/scripts/companionactorscript.pex",
        // "../../archive/raw/Data/scripts/workshopobjectscript.pex",
        // "../../archive/raw/Data/scripts/workshopobjectactorscript.pex",
        // "../../archive/raw/Data/scripts/movablestatic.pex",
        // "../../archive/raw/Data/scripts/flora.pex",
        // "../../archive/raw/Data/scripts/activator.pex",
        // "../../archive/raw/Data/scripts/teleportactorscript.pex",
        files
    {
        dbg!(&p);
        let buf = std::fs::read(&p).unwrap();
        let Ok(m) = openfo4_pex::Module::decode(&buf[..]) else {
            continue;
        };

        let x = engine.register(&m);
        dbg!(x);
    }

    classes::register_builtins(&mut engine);

    // engine.compile("Keyword");
    // engine.compile("ActorValue");

    let instance = engine.compile("Hardcore:HC_ManagerScript");
    // let instance = engine.compile("CastleArtilleryScrapScript");

    // dbg!(engine.get_class_layout("YoureSPECIALscript"));
    // dbg!(engine.get_class_layout("CastleArtilleryScrapScript"));

    let func = instance.get_func("onplayersleepstop").unwrap();
    // // dbg!(&func);

    dbg!(func.stack_size());

    let typed = func.typed::<(isize, isize), ()>().unwrap();

    let layout = engine.get_class_layout("PlayerSleepScript").unwrap();

    // let mut header = vec![0_usize; 256];
    // header[..3].copy_from_slice(&[3_usize, 709_usize, 69_usize]);
    // let object = header.as_ptr();
    // let object_ptr = &object as *const *const usize;
    let mut pool = ObjectPool::default();
    let player = pool
        .insert(engine.get_class_layout("Actor").unwrap().clone())
        .finalize();
    let health = pool
        .insert(engine.get_class_layout("ActorValue").unwrap().clone())
        .finalize();
    let strength = pool
        .insert(engine.get_class_layout("ActorValue").unwrap().clone())
        .finalize();
    let perception = pool
        .insert(engine.get_class_layout("ActorValue").unwrap().clone())
        .finalize();
    let endurance = pool
        .insert(engine.get_class_layout("ActorValue").unwrap().clone())
        .finalize();
    let charisma = pool
        .insert(engine.get_class_layout("ActorValue").unwrap().clone())
        .finalize();
    let intelligence = pool
        .insert(engine.get_class_layout("ActorValue").unwrap().clone())
        .finalize();
    let agility = pool
        .insert(engine.get_class_layout("ActorValue").unwrap().clone())
        .finalize();
    let luck = pool
        .insert(engine.get_class_layout("ActorValue").unwrap().clone())
        .finalize();
    let aggression = pool
        .insert(engine.get_class_layout("ActorValue").unwrap().clone())
        .finalize();
    let confidence = pool
        .insert(engine.get_class_layout("ActorValue").unwrap().clone())
        .finalize();
    let suspicous = pool
        .insert(engine.get_class_layout("ActorValue").unwrap().clone())
        .finalize();

    let object = pool.insert(layout.clone());

    let state = State {
        player,
        actor_values: CoreActorValues {
            health,
            strength,
            perception,
            endurance,
            charisma,
            intelligence,
            agility,
            luck,
            aggression,
            confidence,
            suspicious: suspicous,
        },
        provider: Box::new(TestProvider {}),
    };
    let mut ctx = Context::new(&engine, Stack::new(func.stack_size()), state);

    let object_ref = object.finalize();
    let object_ptr = object_ref.addr().to_raw();

    let res = typed.call(&mut ctx, (object_ptr as isize, object_ptr as isize));
    dbg!(&res);

    let res = typed.call(&mut ctx, (object_ptr as isize, 0));
    dbg!(&res);

    println!("exit");
}

struct TestProvider {}

impl StateProvider for TestProvider {
    fn get_actor_value(
        &self,
        object: openfo4_script::runtime::ObjectAddr,
        actor_value: openfo4_script::runtime::ObjectAddr,
    ) -> Option<f32> {
        todo!()
    }

    fn get_global(&self, global: openfo4_script::runtime::ObjectAddr) -> Option<f32> {
        todo!()
    }

    fn set_global(&self, global: openfo4_script::runtime::ObjectAddr, value: f32) {
        todo!()
    }

    fn set_actor_value(
        &self,
        object: openfo4_script::runtime::ObjectAddr,
        actor_value: openfo4_script::runtime::ObjectAddr,
        value: f32,
    ) {
        todo!()
    }

    fn get_owner(
        &self,
        object: openfo4_script::runtime::ObjectAddr,
    ) -> openfo4_script::runtime::ObjectAddr {
        todo!()
    }

    fn set_owner(
        &self,
        object: openfo4_script::runtime::ObjectAddr,
        owner: openfo4_script::runtime::ObjectAddr,
    ) {
        todo!()
    }

    fn linked_ref(
        &self,
        object: openfo4_script::runtime::ObjectAddr,
        keyword: openfo4_script::runtime::ObjectAddr,
    ) -> Option<openfo4_script::runtime::ObjectAddr> {
        todo!()
    }
}
