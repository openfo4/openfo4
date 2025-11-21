use openfo4_script::runtime::{Caller, RawVal};
use openfo4_script::{Engine, Registry, classes};

fn main() {
    let mut registry = Registry::new();

    // let mut files = Vec::new();
    // let mut dirs = vec![PathBuf::from("../../archive/raw/Data/scripts")];

    // while let Some(dir) = dirs.pop() {
    //     for entry in std::fs::read_dir(dir).unwrap() {
    //         let entry = entry.unwrap();

    //         if entry.file_type().unwrap().is_dir() {
    //             dirs.push(entry.path());
    //         } else {
    //             files.push(entry.path());
    //         }
    //     }
    // }

    let mut engine = Engine::new();

    for p in [
        "../../archive/raw/Data/scripts/math.pex",
        "../../archive/raw/Data/scripts/faction.pex",
        "../../archive/raw/Data/scripts/keyword.pex",
        "../../archive/raw/Data/scripts/location.pex",
        "../../archive/raw/Data/scripts/form.pex",
        "../../archive/raw/Data/scripts/actorbase.pex",
        "../../archive/raw/Data/scripts/race.pex",
        "../../archive/raw/Data/scripts/outfit.pex",
        "../../archive/raw/Data/scripts/outputmodel.pex",
        "../../archive/raw/Data/scripts/formlist.pex",
        "../../archive/raw/Data/scripts/message.pex",
        "../../archive/raw/Data/scripts/actor.pex",
        "../../archive/raw/Data/scripts/spell.pex",
        "../../archive/raw/Data/scripts/class.pex",
        "../../archive/raw/Data/scripts/cell.pex",
        "../../archive/raw/Data/scripts/worldspace.pex",
        "../../archive/raw/Data/scripts/locationreftype.pex",
        "../../archive/raw/Data/scripts/perk.pex",
        "../../archive/raw/Data/scripts/package.pex",
        "../../archive/raw/Data/scripts/quest.pex",
        "../../archive/raw/Data/scripts/alias.pex",
        "../../archive/raw/Data/scripts/globalvariable.pex",
        "../../archive/raw/Data/scripts/magiceffect.pex",
        "../../archive/raw/Data/scripts/objectmod.pex",
        "../../archive/raw/Data/scripts/key.pex",
        "../../archive/raw/Data/scripts/topic.pex",
        "../../archive/raw/Data/scripts/idle.pex",
        "../../archive/raw/Data/scripts/scene.pex",
        "../../archive/raw/Data/scripts/voicetype.pex",
        "../../archive/raw/Data/scripts/encounterzone.pex",
        "../../archive/raw/Data/scripts/referencealias.pex",
        "../../archive/raw/Data/scripts/associationtype.pex",
        "../../archive/raw/Data/scripts/impactdataset.pex",
        "../../archive/raw/Data/scripts/textureset.pex",
        "../../archive/raw/Data/scripts/component.pex",
        "../../archive/raw/Data/scripts/armor.pex",
        "../../archive/raw/Data/scripts/headpart.pex",
        "../../archive/raw/Data/scripts/combatstyle.pex",
        "../../archive/raw/Data/scripts/weapon.pex",
        "../../archive/raw/Data/scripts/action.pex",
        "../../archive/raw/Data/scripts/ammo.pex",
        "../../archive/raw/Data/scripts/scriptobject.pex",
        "../../archive/raw/Data/scripts/miscobject.pex",
        "../../archive/raw/Data/scripts/projectile.pex",
        "../../archive/raw/Data/scripts/objectreference.pex",
        "../../archive/raw/Data/scripts/actorvalue.pex",
        "../../archive/raw/Data/scripts/yourespecialscript.pex",
        "../../archive/raw/Data/scripts/game.pex",
        "../../archive/raw/Data/scripts/camerashot.pex",
        "../../archive/raw/Data/scripts/sound.pex",
        "../../archive/raw/Data/scripts/commonpropertiesscript.pex",
    ] {
        let buf = std::fs::read(p).unwrap();
        let m = openfo4_pex::Module::decode(&buf[..]).unwrap();

        let x = engine.register(&m);
        dbg!(x);
    }

    fn show(
        ctx: Caller<'_, ()>,
        self_ptr: isize,
        arg1: f32,
        arg2: f32,
        arg3: f32,
        arg4: f32,
        arg5: f32,
        arg6: f32,
        arg7: f32,
        arg8: f32,
        arg9: f32,
    ) -> i32 {
        println!("Message.show");
        0
    }

    classes::register_builtins(&mut engine);

    let instance = engine.compile("YoureSPECIALscript");

    let func = instance.get_func("onactivate").unwrap();
    dbg!(&func);

    let typed = func.typed::<(isize, isize), ()>().unwrap();
    typed.call(&engine.state, (0, 0));

    // unsafe {
    //     func.call_unchecked(
    //         &engine.state,
    //         &mut [
    //             RawVal {
    //                 ptr: 0 as *const u8,
    //             },
    //             RawVal {
    //                 ptr: 0 as *const u8,
    //             },
    //         ],
    //     );
    // }

    println!("exit");
}
