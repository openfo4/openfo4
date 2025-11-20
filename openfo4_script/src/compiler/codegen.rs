use std::collections::{HashMap, HashSet};
use std::slice::RChunks;
use std::thread::panicking;

use game_common::collections::arena::{Arena, Key};
use openfo4_jit::{
    Compiler, FunctionBuilder, FunctionId, IntCompareOp, Signature, StackSlot, Type as JitType,
    Value as JitValue,
};
use openfo4_pex::{Function, Instruction, Value, Variable, VariableDefinition};

pub struct CodegenPass {
    compiler: Compiler,
    types: Arena<JitType>,
    primitives: Primitives,
    classes: HashMap<String, ClassDef>,
}

impl CodegenPass {
    pub fn new() -> Self {
        let mut types = Arena::new();

        let mut compiler = Compiler::new();

        let none = types.insert(JitType::I32);
        let bool = types.insert(JitType::I32);
        let int = types.insert(JitType::I32);
        let float = types.insert(JitType::F32);
        let ptr = types.insert(JitType::Pointer);

        Self {
            compiler,
            types,
            primitives: Primitives {
                none,
                bool,
                int,
                float,
                ptr,
            },
            classes: HashMap::new(),
        }
    }

    pub fn declare_class(&mut self, class: &str) {
        self.classes
            .insert(class.to_lowercase(), ClassDef::default());
    }

    pub fn declare_class_function(&mut self, class: &str, f: &Function) {
        let mut params = Vec::new();
        let mut returns = Vec::new();

        for param in &f.params {
            let ty = self.resolve_ty(&param.ty);
            let ty = *self.types.get(ty).unwrap();

            params.push(ty);
        }

        if !f.return_ty.is_empty() {
            let ty = self.resolve_ty(&f.return_ty);
            let ty = *self.types.get(ty).unwrap();

            returns.push(ty);
        }

        let def = self.classes.get_mut(class).unwrap();
        def.functions.insert(
            f.name.to_lowercase(),
            FunctionDef {
                signature: Signature { params, returns },
                func_id: None,
                function: f.clone(),
                native: None,
            },
        );
    }

    pub fn declare_parent_class(&mut self, class: &str, parent: &str) {
        let def = self.classes.get_mut(class).unwrap();
        def.parent = Some(parent.to_lowercase());
    }

    pub fn declare_class_field(&mut self, class: &str, field: &Variable) {
        let def = self.classes.get_mut(class).unwrap();

        def.fields.insert(
            field.definition.name.to_lowercase(),
            field.definition.ty.to_lowercase(),
        );
    }

    pub fn define_native_function(&mut self, class: &str, function: &str, ptr: *const u8) {
        let def = self.classes.get_mut(class).unwrap();
        let func_def = def.functions.get_mut(function).unwrap();
        func_def.native = Some(ptr);
    }

    fn resolve_ty(&self, mut ty: &str) -> Key {
        match &*ty.to_lowercase() {
            "none" => self.primitives.none,
            "bool" => self.primitives.bool,
            "int" => self.primitives.int,
            "float" => self.primitives.float,
            "string" => self.primitives.ptr,
            "var" => self.primitives.ptr,
            _ => self.primitives.ptr,
        }
    }

    pub fn compile_fn(&mut self, class: &str, f: &Function) {
        let mut compiler = Compiler::new();

        // The compile the function we must also compile all functions that
        // are called.
        let mut queue = vec![(class.to_lowercase(), f.name.to_lowercase())];
        let mut dependencies = HashMap::<(String, String), HashSet<(String, String)>>::new();
        while let Some((class, function)) = queue.pop() {
            let def = self
                .classes
                .get(&class)
                .unwrap()
                .functions
                .get(&function)
                .unwrap();

            // Function is already compiled.
            if def.func_id.is_some() {
                continue;
            }

            let callees = find_function_callees(&self.classes, &class, &def.function);

            let mut depends_on = HashSet::new();
            for (callee_class, callee_func) in callees {
                // The callee function may also be only available on a parent class.
                let callee_class =
                    get_parent_class_with_function(&self.classes, &callee_class, &callee_func)
                        .unwrap();

                let def = self
                    .classes
                    .get(&callee_class)
                    .unwrap()
                    .functions
                    .get(&callee_func)
                    .unwrap();

                if !def.func_id.is_some() {
                    depends_on.insert((callee_class.clone(), callee_func.clone()));
                    queue.push((callee_class, callee_func));
                }
            }

            if depends_on.is_empty() {
                // Compile the function.
                let func_id = self.compile_function_internal(&class, &function, &mut compiler);

                self.classes
                    .get_mut(&class)
                    .unwrap()
                    .functions
                    .get_mut(&function)
                    .unwrap()
                    .func_id = Some(func_id);

                // Mark all dependend callers as ready.
                dependencies.retain(|(class_name, func_name), depends| {
                    depends.remove(&(class.clone(), function.clone()));
                    if depends.is_empty() {
                        queue.push((class_name.clone(), func_name.clone()));
                        false
                    } else {
                        true
                    }
                });

                continue;
            }

            dependencies.insert((class, function), depends_on);
        }

        let def = self
            .classes
            .get(&class.to_lowercase())
            .unwrap()
            .functions
            .get(&f.name.to_lowercase())
            .unwrap();

        dbg!(&def.func_id);

        todo!()
    }

    fn compile_function_internal(
        &mut self,
        class: &str,
        function: &str,
        compiler: &mut Compiler,
    ) -> FunctionId {
        let current_class = self.classes.get(class).unwrap();
        let function = self
            .classes
            .get(class)
            .unwrap()
            .functions
            .get(function)
            .unwrap();

        let f = &function.function;

        let mut params = Vec::new();
        let mut returns = Vec::new();

        // For non-static function (methods) the first argument is the object pointer.
        if !f.is_static {
            params.push(JitType::Pointer);
        }

        for param in &f.params {
            let ty = self.resolve_ty(&param.ty);
            let ty = *self.types.get(ty).unwrap();

            params.push(ty);
        }

        if !f.return_ty.is_empty() && f.return_ty.to_lowercase() != "none" {
            let ty = self.resolve_ty(&f.return_ty);
            let ty = *self.types.get(ty).unwrap();

            returns.push(ty);
        }

        let func_id = compiler.declare_function(Signature { params, returns });

        if f.is_native {
            let Some(native_func_ptr) = function.native else {
                panic!(
                    "native function {}.{} is called but not defined",
                    class, &function.function.name
                );
            };

            compiler.define_native_function(func_id, native_func_ptr);

            return func_id;
        }

        let mut builder = compiler.define_function(func_id);

        let entry_block = builder.create_entry_block();
        builder.switch_to_block(entry_block);

        builder.block_params(entry_block);

        let mut locals = HashMap::with_capacity(f.locals.len());

        // Create stack slots for local variables.
        for local in &f.locals {
            let ty = self.resolve_ty(&local.ty);
            let ty = *self.types.get(ty).unwrap();

            let slot = builder.create_typed_stack_slot(ty);
            locals.insert(
                local.name.to_lowercase(),
                Local {
                    ty,
                    slot,
                    ty_name: local.ty.to_lowercase(),
                },
            );

            let value = builder.iconst(JitType::I32, 0);
            builder.stack_store(slot, value, 0);
        }

        // Move function parameters into stack slots.
        for (index, param) in f.params.iter().enumerate() {
            let ty = self.resolve_ty(&param.ty);
            let ty = *self.types.get(ty).unwrap();

            let slot = builder.create_typed_stack_slot(ty);

            locals.insert(
                param.name.to_lowercase(),
                Local {
                    ty,
                    slot,
                    ty_name: param.ty.to_lowercase(),
                },
            );

            let value = builder.block_params(entry_block)[index];
            builder.stack_store(slot, value, 0);
        }

        let mut fields = HashMap::new();
        for (name, ty) in &current_class.fields {
            let ty = self.resolve_ty(&ty);
            let ty = *self.types.get(ty).unwrap();

            fields.insert(name.to_lowercase(), ty);
        }

        let mut translator = FunctionTranslator {
            builder,
            locals,
            fields,
        };

        // Setup blocks
        let mut blocks = HashMap::new();
        for (index, instruction) in f.instructions.iter().enumerate() {
            match instruction {
                Instruction::Branch(offset) => {
                    let offset = match offset {
                        Value::Int(offset) => *offset,
                        _ => unreachable!(),
                    };

                    let addr = usize::min((index as i32 + offset) as usize, f.instructions.len());

                    if !blocks.contains_key(&addr) {
                        let block = translator.builder.create_block();
                        blocks.insert(addr, block);
                    }
                }
                Instruction::BranchIfTrue(op) | Instruction::BranchIfFalse(op) => {
                    let offset = match &op.offset {
                        Value::Int(offset) => *offset,
                        _ => unreachable!(),
                    };

                    let addr = usize::min((index as i32 + offset) as usize, f.instructions.len());

                    if !blocks.contains_key(&addr) {
                        let then_block = translator.builder.create_block();
                        blocks.insert(addr, then_block);
                    }

                    let next_addr = index + 1;
                    if !blocks.contains_key(&next_addr) {
                        let else_block = translator.builder.create_block();
                        blocks.insert(next_addr, else_block);
                    }
                }
                _ => {}
            }
        }

        for (index, instruction) in f.instructions.iter().enumerate() {
            if let Some(block) = blocks.get(&index) {
                if !translator.builder.is_current_block_full() {
                    translator.builder.jump(*block, &[]);
                }

                translator.builder.switch_to_block(*block);
            }

            match instruction {
                Instruction::Nop => {}
                Instruction::IAdd(op) => {
                    let lhs = translator.load(&op.lhs);
                    let rhs = translator.load(&op.rhs);
                    let res = translator.builder.iadd(lhs, rhs);
                    translator.store(&op.dst, res);
                }
                Instruction::ISub(op) => {
                    let lhs = translator.load(&op.lhs);
                    let rhs = translator.load(&op.rhs);
                    let res = translator.builder.isub(lhs, rhs);
                    translator.store(&op.dst, res);
                }
                Instruction::IMul(op) => {
                    let lhs = translator.load(&op.lhs);
                    let rhs = translator.load(&op.rhs);
                    let res = translator.builder.imul(lhs, rhs);
                    translator.store(&op.dst, res);
                }
                Instruction::IDiv(op) => {
                    let lhs = translator.load(&op.lhs);
                    let rhs = translator.load(&op.rhs);
                    let res = translator.builder.sdiv(lhs, rhs);
                    translator.store(&op.dst, res);
                }
                Instruction::IMod(op) => {
                    todo!();
                }
                Instruction::INeg(op) => {
                    let src = translator.load(&op.src);
                    let res = translator.builder.ineg(src);
                    translator.store(&op.dst, res);
                }
                Instruction::FAdd(op) => {
                    let lhs = translator.load(&op.lhs);
                    let rhs = translator.load(&op.rhs);
                    let res = translator.builder.fadd(lhs, rhs);
                    translator.store(&op.dst, res);
                }
                Instruction::FSub(op) => {
                    let lhs = translator.load(&op.lhs);
                    let rhs = translator.load(&op.rhs);
                    let res = translator.builder.fsub(lhs, rhs);
                    translator.store(&op.dst, res);
                }
                Instruction::FMul(op) => {
                    let lhs = translator.load(&op.lhs);
                    let rhs = translator.load(&op.rhs);
                    let res = translator.builder.fmul(lhs, rhs);
                    translator.store(&op.dst, res);
                }
                Instruction::FDiv(op) => {
                    let lhs = translator.load(&op.lhs);
                    let rhs = translator.load(&op.rhs);
                    let res = translator.builder.fdiv(lhs, rhs);
                    translator.store(&op.dst, res);
                }
                Instruction::FNeg(op) => {
                    let src = translator.load(&op.src);
                    let res = translator.builder.fneg(src);
                    translator.store(&op.dst, res);
                }
                Instruction::Store(op) => {
                    let value = translator.load(&op.src);
                    translator.store(&op.dst, value);
                }
                Instruction::CmpEq(op)
                | Instruction::CmpLt(op)
                | Instruction::CmpLe(op)
                | Instruction::CmpGt(op)
                | Instruction::CmpGe(op) => {
                    let cmp_op = match instruction {
                        Instruction::CmpEq(_) => IntCompareOp::Equal,
                        Instruction::CmpLt(_) => IntCompareOp::SignedLessThan,
                        Instruction::CmpLe(_) => IntCompareOp::SignedLessThanOrEqual,
                        Instruction::CmpGt(_) => IntCompareOp::SignedGreaterThan,
                        Instruction::CmpGe(_) => IntCompareOp::SignedGreaterThanOrEqual,
                        _ => unreachable!(),
                    };

                    let lhs = translator.load(&op.lhs);
                    let rhs = translator.load(&op.rhs);
                    let res = translator.builder.icmp(lhs, rhs, cmp_op);
                    translator.store(&op.dst, res);
                }
                Instruction::Branch(offset) => {
                    let offset = match offset {
                        Value::Int(offset) => *offset,
                        _ => unreachable!(),
                    };

                    let addr = usize::min((index as i32 + offset) as usize, f.instructions.len());

                    let block = *blocks.get(&addr).unwrap();
                    translator.builder.jump(block, &[]);
                }
                Instruction::BranchIfTrue(op) => {
                    let offset = match &op.offset {
                        Value::Int(offset) => *offset,
                        _ => unreachable!(),
                    };

                    let addr = usize::min((index as i32 + offset) as usize, f.instructions.len());
                    let then_block = *blocks.get(&addr).unwrap();
                    let else_block = *blocks.get(&(index + 1)).unwrap();

                    let condition = translator.load(&op.condition);

                    translator
                        .builder
                        .brif(condition, then_block, &[], else_block, &[]);
                }
                Instruction::BranchIfFalse(op) => {
                    let offset = match &op.offset {
                        Value::Int(offset) => *offset,
                        _ => unreachable!(),
                    };

                    let addr = usize::min((index as i32 + offset) as usize, f.instructions.len());
                    let else_block = *blocks.get(&addr).unwrap();
                    let then_block = *blocks.get(&(index + 1)).unwrap();

                    let condition = translator.load(&op.condition);

                    translator
                        .builder
                        .brif(condition, then_block, &[], else_block, &[]);
                }
                Instruction::CallMethod(op) => {
                    let Value::Ident(object) = &op.object else {
                        unreachable!();
                    };

                    let function_name = match &op.function_name {
                        Value::Ident(v) => v,
                        _ => unreachable!(),
                    };

                    let find_var = |var_name: &str| -> String {
                        if let Some(local) = translator.locals.get(&var_name.to_lowercase()) {
                            return local.ty_name.clone();
                        }

                        if let Some(ty) = current_class.fields.get(&var_name.to_lowercase()) {
                            return ty.to_lowercase();
                        }

                        todo!()
                    };

                    let mut class_name = find_var(&object);
                    class_name = get_parent_class_with_function(
                        &self.classes,
                        &class_name,
                        &function_name.to_lowercase(),
                    )
                    .unwrap();

                    let func = self
                        .classes
                        .get(&class_name.to_lowercase())
                        .unwrap()
                        .functions
                        .get(&function_name.to_lowercase())
                        .unwrap()
                        .func_id
                        .unwrap();

                    let mut args = op
                        .args
                        .iter()
                        .map(|v| translator.load(v))
                        .collect::<Vec<_>>();

                    // In our ABI the first argument is the object pointer.
                    let object = translator.load(&op.object);
                    args.insert(0, object);

                    let returns = translator.builder.call(func, &args);
                    if let Some(res) = returns.get(0).copied() {
                        translator.store(&op.return_assignment, res);
                    }
                }
                Instruction::CallParent(op) => {}
                Instruction::CallStatic(op) => {
                    let (class_name, function_name) = match (&op.class, &op.function) {
                        (Value::Ident(class), Value::Ident(func)) => (class, func),
                        _ => unreachable!(),
                    };

                    let func = self
                        .classes
                        .get(&class_name.to_lowercase())
                        .unwrap()
                        .functions
                        .get(&function_name.to_lowercase())
                        .unwrap()
                        .func_id
                        .unwrap();

                    let args = op
                        .args
                        .iter()
                        .map(|v| translator.load(v))
                        .collect::<Vec<_>>();

                    let returns = translator.builder.call(func, &args);
                    if let Some(res) = returns.get(0).copied() {
                        translator.store(&op.return_assignment, res);
                    }
                }
                op => todo!("{:?}", op),
            }
        }

        if let Some(block) = blocks.get(&f.instructions.len()) {
            translator.builder.switch_to_block(*block);
            translator.builder.return_(&[]);
        }

        println!("{}", f.text());

        // translator.builder.return_(&[]);
        translator.builder.finalize();

        let compiled = compiler.compile_function(func_id);
        println!("{}", compiled.text().unwrap());

        func_id
    }
}

struct FunctionTranslator<'a> {
    builder: FunctionBuilder<'a>,
    locals: HashMap<String, Local>,
    fields: HashMap<String, JitType>,
}

impl<'a> FunctionTranslator<'a> {
    fn load(&mut self, value: &Value) -> JitValue {
        match value {
            Value::Null => self.builder.iconst(JitType::I32, 0),
            Value::Bool(value) => self.builder.iconst(JitType::I32, u8::from(*value)),
            Value::Int(value) => self.builder.iconst(JitType::I32, *value),
            Value::Float(value) => self.builder.fconst(JitType::F32, *value),
            Value::String(value) => todo!(),
            Value::Ident(name) => {
                if let Some(local) = self.locals.get(&name.to_lowercase()) {
                    return self.builder.stack_load(local.slot, local.ty, 0);
                }

                dbg!(&self.fields);
                if let Some(ty) = self.fields.get(&name.to_lowercase()) {
                    // TODO
                    return self.builder.iconst(*ty, 0);
                }

                todo!()
            }
        }
    }

    fn store(&mut self, slot: &Value, value: JitValue) {
        let Value::Ident(name) = slot else {
            return;
        };

        if let Some(local) = self.locals.get(&name.to_lowercase()) {
            self.builder.stack_store(local.slot, value, 0);
            return;
        }

        // TODO
    }
}

#[derive(Clone, Debug)]
struct Local {
    ty: JitType,
    ty_name: String,
    slot: StackSlot,
}

struct Type {
    size: usize,
    align: usize,
}

impl Type {
    const NONE: Self = Self { size: 0, align: 0 };
    const BOOL: Self = Self { size: 1, align: 1 };
    const INT: Self = Self { size: 4, align: 4 };
    const FLOAT: Self = Self { size: 4, align: 4 };
    const STRING: Self = Self { size: 8, align: 8 };
}

#[derive(Clone, Debug)]
struct Primitives {
    none: Key,
    bool: Key,
    int: Key,
    float: Key,
    ptr: Key,
}

#[derive(Clone, Debug, Default)]
struct ClassDef {
    functions: HashMap<String, FunctionDef>,
    parent: Option<String>,
    fields: HashMap<String, String>,
}

#[derive(Clone, Debug)]
struct FunctionDef {
    signature: Signature,
    func_id: Option<FunctionId>,
    function: Function,
    native: Option<*const u8>,
}

fn find_function_callees(
    classes: &HashMap<String, ClassDef>,
    class: &str,
    function: &Function,
) -> Vec<(String, String)> {
    let mut callees = Vec::new();

    let current_class = classes.get(class).unwrap();

    let find_var = |var_name: &str| -> String {
        for local in &function.locals {
            if local.name.to_lowercase() == var_name.to_lowercase() {
                return local.ty.to_lowercase();
            }
        }

        if let Some(ty) = current_class.fields.get(&var_name.to_lowercase()) {
            return ty.to_lowercase();
        }

        todo!()
    };

    for instruction in &function.instructions {
        match instruction {
            Instruction::CallMethod(op) => {
                let Value::Ident(object) = &op.object else {
                    continue;
                };

                let class_name = find_var(&object);
                let function_name = match &op.function_name {
                    Value::Ident(v) => v,
                    _ => unreachable!(),
                };

                callees.push((class_name, function_name.to_lowercase()));
            }
            Instruction::CallParent(op) => {}
            Instruction::CallStatic(op) => {
                let (class_name, function_name) = match (&op.class, &op.function) {
                    (Value::Ident(a), Value::Ident(b)) => (a, b),
                    _ => unreachable!(),
                };

                callees.push((class_name.to_lowercase(), function_name.to_lowercase()));
            }
            _ => {}
        }
    }

    callees.dedup();
    callees
}

fn get_parent_class_with_function<'a>(
    classes: &'a HashMap<String, ClassDef>,
    class_name: &str,
    func_name: &str,
) -> Option<String> {
    let mut class_name = class_name.to_lowercase();

    loop {
        let class = classes.get(&class_name)?;

        if class.functions.get(&func_name.to_lowercase()).is_some() {
            return Some(class_name);
        }

        let Some(parent) = &class.parent else {
            return None;
        };

        class_name = parent.to_lowercase();
    }
}
