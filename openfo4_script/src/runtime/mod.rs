pub mod object;
mod stack;

pub(crate) mod host_functions;

use std::any::Any;
use std::marker::PhantomData;
use std::mem::{MaybeUninit, transmute};
use std::sync::{Arc, Weak};

use game_common::cell::UnsafeRefCell;
use openfo4_jit::{Signature, Type};
use thiserror::Error;

pub use object::{ArrayObject, Object, ObjectAddr, ObjectPool, ObjectRef};
pub use stack::Stack;

use crate::compiler::resources::TypeId;
use crate::compiler::{CompilationResult, PscSignature};
use crate::engine::{Engine, EngineShared};

pub(crate) const RESULT_OK: i32 = 0;
pub(crate) const RESULT_ERROR: i32 = 1;
pub(crate) const RESULT_SUSPEND: i32 = 2;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum Exception {
    Error = RESULT_ERROR,
    Suspend = RESULT_SUSPEND,
}

type NativeFnPtr = unsafe extern "C" fn(*const RawContext, *const RawVal, *mut RawVal) -> i32;

/// Context passed to guest functions.
pub struct Context<'a, T> {
    engine: Arc<EngineShared>,
    stack: &'a mut Stack,
    data: T,
}

impl<'a, T> Context<'a, T> {
    pub fn new(engine: &Engine, stack: &'a mut Stack, data: T) -> Self {
        assert!(Arc::ptr_eq(&engine.shared, &stack.engine));

        Self {
            engine: engine.shared.clone(),
            stack,
            data,
        }
    }

    pub fn stack(&self) -> &Stack {
        &self.stack
    }

    pub fn data_mut(&mut self) -> &mut T {
        &mut self.data
    }

    pub fn into_data(self) -> T {
        self.data
    }
}

// The stack pointer must be at offset 0.
#[derive(Debug)]
#[repr(C)]
pub(crate) struct RawContext {
    pub stack_ptr: RawStack,
    pub state: *const HostState,
    pub data: *mut dyn Any,
    pub stack: *mut Stack,
    panic: PanicMsg,
}

#[derive(Clone, Debug)]
#[repr(C)]
pub(crate) struct RawStack {
    pub ptr: *mut u8,
    pub len: usize,
}

#[derive(Debug, Default)]
struct PanicMsg {
    #[cfg(panic = "unwind")]
    msg: Option<Box<dyn Any + Send>>,
}

impl PanicMsg {
    /// Sets the panic message.
    #[cfg(panic = "unwind")]
    #[inline]
    fn set(&mut self, msg: Box<dyn Any + Send>) {
        self.msg = Some(msg);
    }

    /// Resumes the caught panic on the current thread.
    #[inline]
    fn resume(&mut self) {
        #[cfg(panic = "unwind")]
        if let Some(msg) = self.msg.take() {
            std::panic::resume_unwind(msg);
        }
    }
}

/// A union over all values that can be used in Papyrus functions.
#[derive(Copy, Clone)]
#[repr(C)]
pub union RawVal {
    /// A boolean type.
    ///
    /// This is the equivalent of the builtin `bool` type.
    pub bool: bool,
    /// An integer type.
    ///
    /// This is the equivalent of the builtin `int` type.
    pub i32: i32,
    /// A floating-point type.
    ///
    /// This is the equivalent of the builtin `float` type.
    pub f32: f32,
    /// An object pointer.
    ///
    /// This is the equivalent of class objects, structs, arrays and the builtin types `string`
    /// and `var`.
    pub object: ObjectAddr,
}

/// A value can be used in Papyrus functions.
#[derive(Copy, Clone, Debug)]
pub enum Val {
    /// A boolean type.
    ///
    /// This is the equivalent of the builtin `bool` type.
    Bool(bool),
    /// An integer type.
    ///
    /// This is the equivalent of the builtin `int` type.
    I32(i32),
    /// A floating-point type.
    ///
    /// This is the equivalent of the builtin `float` type.
    F32(f32),
    /// An object pointer.
    ///
    /// This is the equivalent of class objects, structs, arrays and the builtin types `string`
    /// and `var`.
    Object(ObjectAddr),
}

impl Val {
    const fn into_raw(self) -> RawVal {
        match self {
            Self::Bool(bool) => RawVal { bool },
            Self::I32(i32) => RawVal { i32 },
            Self::F32(f32) => RawVal { f32 },
            Self::Object(object) => RawVal { object },
        }
    }

    fn get_type_id(self, pool: &ObjectPool) -> TypeRef {
        let res = pool.resources();
        match self {
            Self::Bool(_) => TypeRef::Concrete(res.primitives.bool),
            Self::I32(_) => TypeRef::Concrete(res.primitives.int),
            Self::F32(_) => TypeRef::Concrete(res.primitives.float),
            Self::Object(addr) => {
                if addr == ObjectAddr::NULL {
                    return TypeRef::Null;
                }

                match pool.get(addr) {
                    Some(object) => TypeRef::Concrete(object.type_id()),
                    None => TypeRef::Invalid,
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Func {
    ptr: NativeFnPtr,
    signature: Signature,
    psc_sig: PscSignature,
    engine: Weak<EngineShared>,
}

impl Func {
    /// # Safety
    ///
    /// - The function pointer must point to a [`NativeFnPtr`] function.
    pub(crate) unsafe fn new(res: &CompilationResult, engine: Weak<EngineShared>) -> Self {
        // Should always match the ABI of `host_call_trampoline`.
        debug_assert_eq!(
            res.function.signature(),
            &Signature {
                params: vec![Type::Pointer, Type::Pointer, Type::Pointer],
                returns: vec![Type::I32]
            }
        );

        Self {
            // SAFETY: The caller guarantees that the function is a valid
            // function pointer of type `NativeFnPtr`.
            ptr: unsafe { transmute::<*const u8, NativeFnPtr>(res.function.as_ptr()) },
            signature: res.signature.clone(),
            psc_sig: res.psc_signature.clone(),
            engine,
        }
    }

    pub fn call<T>(&self, ctx: &mut Context<'_, T>, params: &[Val]) -> Result<Vec<Val>, Exception>
    where
        T: 'static,
    {
        assert_eq!(
            self.psc_sig.params.len(),
            params.len(),
            "function takes {} parameters, {} were given",
            self.psc_sig.params.len(),
            params.len()
        );
        validate_parameter_types(&ctx.engine.state.pool, params, &self.psc_sig);

        let mut r_params = vec![MaybeUninit::<RawVal>::uninit(); params.len()];
        let mut r_returns = vec![MaybeUninit::<RawVal>::uninit(); self.signature.returns.len()];

        // We don't need logic to drop written `MaybeUninit<RawVal>` values.
        const {
            assert!(!core::mem::needs_drop::<RawVal>());
        }

        for (index, param) in params.iter().enumerate() {
            r_params[index].write(param.into_raw());
        }

        // SAFETY:
        // - When we have constructed this `TypedFunc` we have asserted that the signature
        //   matches.
        // - We have filled all params according to the function signature.
        // - We have allocated enough capacity for all return values according to the function
        //   signature.
        unsafe {
            self.call_unchecked(
                ctx,
                r_params.as_ptr().cast::<RawVal>(),
                r_returns.as_mut_ptr().cast::<RawVal>(),
            )?;
        }

        let mut returns = Vec::with_capacity(self.signature.returns.len());
        for (index, ty) in self.signature.returns.iter().enumerate() {
            let raw = unsafe { r_returns[index].assume_init_ref() };

            unsafe {
                returns.push(match ty {
                    Type::I8 => Val::Bool(<bool as NativeType>::from_raw(raw)),
                    Type::I32 => Val::I32(<i32 as NativeType>::from_raw(raw)),
                    Type::F32 => Val::F32(<f32 as NativeType>::from_raw(raw)),
                    Type::Pointer => Val::Object(<ObjectAddr as NativeType>::from_raw(raw)),
                    _ => unreachable!(),
                });
            }
        }

        Ok(returns)
    }

    /// Calls the guest function.
    ///
    /// # Panics
    ///
    /// - Panics if the provided [`Context`] was created from a different [`Engine`] than this
    ///   `Func`.
    /// - Panics if the [`Stack`] of the provided [`Context`] is not empty and contains contents
    ///   for a different `Func`.
    ///
    /// # Safety
    ///
    /// - The values in `params` must match the function signature.
    /// - The `returns` allocation must be large enough for the return parameters of the function
    ///   signature.
    pub unsafe fn call_unchecked<T>(
        &self,
        ctx: &mut Context<'_, T>,
        params: *const RawVal,
        returns: *mut RawVal,
    ) -> Result<(), Exception>
    where
        T: 'static,
    {
        // The guest function will read from the stack and on suspension assume that
        // current contents match the contents of the previous writes.
        // We must ensure that we only resume using a stack that is compatible with
        // function we are calling, otherwise the guest function will begin reading
        // arbitrary memory and reinterpret memory incorrectly.
        assert!(
            core::ptr::addr_eq(Arc::as_ptr(&ctx.engine), Weak::as_ptr(&self.engine)),
            "attempted to call func with context from another Engine"
        );
        if let Some(stack_fn_ptr) = ctx.stack.func {
            let fn_ptr = self.ptr as *const u8;
            assert!(
                core::ptr::addr_eq(fn_ptr, stack_fn_ptr),
                "attempted to reuse stack for incompatible func"
            );
        }

        ctx.stack.func = Some(self.ptr as *const u8);

        // Allocate enough stack space for the callee prologue.
        if ctx.stack.size() == 0 {
            ctx.stack.alloc(size_of::<usize>());
        }

        // We must lock the call table before we call the guest function.
        let _lock = ctx.engine.call_table_lock.read();

        let mut raw_ctx = RawContext {
            stack_ptr: RawStack {
                ptr: ctx.stack.as_ptr(),
                len: 0,
            },
            state: &ctx.engine.state,
            data: &mut ctx.data as *mut dyn Any,
            stack: ctx.stack,
            panic: PanicMsg::default(),
        };

        let res = unsafe { (self.ptr)(&raw_ctx, params, returns) };

        // If the callee calls into a native host function that panics,
        // we will resume the panic here.
        // The following code will not be executed.
        raw_ctx.panic.resume();

        if res == RESULT_OK {
            ctx.stack.free(size_of::<usize>());
            debug_assert_eq!(ctx.stack.size(), 0);

            ctx.stack.func = None;

            Ok(())
        } else {
            // SAFETY: The result parameter is only created as RESULT_OK from inside
            // compiled functions. All other values are only produced by host functions
            // that return a Exception, so we are doing a type roundtrip.
            Err(unsafe { transmute::<i32, Exception>(res) })
        }
    }

    pub fn typed<Params, Returns>(
        &self,
    ) -> Result<TypedFunc<'_, Params, Returns>, FuncSignatureMismatch>
    where
        Params: NativeTuple,
        Returns: NativeTuple,
    {
        if &self.signature.params[1..] != Params::TYPES
            || &self.signature.returns[..self.signature.returns.len() - 1] != Returns::TYPES
        {
            let expected = Signature {
                params: Params::TYPES.to_vec(),
                returns: Returns::TYPES.to_vec(),
            };
            let found = self.signature.clone();

            return Err(FuncSignatureMismatch { expected, found });
        }

        Ok(TypedFunc {
            func: self,
            _marker: PhantomData,
        })
    }
}

#[derive(Clone, Debug, Error)]
#[error("expected {expected:?} found {found:?}")]
pub struct FuncSignatureMismatch {
    expected: Signature,
    found: Signature,
}

/// A statically typed function.
pub struct TypedFunc<'a, Params, Returns> {
    func: &'a Func,
    _marker: PhantomData<fn(Params) -> Returns>,
}

impl<'a, Params, Returns> TypedFunc<'a, Params, Returns>
where
    Params: NativeTuple,
    Returns: NativeTuple,
{
    pub fn call<T>(&self, ctx: &mut Context<'_, T>, params: Params) -> Result<Returns, Exception>
    where
        T: 'static,
    {
        validate_parameter_types(&ctx.engine.state.pool, &params, &self.func.psc_sig);

        // TODO: When const_generics is stabilized we can avoid the heap allocation
        // and use a regular array.
        let mut r_params = vec![MaybeUninit::<RawVal>::uninit(); Params::SIZE];
        let mut r_returns = vec![MaybeUninit::<RawVal>::uninit(); Returns::SIZE];

        // We don't need logic to drop written `MaybeUninit<RawVal>` values.
        const {
            assert!(!core::mem::needs_drop::<RawVal>());
        }

        unsafe {
            params.write_raw(r_params.as_mut_ptr().cast::<RawVal>());
        }

        // SAFETY:
        // - When we have constructed this `TypedFunc` we have asserted that the signature
        //   matches.
        // - We have filled all params according to the function signature.
        // - We have allocated enough capacity for all return values according to the function
        //   signature.
        unsafe {
            self.func.call_unchecked(
                ctx,
                r_params.as_ptr().cast::<RawVal>(),
                r_returns.as_mut_ptr().cast::<RawVal>(),
            )?;
        }

        let returns = unsafe { Returns::from_raw(r_returns.as_ptr().cast::<RawVal>()) };
        Ok(returns)
    }
}

/// Context passed to host functions called from the guest.
pub struct Caller<'a, T>
where
    T: 'static,
{
    ctx: &'a RawContext,
    _marker: PhantomData<T>,
}

impl<'a, T> Caller<'a, T> {
    pub fn data(&self) -> &T {
        unsafe { &*self.ctx.data }.downcast_ref().unwrap()
    }

    pub fn data_mut(&mut self) -> &mut T {
        let data_ref = unsafe { &mut *self.ctx.data };
        data_ref.downcast_mut().unwrap()
    }

    pub unsafe fn array<U>(&self, addr: ObjectAddr) -> &[U] {
        unsafe {
            let object = &*addr.to_raw().cast::<ArrayObject>();

            // Array with len 0 MAY have a zero pointer that we cannot use,
            // even for a zero-sized slice.
            if object.len == 0 {
                &[]
            } else {
                core::slice::from_raw_parts(object.ptr.cast::<U>(), object.len)
            }
        }
    }

    pub unsafe fn array_mut<U>(&mut self, addr: ObjectAddr) -> &mut [U] {
        unsafe {
            let object = &*addr.to_raw().cast::<ArrayObject>();

            // Array with len 0 MAY have a zero pointer that we cannot use,
            // even for a zero-sized slice.
            if object.len == 0 {
                &mut []
            } else {
                core::slice::from_raw_parts_mut(object.ptr.cast::<U>(), object.len)
            }
        }
    }

    pub unsafe fn string(&self, addr: ObjectAddr) -> &str {
        unsafe {
            let data = self.array::<u8>(addr);
            std::str::from_utf8_unchecked(data)
        }
    }

    pub unsafe fn copy_array_of_structs<U>(&self, addr: ObjectAddr) -> Vec<U> {
        let addrs = unsafe { self.array::<ObjectAddr>(addr) };

        let mut objects = Vec::with_capacity(addrs.len());
        for addr in addrs {
            let object = unsafe { addr.to_raw().cast::<U>().read() };
            objects.push(object);
        }

        objects
    }

    pub fn object_pool(&self) -> &ObjectPool {
        unsafe { &(*self.ctx.state).pool }
    }
}

pub(crate) struct HostState {
    pub call_table: UnsafeRefCell<CallTable>,
    pub pool: ObjectPool,
}

#[derive(Default)]
pub(crate) struct CallTable {
    functions: Vec<Box<dyn Fn(*const RawContext, *const RawVal, *mut RawVal) -> i32 + Send + Sync>>,
}

impl CallTable {
    pub fn insert<F>(&mut self, func: F) -> usize
    where
        F: Fn(*const RawContext, *const RawVal, *mut RawVal) -> i32 + Send + Sync + 'static,
    {
        let index = self.functions.len();
        self.functions.push(Box::new(func));
        index
    }
}

pub trait IntoFunc<T, Params, Returns> {
    #[doc(hidden)]
    #[expect(private_interfaces)]
    fn into_func(self, call_table: &mut CallTable, signature: PscSignature) -> usize;

    #[doc(hidden)]
    fn signature() -> Signature;
}

macro_rules! impl_into_func {
    ($($params:ident),* => $($returns:ident),*) => {
        #[allow(unused_assignments)]
        #[allow(non_snake_case)]
        #[allow(unused)]
        impl<F, T, $($params,)* $($returns,)*> IntoFunc<T, ($($params,)*), ($($returns,)*)> for F
        where
            F: Fn(Caller<'_, T>, $($params),*) -> Result<($($returns),*), Exception> + Send + Sync + 'static,
            T: 'static,
            $(
                $params: NativeType,
            )*
            $(
                $returns: NativeType,
            )*
        {
            #[expect(private_interfaces)]
            fn into_func(self, call_table: &mut CallTable, signature: PscSignature) -> usize {
                let callback = move |ctx: *const RawContext, params: *const RawVal, returns: *mut RawVal| -> i32 {
                    let ctx = unsafe { &*ctx };

                    let mut index = 0;
                    $(
                        let $params = unsafe { $params::from_raw(&*params.add(index)) };
                        index += 1;
                    )*

                    let caller = Caller {
                        ctx,
                        _marker: PhantomData,
                    };

                    match self(caller, $($params),*) {
                        Ok(($($returns),*)) => {
                            validate_return_value_types(ctx, &($($returns,)*), &signature);

                            let mut index = 0;
                            $(
                                unsafe {
                                    returns.add(index).write($returns.to_raw());
                                    index += 1;
                                }
                            )*

                            RESULT_OK
                        },
                        // When returning without RESULT_OK we can leave all return
                        // values uninitialized.
                        Err(err) => err as i32,
                    }
                };

                call_table.insert(callback)
            }

            fn signature() -> Signature {
                let mut params = Vec::new();
                $(
                    params.push($params::TYPE);
                )*

                let mut returns = Vec::new();
                $(
                    returns.push($returns::TYPE);
                )*

                Signature { params, returns }
            }
        }
    };
}

fn validate_return_value_types<T>(ctx: &RawContext, values: &T, signature: &PscSignature)
where
    T: NativeTuple,
{
    let pool = unsafe { &(*ctx.state).pool };
    let res = pool.resources();

    for (is, expect) in values.type_ids(pool).zip(&signature.returns) {
        match is {
            TypeRef::Concrete(is) => {
                if is == *expect {
                    continue;
                }

                let is_ty = res.types.get(is).unwrap();
                if !is_ty.info.classes.contains(expect) {
                    panic!("type {:?} is not compatible with {:?}", is, expect);
                }
            }
            TypeRef::Null => {
                let expect = res.types.get(*expect).unwrap();
                if !expect.info.allows_null {
                    panic!("type {:?} is null, but the type is non-nullable", expect);
                }
            }
            TypeRef::Invalid => {
                panic!("returning invalid value");
            }
        }
    }
}

fn validate_parameter_types<T>(pool: &ObjectPool, params: &T, signature: &PscSignature)
where
    T: ?Sized + TypeArray,
{
    let res = pool.resources();

    for (is, expect) in params.type_ids(pool).zip(&signature.params) {
        match is {
            TypeRef::Concrete(is) => {
                if is == *expect {
                    continue;
                }

                let is_ty = res.types.get(is).unwrap();
                if !is_ty.info.classes.contains(expect) {
                    panic!("type {:?} is not compatible with {:?}", is, expect);
                }
            }
            TypeRef::Null => {
                let expect = res.types.get(*expect).unwrap();
                if !expect.info.allows_null {
                    panic!("type {:?} is null, but the type is non-nullable", expect);
                }
            }
            TypeRef::Invalid => {
                panic!("passing invalid value");
            }
        }
    }
}

impl_into_func!(=>);
impl_into_func!(=> R0);
impl_into_func!(A0 =>);
impl_into_func!(A0 => R0);
impl_into_func!(A0, A1 =>);
impl_into_func!(A0, A1 => R0);
impl_into_func!(A0, A1, A2 =>);
impl_into_func!(A0, A1, A2 => R0);
impl_into_func!(A0, A1, A2, A3 =>);
impl_into_func!(A0, A1, A2, A3 => R0);
impl_into_func!(A0, A1, A2, A3, A4 =>);
impl_into_func!(A0, A1, A2, A3, A4 => R0);
impl_into_func!(A0, A1, A2, A3, A4, A5 =>);
impl_into_func!(A0, A1, A2, A3, A4, A5 => R0);
impl_into_func!(A0, A1, A2, A3, A4, A5, A6 =>);
impl_into_func!(A0, A1, A2, A3, A4, A5, A6 => R0);
impl_into_func!(A0, A1, A2, A3, A4, A5, A6, A7 =>);
impl_into_func!(A0, A1, A2, A3, A4, A5, A6, A7 => R0);
impl_into_func!(A0, A1, A2, A3, A4, A5, A6, A7, A8 =>);
impl_into_func!(A0, A1, A2, A3, A4, A5, A6, A7, A8 => R0);
impl_into_func!(A0, A1, A2, A3, A4, A5, A6, A7, A8, A9 =>);
impl_into_func!(A0, A1, A2, A3, A4, A5, A6, A7, A8, A9 => R0);
impl_into_func!(A0, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10 =>);
impl_into_func!(A0, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10 => R0);
impl_into_func!(A0, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11 =>);
impl_into_func!(A0, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11 => R0);

/// A type that can be send to Papyrus functions.
pub unsafe trait NativeType: Copy {
    const TYPE: Type;

    unsafe fn from_raw(val: &RawVal) -> Self;

    fn to_raw(self) -> RawVal;

    #[doc(hidden)]
    #[expect(private_interfaces)]
    fn get_type_id(&self, pool: &ObjectPool) -> TypeRef;
}

unsafe impl NativeType for bool {
    const TYPE: Type = Type::I8;

    unsafe fn from_raw(val: &RawVal) -> Self {
        unsafe { val.bool }
    }

    fn to_raw(self) -> RawVal {
        RawVal { bool: self }
    }

    #[doc(hidden)]
    #[expect(private_interfaces)]
    fn get_type_id(&self, pool: &ObjectPool) -> TypeRef {
        Val::Bool(*self).get_type_id(pool)
    }
}

unsafe impl NativeType for i32 {
    const TYPE: Type = Type::I32;

    unsafe fn from_raw(val: &RawVal) -> Self {
        unsafe { val.i32 }
    }

    fn to_raw(self) -> RawVal {
        RawVal { i32: self }
    }

    #[doc(hidden)]
    #[expect(private_interfaces)]
    fn get_type_id(&self, pool: &ObjectPool) -> TypeRef {
        Val::I32(*self).get_type_id(pool)
    }
}

unsafe impl NativeType for f32 {
    const TYPE: Type = Type::F32;

    unsafe fn from_raw(val: &RawVal) -> Self {
        unsafe { val.f32 }
    }

    fn to_raw(self) -> RawVal {
        RawVal { f32: self }
    }

    #[doc(hidden)]
    #[expect(private_interfaces)]
    fn get_type_id(&self, pool: &ObjectPool) -> TypeRef {
        Val::F32(*self).get_type_id(pool)
    }
}

unsafe impl NativeType for ObjectAddr {
    const TYPE: Type = Type::Pointer;

    unsafe fn from_raw(val: &RawVal) -> Self {
        unsafe { val.object }
    }

    fn to_raw(self) -> RawVal {
        RawVal { object: self }
    }

    #[doc(hidden)]
    #[expect(private_interfaces)]
    fn get_type_id(&self, pool: &ObjectPool) -> TypeRef {
        Val::Object(*self).get_type_id(pool)
    }
}

#[expect(private_bounds)]
pub unsafe trait NativeTuple: TypeArray {
    const SIZE: usize;
    const TYPES: &[Type];

    unsafe fn write_raw(self, ptr: *mut RawVal);

    unsafe fn from_raw(ptr: *const RawVal) -> Self;
}

unsafe impl NativeTuple for () {
    const SIZE: usize = 0;
    const TYPES: &[Type] = &[];

    unsafe fn from_raw(_ptr: *const RawVal) -> Self {
        ()
    }

    unsafe fn write_raw(self, _ptr: *mut RawVal) {}
}

impl TypeArray for () {
    fn type_ids(&self, _pool: &ObjectPool) -> impl Iterator<Item = TypeRef> {
        std::iter::empty()
    }
}

unsafe impl<T> NativeTuple for T
where
    T: NativeType,
{
    const SIZE: usize = Self::TYPES.len();
    const TYPES: &[Type] = &[T::TYPE];

    unsafe fn from_raw(ptr: *const RawVal) -> Self {
        unsafe { T::from_raw(&*ptr) }
    }

    unsafe fn write_raw(self, ptr: *mut RawVal) {
        unsafe {
            ptr.write(self.to_raw());
        }
    }
}

impl<T> TypeArray for T
where
    T: NativeType,
{
    fn type_ids(&self, pool: &ObjectPool) -> impl Iterator<Item = TypeRef> {
        std::iter::once(self.get_type_id(pool))
    }
}

macro_rules! impl_native_tuple {
    ($($t:ident),*) => {
        #[allow(unused_assignments)]
        #[allow(non_snake_case)]
        unsafe impl<$($t,)*> NativeTuple for ( $($t,)* )
        where
            $(
                $t: NativeType,
            )*
        {
            const TYPES: &[Type] = &[ $($t::TYPE,)* ];
            const SIZE: usize = Self::TYPES.len();

            unsafe fn from_raw(ptr: *const RawVal) -> Self {
                let mut offset = 0;
                $(
                    let $t = unsafe { $t::from_raw(&*ptr.add(offset)) };
                    offset += 1;
                )*

                ($($t,)*)
            }

            unsafe fn write_raw(self, ptr: *mut RawVal) {
                let ( $($t,)* ) = self;

                let mut offset = 0;
                $(
                    unsafe { ptr.add(offset).write($t.to_raw()); }
                    offset += 1;
                )*
            }
        }

        #[allow(non_snake_case)]
        impl<$($t,)*> TypeArray for ( $($t,)* )
        where
            $(
                $t: NativeType,
            )*
        {
            fn type_ids(&self, pool: &ObjectPool) -> impl Iterator<Item = TypeRef> {
                let ( $($t,)* ) = self;
                [
                    $(
                        $t.get_type_id(pool),
                    )*
                ]
                .into_iter()
            }
        }
    };
}

impl_native_tuple!(T0);
impl_native_tuple!(T0, T1);
impl_native_tuple!(T0, T1, T2);
impl_native_tuple!(T0, T1, T2, T3);
impl_native_tuple!(T0, T1, T2, T3, T4);
impl_native_tuple!(T0, T1, T2, T3, T4, T5);
impl_native_tuple!(T0, T1, T2, T3, T4, T5, T6);
impl_native_tuple!(T0, T1, T2, T3, T4, T5, T6, T7);
impl_native_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8);
impl_native_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum TypeRef {
    Concrete(TypeId),
    Null,
    /// Invalid type reference.
    Invalid,
}

trait TypeArray {
    fn type_ids(&self, pool: &ObjectPool) -> impl Iterator<Item = TypeRef>;
}

impl TypeArray for [Val] {
    fn type_ids(&self, pool: &ObjectPool) -> impl Iterator<Item = TypeRef> {
        self.iter().map(|v| v.get_type_id(pool))
    }
}
