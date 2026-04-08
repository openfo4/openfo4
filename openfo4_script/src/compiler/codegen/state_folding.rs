use game_common::string::Lowercase;
use hashbrown::HashMap;
use openfo4_pex::{
    Binop, CallMethod, Class, ConditionalBranch, Function, Instruction, Property, State, Value,
    Variable, VariableDefinition,
};

pub fn fold_states(class: &mut Class) {
    for var in &class.variables {
        if var.definition.name.eq_ignore_ascii_case("::state") {
            panic!("the variable name ::state is reserved");
        }
    }

    class.variables.push(Variable {
        definition: VariableDefinition {
            name: "::state".to_owned(),
            ty: "string".to_owned(),
        },
        value: Value::String(class.default_state.clone()),
        is_const: false,
    });

    class.properties.push(Property {
        name: format!("{}.::state", class.name),
        ty: "string".to_owned(),
        variable: Some("::state".to_owned()),
        // FIXME: This is where we should call OnBeginState / OnEndState
        setter: None,
        getter: None,
    });

    if class.states.len() <= 1 {
        return;
    }

    let mut functions_per_state = HashMap::<_, Vec<_>>::new();

    for state in &mut class.states {
        for func in &mut state.functions {
            let old_name = Lowercase::new(&*func.name);
            let new_name = format!("__OF4_renamed__state__{}__func__{}", state.name, func.name);
            func.name = new_name;

            functions_per_state
                .entry(old_name)
                .or_default()
                .push((&state.name, func));
        }
    }

    let mut functions = Vec::new();

    for (func_name, funcs) in functions_per_state {
        debug_assert!(!funcs.is_empty());

        let calls = funcs
            .iter()
            .map(|(state, func)| (state.as_str(), func.name.as_str()))
            .collect::<Vec<_>>();

        let params = funcs[0].1.params.clone();
        let return_ty = funcs[0].1.return_ty.clone();

        let root = generate_root_func(func_name.into_inner(), params, return_ty, &calls);
        functions.push(root);
    }

    for state in class.states.drain(..) {
        functions.extend(state.functions);
    }

    class.states.push(State {
        name: "".to_owned(),
        functions,
    });
    class.default_state = "".to_owned();
}

fn generate_root_func(
    name: String,
    params: Vec<VariableDefinition>,
    return_ty: String,
    calls: &[(&str, &str)],
) -> Function {
    let locals = vec![
        VariableDefinition {
            name: "is_eq".to_owned(),
            ty: "bool".to_owned(),
        },
        VariableDefinition {
            name: "result".to_owned(),
            ty: return_ty.clone(),
        },
    ];

    let mut instructions = Vec::new();

    for (state, func) in calls {
        instructions.extend_from_slice(&[
            Instruction::CmpEq(Binop {
                lhs: Value::Ident("::state".to_owned()),
                rhs: Value::String(state.to_string()),
                dst: Value::Ident("is_eq".to_owned()),
            }),
            Instruction::BranchIfFalse(ConditionalBranch {
                condition: Value::Ident("is_eq".to_owned()),
                offset: Value::Int(3),
            }),
            Instruction::CallMethod(CallMethod {
                function_name: Value::Ident(func.to_string()),
                object: Value::Ident("self".to_owned()),
                args: params
                    .iter()
                    .map(|v| Value::Ident(v.name.clone()))
                    .collect(),
                return_assignment: Value::Ident("result".to_owned()),
            }),
            Instruction::Return(Value::Ident("result".to_owned())),
        ]);
    }

    instructions.push(Instruction::Return(Value::Ident("result".to_owned())));

    Function {
        name,
        is_static: false,
        is_native: false,
        params,
        return_ty,
        locals,
        instructions,
    }
}
