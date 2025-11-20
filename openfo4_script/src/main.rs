use openfo4_script::{ClassId, Registry};

fn main() {
    let mut registry = Registry::new();

    for p in [
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

        let x = registry.register(&m).unwrap();
        dbg!(x);
    }

    registry.define_native_function("Game", "GetPlayer", 420 as *const u8);
    registry.define_native_function("ObjectReference", "SetValue", 69 as *const u8);
    registry.define_native_function("ObjectReference", "GetBaseValue", 69 as *const u8);
    registry.define_native_function("Message", "Show", 69 as *const u8);

    registry
        .compile(ClassId("YoureSPECIALscript".to_lowercase()))
        .unwrap();
}
