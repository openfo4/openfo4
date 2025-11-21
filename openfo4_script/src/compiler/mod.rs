mod codegen;
mod typecheck;

use std::collections::HashMap;

use game_common::collections::arena::{Arena, Key};
use openfo4_jit::{CompiledFunction, Signature};
use openfo4_pex::{Instruction, Value, Variable, VariableDefinition};

use crate::Error;
use crate::compiler::codegen::CodegenPass;

#[derive(Clone, Debug)]
pub struct Context {
    types: Arena<Type>,
    primitives: Primitives,
    classes: HashMap<String, Class>,
    native_functions: Vec<(String, String, NativeFunction)>,
}

impl Context {
    pub fn new() -> Self {
        let mut types = Arena::new();

        let primitives = Primitives {
            none: TypeId(types.insert(Type::None)),
            bool: TypeId(types.insert(Type::Bool)),
            i32: TypeId(types.insert(Type::I32)),
            i64: TypeId(types.insert(Type::I64)),
            f32: TypeId(types.insert(Type::F32)),
        };

        Self {
            types,
            primitives,
            classes: HashMap::new(),
            native_functions: Vec::new(),
        }
    }

    fn resolve_ty(&mut self, mut ty: &str) -> Result<TypeId, Error> {
        if ty.contains("#") {
            return Ok(self.primitives.none);
        }

        let mut array_depth = 0;
        while let Some(p) = ty.strip_suffix("[]") {
            ty = p;
            array_depth += 1;
        }

        let mut element_ty = match &*ty.to_lowercase() {
            "none" => self.primitives.none,
            "bool" => self.primitives.bool,
            "int" => self.primitives.i32,
            "float" => self.primitives.f32,
            "string" => self.primitives.none,
            // Var is akin to an opaque value pointer.
            "var" => self.primitives.i64,
            name => self
                .classes
                .get(name)
                .map(|c| c.ty)
                .ok_or_else(|| Error::UndeclaredType(name.to_owned()))?,
        };

        for _ in 0..array_depth {
            element_ty = self
                .types
                .iter()
                .find_map(|v| match v.1 {
                    Type::Array { element } => (*element == element_ty).then_some(*element),
                    _ => None,
                })
                .unwrap_or_else(|| {
                    TypeId(self.types.insert(Type::Array {
                        element: element_ty,
                    }))
                });
        }

        Ok(element_ty)
    }

    pub fn declare_class(&mut self, class: &str) {
        let ty = TypeId(self.types.insert(Type::Class));
        self.classes.insert(
            class.to_lowercase(),
            Class {
                ty,
                functions: HashMap::new(),
                fields: HashMap::new(),
                parent: None,
                parent_name: None,
                field_names: HashMap::new(),
            },
        );
    }

    pub fn declare_parent_class(&mut self, class: &str, parent: &str) {
        self.classes
            .get_mut(&class.to_lowercase())
            .unwrap()
            .parent_name = Some(parent.to_lowercase());

        let parent = self.resolve_ty(parent).unwrap();

        self.classes.get_mut(&class.to_lowercase()).unwrap().parent = Some(parent);
    }

    pub fn declare_class_function(&mut self, class: &str, f: &openfo4_pex::Function) {
        let mut params = Vec::new();
        for param in &f.params {
            let ty = self.resolve_ty(&param.ty).unwrap();
            params.push(ty);
        }

        let returns = self.resolve_ty(&f.return_ty).unwrap();

        self.classes
            .get_mut(&class.to_lowercase())
            .unwrap()
            .functions
            .insert(
                f.name.to_lowercase(),
                Function {
                    params,
                    returns,
                    f: f.clone(),
                },
            );
    }

    pub fn declare_class_field(&mut self, class: &str, field: &openfo4_pex::Variable) {
        let ty = self.resolve_ty(&field.definition.ty).unwrap();

        self.classes
            .get_mut(&class.to_lowercase())
            .unwrap()
            .fields
            .insert(field.definition.name.to_lowercase(), ty);
        self.classes
            .get_mut(&class.to_lowercase())
            .unwrap()
            .field_names
            .insert(field.definition.name.to_lowercase(), field.clone());
    }

    pub fn define_native_function(&mut self, class: &str, function: &str, func: NativeFunction) {
        self.native_functions
            .push((class.to_lowercase(), function.to_lowercase(), func));
    }

    pub fn compile_fn<'a>(
        &mut self,
        class: &str,
        f: &openfo4_pex::Function,
    ) -> Result<CompiledFunction, Error> {
        let mut ctx = FunctionContext {
            class: class.to_lowercase(),
            locals: HashMap::new(),
        };

        for local in &f.locals {
            let name = local.name.to_lowercase();
            let ty = self.resolve_ty(&local.ty).unwrap();
            ctx.locals.insert(name, ty);
        }

        dbg!(&ctx);

        for (index, op) in f.instructions.iter().enumerate() {
            match op {
                Instruction::Nop => (),
                Instruction::IAdd(op)
                | Instruction::ISub(op)
                | Instruction::IMul(op)
                | Instruction::IDiv(op)
                | Instruction::IMod(op) => {
                    let lhs_ty = ctx.get_type(self, &op.lhs)?;
                    let rhs_ty = ctx.get_type(self, &op.rhs)?;
                    let dst_ty = ctx.get_type(self, &op.dst)?;

                    if lhs_ty.is_int() || rhs_ty.is_int() || dst_ty.is_int() {
                        panic!("all types must be int");
                    }
                }
                Instruction::FAdd(op)
                | Instruction::FSub(op)
                | Instruction::FMul(op)
                | Instruction::FDiv(op) => {
                    let lhs_ty = ctx.get_type(self, &op.lhs)?;
                    let rhs_ty = ctx.get_type(self, &op.rhs)?;
                    let dst_ty = ctx.get_type(self, &op.dst)?;

                    if lhs_ty.is_float() || rhs_ty.is_float() || dst_ty.is_float() {
                        panic!("all types must be float");
                    }
                }
                Instruction::CmpEq(op)
                | Instruction::CmpLt(op)
                | Instruction::CmpLe(op)
                | Instruction::CmpGt(op)
                | Instruction::CmpGe(op) => {
                    let lhs_ty = ctx.get_type(self, &op.lhs)?;
                    let rhs_ty = ctx.get_type(self, &op.rhs)?;
                    let dst_ty = ctx.get_type(self, &op.dst)?;

                    assert!(lhs_ty == rhs_ty);
                    assert!(dst_ty == Type::Bool);
                }
                Instruction::Store(op) => {
                    let src_ty = ctx.get_type(self, &op.src)?;
                    let dst_ty = ctx.get_type(self, &op.dst)?;

                    assert!(src_ty == dst_ty);
                }
                Instruction::Branch(offset) => {
                    let offset = match offset {
                        Value::Int(offset) => offset,
                        _ => todo!(),
                    };
                }
                Instruction::BranchIfFalse(op) | Instruction::BranchIfTrue(op) => {
                    let ty = ctx.get_type(self, &op.condition)?;

                    assert!(ty == Type::Bool);

                    let offset = match op.offset {
                        Value::Int(offset) => offset,
                        _ => todo!(),
                    };

                    dbg!(f.instructions.len());
                    dbg!(&index);

                    dbg!(&op);
                }
                Instruction::CallMethod(op) => {
                    let ty = ctx.get_type_id(self, &op.object)?;

                    let func_name = match &op.function_name {
                        Value::Ident(s) => s,
                        _ => todo!(),
                    };

                    dbg!(&ty);
                    let Some(class) = self.classes.iter().find(|v| v.1.ty == ty) else {
                        todo!();
                    };

                    let func = ctx.get_function(self, ty, &func_name)?;

                    // Validate that passed arguments match the function signature.
                    assert_eq!(func.params.len(), op.args.len());
                    for (param_ty, value) in func.params.iter().zip(&op.args) {
                        let ty = ctx.get_type_id(self, value)?;
                        assert_eq!(*param_ty, ty);
                    }

                    let ty = ctx.get_type_id(self, &op.return_assignment)?;
                    assert_eq!(func.returns, ty);
                }
                Instruction::CallStatic(op) => {
                    dbg!(&op);
                }
                op => panic!("{:?} {:?}", index, op),
            }
        }

        let mut codegen = CodegenPass::new();

        for class in self.classes.keys() {
            codegen.declare_class(class);
        }

        for (class_name, class) in &self.classes {
            if let Some(parent) = &class.parent_name {
                codegen.declare_parent_class(&class_name, parent);
            }

            for field in &class.field_names {
                codegen.declare_class_field(&class_name, field.1);
            }
        }

        for (clas_name, class) in &self.classes {
            for func in &class.functions {
                codegen.declare_class_function(&clas_name, &func.1.f);
            }
        }

        for (class, func, ptr) in &self.native_functions {
            codegen.define_native_function(&class, &func, ptr.clone());
        }

        let res = codegen.compile_fn(class, f);

        Ok(res)
    }
}

#[derive(Clone, Debug)]
struct FunctionContext {
    class: String,
    locals: HashMap<String, TypeId>,
}

impl FunctionContext {
    fn get_type_id(&self, ctx: &Context, value: &Value) -> Result<TypeId, Error> {
        match value {
            Value::Null => Ok(ctx.primitives.none),
            Value::Bool(_) => Ok(ctx.primitives.bool),
            Value::Int(_) => Ok(ctx.primitives.i32),
            Value::Float(_) => Ok(ctx.primitives.f32),
            Value::String(_) => todo!(),
            Value::Ident(ident) => match self.locals.get(&ident.to_lowercase()).or_else(|| {
                ctx.classes
                    .get(&self.class)
                    .unwrap()
                    .fields
                    .get(&ident.to_lowercase())
            }) {
                Some(ty) => Ok(*ty),
                None => todo!(),
            },
        }
    }

    fn get_type(&self, ctx: &Context, value: &Value) -> Result<Type, Error> {
        match value {
            Value::Null => Ok(Type::None),
            Value::Bool(_) => Ok(Type::Bool),
            Value::Int(_) => Ok(Type::I32),
            Value::Float(_) => Ok(Type::F32),
            Value::String(_) => todo!(),
            Value::Ident(ident) => match self.locals.get(&ident.to_lowercase()).or_else(|| {
                ctx.classes
                    .get(&self.class)
                    .unwrap()
                    .fields
                    .get(&ident.to_lowercase())
            }) {
                Some(ty) => Ok(ctx.types.get(ty.0).unwrap().clone()),
                None => todo!(),
            },
        }
    }

    fn get_function<'a>(
        &self,
        ctx: &'a Context,
        mut class: TypeId,
        func_name: &str,
    ) -> Result<&'a Function, Error> {
        let Some((root_class_name, _)) = ctx.classes.iter().find(|v| v.1.ty == class) else {
            todo!();
        };

        loop {
            let Some((_, c)) = ctx.classes.iter().find(|v| v.1.ty == class) else {
                todo!();
            };

            if let Some(func) = c.functions.get(&func_name.to_lowercase()) {
                return Ok(func);
            }

            if let Some(parent) = c.parent {
                class = parent;
            } else {
                break;
            }
        }

        Err(Error::UndeclaredMethod {
            class: root_class_name.clone(),
            function: func_name.to_owned(),
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
struct TypeId(Key);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    None,
    Bool,
    I32,
    I64,
    F32,
    Struct { members: HashMap<String, String> },
    Class,
    Array { element: TypeId },
}

impl Type {
    pub const fn is_int(&self) -> bool {
        matches!(self, Self::I32 | Self::I64)
    }

    pub const fn is_float(&self) -> bool {
        matches!(self, Self::I32 | Self::I64)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Primitives {
    none: TypeId,
    bool: TypeId,
    i32: TypeId,
    i64: TypeId,
    f32: TypeId,
}

#[derive(Clone, Debug)]
pub struct Function {
    pub params: Vec<TypeId>,
    pub returns: TypeId,
    pub f: openfo4_pex::Function,
}

#[derive(Clone, Debug)]
pub struct Class {
    pub ty: TypeId,
    pub parent: Option<TypeId>,
    pub parent_name: Option<String>,
    pub functions: HashMap<String, Function>,
    pub fields: HashMap<String, TypeId>,
    pub field_names: HashMap<String, Variable>,
}

pub trait Pass {
    type Result;

    fn declare_pass(&mut self, class: &str) -> Self::Result;
    fn declare_parent_class(&mut self, class: &str, parent: &str) -> Self::Result;
    fn declare_class_function(&mut self, class: &str, f: &openfo4_pex::Function) -> Self::Result;
    fn declare_class_field(&mut self, class: &str, field: &openfo4_pex::Variable) -> Self::Result;

    fn compile_fn(&mut self, class: &str, f: &openfo4_pex::Function) -> Self::Result;
}

#[derive(Clone, Debug)]
pub struct NativeFunction {
    pub ptr: *const u8,
    pub index: usize,
    pub signature: Signature,
}
