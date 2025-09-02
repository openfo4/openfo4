use crate::nodes::{Context, GraphNode};
use crate::{ValueExpression, VariableId};

#[derive(Clone, Debug)]
pub struct AssignVariables {
    /// Only assign variables if this condition evaluates to true.
    pub condition: ValueExpression,
    pub variables: Vec<AssignVariable>,
}

impl GraphNode for AssignVariables {
    fn activate<T>(&mut self, ctx: T)
    where
        T: Context,
    {
        self.update(ctx);
    }

    fn update<T>(&mut self, mut ctx: T)
    where
        T: Context,
    {
        let enable = self.condition.eval(ctx.variables()).to_bool().unwrap();
        if !enable {
            return;
        }

        for var in &self.variables {
            let value = var.value.eval(ctx.variables());
            ctx.set_variable(var.variable, value);
        }
    }
}

#[derive(Clone, Debug)]
pub struct AssignVariable {
    pub variable: VariableId,
    pub value: ValueExpression,
}
