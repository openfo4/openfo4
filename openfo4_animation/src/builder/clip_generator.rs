use std::time::Duration;

use openfo4_hkx::behavior::{ClipGenerator, PlaybackMode};

use crate::builder::{BuildNode, BuilderContext, Error, UnknownVariableName};
use crate::{Clip, ClipTrigger, Node, NodeInner, Value, ValueExpression, parse_event};

impl BuildNode for ClipGenerator {
    fn build(&self, ctx: &mut BuilderContext<'_>) -> Result<Option<Node>, Error> {
        let mut speed = ValueExpression::Const(Value::F32(self.playback_speed));

        if let Some(set) = &self.parent.node.bindable.variable_binding_set {
            for binding in &*set.bindings {
                let variable = ctx
                    .get_variable(binding.binding_type, binding.variable_index)
                    .unwrap();

                match &*binding.member_path {
                    "playbackSpeed" => {
                        speed = ValueExpression::Variable(variable);
                    }
                    name => {
                        return Err(Error::UnknownVariableName(UnknownVariableName {
                            class: "hkbClipGenerator",
                            name: name.to_owned(),
                        }));
                    }
                }
            }
        }

        let mut triggers = Vec::new();
        if let Some(t) = &self.triggers {
            for trigger in &*t.triggers {
                let event = parse_event(ctx.hkx_file(), &trigger.event);

                let local_time = if trigger.relative_to_end_of_clip {
                    // The offset is from the end of the clip.
                    // If the `local_time` is > 0, then the trigger happens
                    // after the end of the clip, but such a case can never happen!
                    if trigger.local_time > 0.0 {
                        continue;
                    }

                    trigger.local_time.abs()
                } else {
                    // The offset is from the start of the clip.
                    // If the `local_time` is < 0, then the trigger happens
                    // before the end of the clip, but such a case can never happen!
                    if trigger.local_time < 0.0 {
                        continue;
                    }

                    trigger.local_time.abs()
                };

                triggers.push(ClipTrigger {
                    local_time: Duration::from_secs_f32(local_time),
                    from_end: trigger.relative_to_end_of_clip,
                    event,
                });
            }
        }

        let animation_name = self.animation_name.replace("\\", "/");

        Ok(Some(Node {
            name: format!("{} {}", self.parent.node.name, animation_name),
            inner: NodeInner::Clip(Clip {
                path: self.animation_name.clone(),
                triggers,
                looping: matches!(self.mode, PlaybackMode::Looping),
                speed,
            }),
            listeners: Vec::new(),
            on_activate: Vec::new(),
            on_deactivate: Vec::new(),
            on_update: Vec::new(),
        }))
    }
}
