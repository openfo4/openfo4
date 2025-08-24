//! BSEventsEveryNEventsModifier

use openfo4_hkx::behavior::BSEventEveryNEventsModifier;

use crate::{
    Command, Listener, ListenerEvent, Node, Value, ValueExpression,
    builder::{BuildNode, BuilderContext, Error, UnknownVariableName, parse_event},
};

impl BuildNode for BSEventEveryNEventsModifier {
    fn build(&self, ctx: &mut BuilderContext<'_>) -> Result<Option<Node>, Error> {
        let mut enable = ValueExpression::Const(Value::Bool(self.parent.enable));

        if let Some(set) = &self.parent.node.bindable.variable_binding_set {
            for binding in &*set.bindings {
                let variable = ctx
                    .get_variable(binding.binding_type, binding.variable_index)
                    .unwrap();

                match &*binding.member_path {
                    "enable" => {
                        enable = ValueExpression::Variable(variable);
                    }
                    name => {
                        return Err(Error::UnknownVariableName(UnknownVariableName {
                            class: "BSEventsEveryNEventsModifier",
                            name: name.to_owned(),
                        }));
                    }
                }
            }
        }

        let listen_event = parse_event(ctx, &self.event_to_check_for);
        let send_event = parse_event(ctx, &self.event_to_send);

        if self.randomize_number_of_events {
            // todo!()
        }

        let counter = ctx.create_variable(Value::I32(0));

        let mut listeners = vec![Listener {
            event: ListenerEvent::Event(listen_event.id),
            reaction: vec![Command::Conditional(enable, vec![])],
        }];

        Ok(None)
    }
}
