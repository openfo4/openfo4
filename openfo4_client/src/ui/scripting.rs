use openfo4_egui::egui::{CollapsingHeader, Context, ScrollArea, Window};

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
                        let object = &script.get().object;

                        let ty = engine.get_type_by_id(typeid).unwrap();

                        for (class, name, prop) in ty.info().properties.iter() {
                            // if class.as_inner() != script.name() {
                            //     continue;
                            // }

                            let ty = engine.get_type_by_id(prop.ty).unwrap();

                            let data = match ty.id() {
                                id if id == engine.bool_type() => object
                                    .try_get::<bool>(class, name.as_inner())
                                    .map(|v| v.map(|v| v.to_string())),
                                id if id == engine.int_type() => object
                                    .try_get::<i32>(class, name.as_inner())
                                    .map(|v| v.map(|v| v.to_string())),
                                id if id == engine.float_type() => object
                                    .try_get::<f32>(class, name.as_inner())
                                    .map(|v| v.map(|v| v.to_string())),
                                // TODO
                                _ => continue,
                            };

                            let data = data.map(|v| v.unwrap());

                            ui.label(format!("{}: {}", name, data.as_deref().unwrap_or("locked")));
                        }
                    }
                });
            }
        });
    });
}
