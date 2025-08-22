use openfo4_hkx::behavior::BSModifyOnceModifier;

use crate::builder::{BuildNode, BuilderContext, Dependency, Error, UnknownVariableName};
use crate::{Command, Node};

impl BuildNode for BSModifyOnceModifier {
    fn dependencies(&self, _ctx: &mut BuilderContext<'_>) -> Result<Vec<Dependency>, Error> {
        let mut children = Vec::new();

        if let Some(modifier) = &self.p_on_activate_modifier {
            children.push(Dependency::Node(modifier.ptr));
        }

        if let Some(modifier) = &self.p_on_deactivate_modifier {
            children.push(Dependency::Node(modifier.ptr));
        }

        Ok(children)
    }

    fn build(&self, ctx: &mut BuilderContext<'_>) -> Result<Option<Node>, Error> {
        if let Some(set) = &self.parent.node.bindable.variable_binding_set {
            for binding in &*set.bindings {
                match &*binding.member_path {
                    name => {
                        return Err(Error::UnknownVariableName(UnknownVariableName {
                            class: "BSModifyOnceModifier",
                            name: name.to_owned(),
                        }));
                    }
                }
            }
        }

        let mut on_activate = Vec::new();
        let mut on_deactivate = Vec::new();

        if let Some(modifier) = &self.p_on_activate_modifier
            && let Some(node) = ctx.get_node(modifier.ptr)
        {
            on_activate.push(Command::ActivateNode(node));
            on_activate.push(Command::DeactivateNode(node));
        }

        if let Some(modifier) = &self.p_on_deactivate_modifier
            && let Some(node) = ctx.get_node(modifier.ptr)
        {
            on_deactivate.push(Command::ActivateNode(node));
            on_deactivate.push(Command::DeactivateNode(node));
        }

        Ok(Some(Node {
            name: self.parent.node.name.clone(),
            on_activate,
            on_deactivate,
            ..Default::default()
        }))
    }
}
