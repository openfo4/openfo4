use crate::nodes::{Context, GraphNode};
use crate::{Value, ValueExpression, VariableId};

#[derive(Clone, Debug)]
pub struct IsActive {
    pub condition: ValueExpression,
    pub variables: Vec<VariableId>,
    pub inverted_variables: Vec<VariableId>,
    pub is_active: bool,
}

impl GraphNode for IsActive {
    fn activate<T>(&mut self, mut ctx: T)
    where
        T: Context,
    {
        if !self.condition.eval(ctx.variables()).to_bool().unwrap() {
            return;
        }

        self.is_active = true;

        for var in &self.variables {
            ctx.set_variable(*var, Value::Bool(true));
        }

        for var in &self.inverted_variables {
            ctx.set_variable(*var, Value::Bool(false));
        }
    }

    fn deactivate<T>(&mut self, mut ctx: T)
    where
        T: Context,
    {
        self.is_active = false;

        for var in &self.variables {
            ctx.set_variable(*var, Value::Bool(false));
        }

        for var in &self.inverted_variables {
            ctx.set_variable(*var, Value::Bool(true));
        }
    }
}
