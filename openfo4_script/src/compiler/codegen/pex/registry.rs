use std::hash::Hash;
use std::iter;

use game_common::string::Lowercase;
use hashbrown::HashMap;
use indexmap::IndexMap;
use openfo4_jit::Type as JitType;
use openfo4_pex::{Class, Function, Instruction, Struct, Value};
use slab::Slab;

#[derive(Clone, Debug, Default)]
pub struct Registry {
    classes: Slab<Option<ClassDef>>,
    class_by_name: HashMap<Lowercase<String>, ClassId>,
    structs: Slab<Option<StructDef>>,
    struct_by_name: HashMap<Lowercase<String>, StructId>,
    functions: Slab<Option<FunctionDef>>,
}

impl Registry {
    pub fn declare_class(&mut self, name: Lowercase<String>) -> ClassId {
        let id = ClassId(self.classes.insert(None));
        self.class_by_name.insert(name, id);
        id
    }

    pub fn define_class(&mut self, id: ClassId, class: Class) {
        debug_assert!(self.classes[id.0].is_none(), "cannot redefine class");
        let class_id = id;

        let parent = if class.parent.is_empty() || class.parent.eq_ignore_ascii_case("none") {
            None
        } else {
            let parent = Lowercase::new(&*class.parent);
            Some(*self.class_by_name.get(&parent).unwrap())
        };

        let mut fields = HashMap::with_capacity(class.variables.len());
        for var in class.variables {
            let name = Lowercase::new(&*var.definition.name);
            let ty = self.parse_type(&Lowercase::new(&*var.definition.ty));
            fields.insert(
                name,
                Field {
                    ty,
                    value: var.value,
                },
            );
        }

        let mut properties = HashMap::with_capacity(class.properties.len());
        for prop in class.properties {
            let name = Lowercase::new(&*prop.name);

            let field = prop.variable.map(|v| Lowercase::new(&*v));

            let getter = prop.getter.map(|func| {
                let id = self.declare_function();
                self.define_function(
                    id,
                    class_id,
                    iter::once(("".to_owned(), func)),
                    HashMap::new(),
                    None,
                    format!("::get({})", &prop.name),
                );
                id
            });

            let setter = prop.setter.map(|func| {
                let id = self.declare_function();
                self.define_function(
                    id,
                    class_id,
                    iter::once(("".to_owned(), func)),
                    HashMap::new(),
                    None,
                    format!("::set({})", &prop.name),
                );
                id
            });

            properties.insert(
                name,
                Property {
                    field,
                    getter,
                    setter,
                },
            );
        }

        let mut functions = FunctionSet::default();

        if let Some(parent_id) = parent {
            let parent = &self.classes[parent_id.0]
                .as_ref()
                .expect("parent class must be defined before subclasses are defined");

            for (name, func) in &parent.functions.inherited {
                functions.inherited.insert(name.clone(), *func);
            }

            // All overriden functions become inherited.
            for (name, func) in &parent.functions.overriden {
                functions.inherited.insert(
                    name.clone(),
                    InheritedFunction {
                        def_class: func.def_class,
                        def_function: func.def_function,
                        next_parent_impl: func.func,
                    },
                );
            }

            for (name, func) in &parent.functions.new {
                functions.inherited.insert(
                    name.clone(),
                    InheritedFunction {
                        def_class: parent_id,
                        def_function: *func,
                        next_parent_impl: *func,
                    },
                );
            }
        }

        let mut funcs_by_state = HashMap::<_, Vec<_>>::new();
        for state in class.states {
            for func in state.functions {
                funcs_by_state
                    .entry(func.name.clone())
                    .or_default()
                    .push((state.name.clone(), func));
            }
        }

        for (fn_name, states) in funcs_by_state {
            let name = Lowercase::new(&*fn_name);

            let parent_states = if let Some(func) = functions.inherited.get(&name) {
                self.functions[func.next_parent_impl.0]
                    .as_ref()
                    .unwrap()
                    .states
                    .keys()
                    .map(|id| (id.clone(), FunctionStateImpl::Call(func.next_parent_impl)))
                    .collect()
            } else {
                HashMap::new()
            };

            let id = self.declare_function();
            self.define_function(
                id,
                class_id,
                states,
                parent_states,
                Some(name.clone()),
                fn_name,
            );

            if self.function(id).unwrap().is_static {
                functions.new_static.insert(name, id);
                continue;
            }

            if let Some(func) = functions.inherited.remove(&name) {
                functions.overriden.insert(
                    name,
                    OverridenFunction {
                        def_class: func.def_class,
                        def_function: func.def_function,
                        next_parent_impl: func.next_parent_impl,
                        func: id,
                    },
                );
            } else {
                functions.new.insert(name, id);
            }
        }

        // The root class always has the builtin ::state field.
        if parent.is_none() {
            fields.insert(
                Lowercase::new("::state"),
                Field {
                    ty: Type::String,
                    value: Value::String(class.default_state),
                },
            );
            properties.insert(
                Lowercase::new("::state"),
                Property {
                    field: Some(Lowercase::new("::state")),
                    getter: None,
                    setter: None,
                },
            );
        }

        self.classes[id.0] = Some(ClassDef {
            id,
            name: Lowercase::new(&*class.name),
            parent,
            fields,
            properties,
            functions,
            debug_name: class.name,
        });
    }

    pub fn declare_struct(&mut self, name: Lowercase<String>) -> StructId {
        let id = StructId(self.structs.insert(None));
        self.struct_by_name.insert(name, id);
        id
    }

    pub fn define_struct(&mut self, id: StructId, st: Struct) {
        debug_assert!(self.structs[id.0].is_none(), "cannot redefine struct");

        let mut fields = HashMap::with_capacity(st.members.len());
        for member in st.members {
            let name = Lowercase::new(&*member.name);
            let ty = self.parse_type(&Lowercase::new(&*member.ty));
            fields.insert(name, ty);
        }

        self.structs[id.0] = Some(StructDef {
            name: Lowercase::new(&*st.name),
            fields,
            debug_name: st.name,
        });
    }

    fn declare_function(&mut self) -> FunctionId {
        FunctionId(self.functions.insert(None))
    }

    fn define_function<T>(
        &mut self,
        id: FunctionId,
        class: ClassId,
        states: T,
        parent_states: HashMap<StateId, FunctionStateImpl>,
        name: Option<Lowercase<String>>,
        debug_name: String,
    ) where
        T: IntoIterator<Item = (String, Function)>,
    {
        debug_assert!(self.functions[id.0].is_none(), "cannot redefine function");

        let mut states = states.into_iter();
        let (first_state, first_func) = states
            .next()
            .expect("at least one state must be defined for function");

        let params: Vec<_> = first_func
            .params
            .iter()
            .map(|param| self.parse_type(&Lowercase::new(&*param.ty)))
            .collect();

        let returns = self.parse_type(&Lowercase::new(&*first_func.return_ty));

        let is_static = first_func.is_static;
        let is_native = first_func.is_native;

        let mut fn_states = parent_states;
        for (state_name, func) in iter::once((first_state, first_func)).chain(states) {
            let id = if state_name.is_empty() {
                StateId::Default
            } else {
                StateId::Named(state_name)
            };

            let params: Vec<_> = func
                .params
                .iter()
                .map(|param| Lowercase::new(&*param.name))
                .collect();

            let mut locals = HashMap::with_capacity(func.locals.len());
            for local in &func.locals {
                let name = Lowercase::new(&*local.name);
                let ty = self.parse_type(&Lowercase::new(&*local.ty));
                locals.insert(name, ty);
            }

            fn_states.insert(
                id,
                FunctionStateImpl::Body(FunctionBody {
                    params,
                    locals,
                    instructions: func.instructions,
                }),
            );
        }

        self.functions[id.0] = Some(FunctionDef {
            id,
            params,
            returns,
            states: fn_states,
            is_static,
            is_native,
            class,
            debug_name,
            name,
        })
    }

    fn parse_type(&self, mut ty: &Lowercase<str>) -> Type {
        let mut array_depth = 0;
        while let Some(p) = ty.strip_suffix("[]") {
            ty = p;
            array_depth += 1;
        }

        let mut typ = match ty.as_inner() {
            "none" => Type::None,
            "bool" => Type::Bool,
            "int" => Type::Int,
            "float" => Type::Float,
            "string" => Type::String,
            "var" => Type::Var,
            _ if let Some(id) = self.struct_by_name.get(ty) => Type::Struct(*id),
            _ if let Some(id) = self.class_by_name.get(ty) => Type::Class(*id),
            _ => unreachable!(),
        };

        for _ in 0..array_depth {
            typ = Type::Array(Box::new(typ));
        }

        typ
    }

    pub fn class(&self, id: ClassId) -> Option<&ClassDef> {
        self.classes[id.0].as_ref()
    }

    pub fn class_by_name(&self, name: &Lowercase<str>) -> Option<&ClassDef> {
        self.class_by_name.get(name).and_then(|&id| self.class(id))
    }

    pub fn classes(&self) -> impl Iterator<Item = (ClassId, &ClassDef)> {
        self.classes
            .iter()
            .filter_map(|(id, v)| v.as_ref().map(|v| (ClassId(id), v)))
    }

    pub fn struct_(&self, id: StructId) -> Option<&StructDef> {
        self.structs.get(id.0).and_then(|v| v.as_ref())
    }

    pub fn function(&self, id: FunctionId) -> Option<&FunctionDef> {
        self.functions.get(id.0).and_then(|v| v.as_ref())
    }

    pub fn structs(&self) -> impl Iterator<Item = (StructId, &StructDef)> {
        self.structs
            .iter()
            .filter_map(|(id, v)| v.as_ref().map(|v| (StructId(id), v)))
    }
}

#[derive(Clone, Debug)]
pub struct ClassDef {
    pub id: ClassId,
    pub name: Lowercase<String>,
    pub parent: Option<ClassId>,
    pub fields: HashMap<Lowercase<String>, Field>,
    pub properties: HashMap<Lowercase<String>, Property>,
    pub functions: FunctionSet,
    /// The name to use for debugging purposes.
    pub debug_name: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ClassId(usize);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Type {
    None,
    Bool,
    Int,
    Float,
    String,
    Var,
    Struct(StructId),
    Class(ClassId),
    Array(Box<Type>),
}

impl Type {
    pub fn to_jit(&self) -> JitType {
        match self {
            Self::None => JitType::I8,
            Self::Bool => JitType::I8,
            Self::Int => JitType::I32,
            Self::Float => JitType::F32,
            Self::String | Self::Var | Self::Struct(_) | Self::Class(_) | Self::Array(_) => {
                JitType::Pointer
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Field {
    pub ty: Type,
    pub value: Value,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct StructId(usize);

#[derive(Clone, Debug)]
pub struct Property {
    pub field: Option<Lowercase<String>>,
    pub getter: Option<FunctionId>,
    pub setter: Option<FunctionId>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FunctionId(usize);

#[derive(Clone, Debug)]
pub struct StructDef {
    pub name: Lowercase<String>,
    pub fields: HashMap<Lowercase<String>, Type>,
    pub debug_name: String,
}

#[derive(Clone, Debug)]
pub struct FunctionDef {
    pub id: FunctionId,
    pub name: Option<Lowercase<String>>,
    pub params: Vec<Type>,
    pub returns: Type,
    pub states: HashMap<StateId, FunctionStateImpl>,
    pub is_static: bool,
    pub is_native: bool,
    /// The class where this function was defined.
    pub class: ClassId,
    /// The name to use for debugging purposes.
    pub debug_name: String,
}

/// The implementation info of a [`FunctionDef`].
///
/// This does not include signature information of the function.
#[derive(Clone, Debug)]
pub struct FunctionBody {
    pub params: Vec<Lowercase<String>>,
    pub locals: HashMap<Lowercase<String>, Type>,
    pub instructions: Vec<Instruction>,
}

#[derive(Clone, Debug, Default)]
pub struct FunctionSet {
    /// Static functions on this class.
    pub new_static: HashMap<Lowercase<String>, FunctionId>,
    /// Functions that are newly defined in this class.
    pub new: IndexMap<Lowercase<String>, FunctionId>,
    /// Functions that are overriding functions from another class.
    pub overriden: HashMap<Lowercase<String>, OverridenFunction>,
    /// Functions that are defined on a parent class.
    pub inherited: HashMap<Lowercase<String>, InheritedFunction>,
}

impl FunctionSet {
    /// Returns the function with the given `name`.
    ///
    /// This includes methods and static functions.
    pub fn get(&self, name: &Lowercase<str>) -> Option<FunctionId> {
        self.get_method(name).or_else(|| self.get_static(name))
    }

    /// Returns the method with the given `name`.
    ///
    /// This includes overriden and inherited functions from the parent.
    pub fn get_method(&self, name: &Lowercase<str>) -> Option<FunctionId> {
        self.new
            .get(name)
            .or_else(|| self.overriden.get(name).map(|f| &f.func))
            .or_else(|| self.inherited.get(name).map(|f| &f.next_parent_impl))
            .copied()
    }

    /// Returns the method with the given `name` from the parent class.
    pub fn get_parent(&self, name: &Lowercase<str>) -> Option<FunctionId> {
        self.overriden
            .get(name)
            .map(|f| f.next_parent_impl)
            .or_else(|| self.inherited.get(name).map(|f| f.next_parent_impl))
    }

    /// Returns the static function with the given `name`.
    pub fn get_static(&self, name: &Lowercase<str>) -> Option<FunctionId> {
        self.new_static.get(name).copied()
    }

    /// Returns an `Iterator` over all functions available in this set.
    ///
    /// This includes methods, including overriden and inherited functions, and static functions.
    pub fn iter(&self) -> impl Iterator<Item = FunctionId> {
        self.new
            .values()
            .chain(self.overriden.values().map(|f| &f.func))
            .chain(self.inherited.values().map(|f| &f.next_parent_impl))
            .chain(self.new_static.values())
            .copied()
    }
}

#[derive(Copy, Clone, Debug)]
pub struct OverridenFunction {
    /// The class where the function was originally declared.
    pub def_class: ClassId,
    /// The function that originally defined the function.
    pub def_function: FunctionId,
    /// The function in the next supertype that has the function defined.
    pub next_parent_impl: FunctionId,
    /// The implementation of the overriding function.
    pub func: FunctionId,
}

#[derive(Copy, Clone, Debug)]
pub struct InheritedFunction {
    /// The class where the function was originally declared.
    pub def_class: ClassId,
    /// The function that originally defined the function.
    pub def_function: FunctionId,
    /// The function in the next supertype that has the function defined.
    pub next_parent_impl: FunctionId,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum StateId {
    Default,
    Named(String),
}

#[derive(Clone, Debug)]
pub enum FunctionStateImpl {
    Call(FunctionId),
    Body(FunctionBody),
}
