use game_egui::egui::{CollapsingHeader, Context, ScrollArea, Window};
use openfo4_script::compiler::resources::PropertyType;

use crate::state::GameState;

#[derive(Clone, Debug, Default)]
pub struct State {
    filter: String,
}

pub fn draw_objects(ctx: &Context, game: &GameState, state: &mut State) {
    let objects = game.executor.objects();
    let engine = game.executor.engine();

    Window::new("Objects").show(ctx, |ui| {
        ui.text_edit_singleline(&mut state.filter);

        ScrollArea::vertical().show(ui, |ui| {
            for object in objects.iter() {
                let name = match object.record() {
                    Some(record) => format!("Object {:?} ({})", object.id(), record),
                    None => format!("Object {:?}", object.id()),
                };

                if !name.contains(&state.filter) {
                    continue;
                }

                CollapsingHeader::new(name).show(ui, |ui| {
                    for script in object.iter() {
                        ui.heading(script.name());

                        let typeid = script.get().layout;
                        let info = engine.type_info(typeid);

                        let object = engine.object_pool().get(&script.get().object).unwrap();

                        for ((class, prop), info) in &info.properties {
                            if class.as_inner() != script.name() {
                                continue;
                            }

                            let data = unsafe {
                                match info.ty {
                                    PropertyType::Inline(ty) if ty == engine.bool_type() => object
                                        .get_typed::<i8>(class.as_inner(), prop.as_inner())
                                        .unwrap()
                                        .to_string(),
                                    PropertyType::Inline(ty) if ty == engine.int_type() => object
                                        .get_typed::<i32>(class.as_inner(), prop.as_inner())
                                        .unwrap()
                                        .to_string(),
                                    PropertyType::Inline(ty) if ty == engine.float_type() => object
                                        .get_typed::<f32>(class.as_inner(), prop.as_inner())
                                        .unwrap()
                                        .to_string(),
                                    PropertyType::Pointer(_ty) => object
                                        .get_typed::<usize>(class.as_inner(), prop.as_inner())
                                        .unwrap()
                                        .to_string(),
                                    // TODO
                                    _ => continue,
                                }
                            };

                            ui.label(format!("{}: {}", prop, data));
                        }
                    }
                });
            }
        });
    });
}
