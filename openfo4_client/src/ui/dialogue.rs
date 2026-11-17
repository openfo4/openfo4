use openfo4_egui::egui::{Context, Window};
use openfo4_game::state::GameState;
use openfo4_game::systems::quests::dialogue::PlayerDialogueQueue;

pub fn draw(ctx: &Context, state: &mut GameState) {
    Window::new("Dialogue").show(ctx, |ui| {
        for mut queue in state.world.query_mut::<&mut PlayerDialogueQueue>() {
            let Some(front) = queue.front() else {
                continue;
            };

            for (index, opt) in front.options.iter().enumerate() {
                if ui.button(opt).clicked() {
                    front.selector.set(index).unwrap();
                    queue.pop_front();
                    break;
                }
            }
        }
    });
}
