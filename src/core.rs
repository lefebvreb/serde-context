use core::any::{TypeId, type_name};
use core::cell::UnsafeCell;
use core::error::Error;
use core::fmt::{Debug, Display, Formatter, Result as FmtResult};
use core::marker::PhantomData;
use core::mem::{MaybeUninit, take, transmute_copy};
use core::ptr::{NonNull, copy_nonoverlapping};

use rustc_hash::{FxBuildHasher, FxHashMap};

#[repr(C)]
#[derive(Copy, Clone)]
struct ErasedRef {
    ptr: NonNull<()>,
    meta: MaybeUninit<usize>,
}

impl ErasedRef {
    const fn from_ref<T: ?Sized>(value: &T) -> Self {
        let mut this = MaybeUninit::uninit();
        // SAFETY: both pointers are obviously valid. `Self` is exactly as large as a wide pointer,
        // such that an `&T` always fit inside a `Self`.
        unsafe { copy_nonoverlapping(&raw const value as *const u8, &raw mut this as *mut u8, size_of::<&T>()) };
        // SAFETY: The `ptr` field of `this` has been initialized by the memcpy just above, and the `meta` has been
        // left uninitialized iff `&T` is a thin pointer.
        unsafe { this.assume_init() }
    }

    /// # Safety
    ///
    /// * `self` must have been obtained through a call to `Self::from_ref::<T>`.
    /// * `'a` must not outlive the original lifetime of the reference that was used
    ///   in the call to `Self::from_ref::<T>`.
    pub(crate) const unsafe fn as_ref<'a, T: ?Sized>(self) -> &'a T {
        // SAFETY: we know this pointer was built from a reference to `T`, and that the lifetime `'a` is
        // shorter than that of the reference it came from.
        unsafe { transmute_copy::<Self, &T>(&self) }
    }
}

type ErasedRefMap = FxHashMap<TypeId, ErasedRef>;

#[repr(transparent)]
pub struct ContextCell(UnsafeCell<ErasedRefMap>);

impl ContextCell {
    const fn new() -> Self {
        Self(UnsafeCell::new(FxHashMap::with_hasher(FxBuildHasher)))
    }

    fn insert(&self, key: TypeId, value: ErasedRef) {
        // SAFETY: `Self` isn't `Sync`, and no borrows to its content are ever returned.
        let this = unsafe { &mut *self.0.get() };
        this.insert(key, value);
    }

    fn take_if_empty(&self) -> Option<ErasedRefMap> {
        // SAFETY: `Self` isn't `Sync`, and no borrows to its content are ever returned.
        let this = unsafe { &mut *self.0.get() };
        (!this.is_empty()).then(|| take(this))
    }

    fn replace_with(&self, states: ErasedRefMap) {
        // SAFETY: `Self` isn't `Sync`, and no borrows to its content are ever returned.
        let this = unsafe { &mut *self.0.get() };
        *this = states;
    }

    fn clear(&self) {
        // SAFETY: `Self` isn't `Sync`, and no borrows to its content are ever returned.
        let this = unsafe { &mut *self.0.get() };
        this.clear();
    }

    fn get(&self, key: TypeId) -> Option<ErasedRef> {
        // SAFETY: `Self` isn't `Sync`, and no borrows to its content are ever returned.
        let this = unsafe { &*self.0.get() };
        this.get(&key).copied()
    }
}

/// Types that can be passed as context for (de)serialization.
///
/// You need not implement this trait yourself, it is already implemented for:
/// * the unit type `()`, when context is required by an API but you have none to provide.
/// * references to `'static` types (sized or not).
/// * tuples of these types implementing [`Context`], up to 12 elements in length. This includes
///   tuples of references, and tuples of tuples or references, and so on.
///
/// You pass context to [`serialize_with_context`](crate::serialize_with_context) or
/// [`deserialize_with_context`](crate::deserialize_with_context), and it can then be
/// accessed and used in [`Serialize`](https://docs.rs/serde/latest/serde/trait.Serialize.html) and
/// [`Deserialize`](https://docs.rs/serde/latest/serde/trait.Deserialize.html)
/// implementations by calling [`context_scope`](crate::context_scope). See these function's respective
/// documentations for precise usage, as well as this [`crate`]'s top-level documentation and
/// [its examples directory](https://codeberg.org/blefebvre/serde_context/src/branch/main/examples).
pub trait Context: Sized {
    #[doc(hidden)]
    fn __insert(self, cx: &ContextCell);
}

impl Context for () {
    fn __insert(self, _: &ContextCell) {}
}

impl<T: 'static + ?Sized> Context for &T {
    fn __insert(self, cx: &ContextCell) {
        cx.insert(TypeId::of::<T>(), ErasedRef::from_ref(self));
    }
}

macro_rules! for_all_tuples {
    ($macro:ident $t:ident $($ts:ident)+) => {
        for_all_tuples!($macro $($ts)+);
        $macro!(#[cfg_attr(docsrs, doc(hidden))] $t $($ts)+);
    };
    ($macro:ident $t:ident) => {};
    ($macro:ident) => {
        $macro!(#[cfg_attr(docsrs, doc(fake_variadic), doc = "This trait is implemented for tuples up to 12 items long.")] T);
        for_all_tuples!($macro A B C D E F G H I J K L);
    };
}

macro_rules! impl_context {
    (#[$attr:meta] $($t:ident)*) => {
        #[$attr]
        impl<$($t: Context,)*> Context for ($($t,)*) {
            #[allow(non_snake_case)]
            fn __insert(self, cx: &ContextCell) {
                let ($($t,)*) = self;
                $($t.__insert(cx);)*
            }
        }
    };
}

for_all_tuples!(impl_context);

/// Error that may be returned when requested context is missing.
///
/// This error can be returned by [`ContextRef::get`] whenever the requested `T`
/// was not provided in the global context.
pub struct MissingContextError<T: ?Sized>(PhantomData<T>);

impl<T: ?Sized> Debug for MissingContextError<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_tuple("MissingContextError").finish()
    }
}

impl<T: ?Sized> Display for MissingContextError<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "type \"{}\" is missing from context", type_name::<T>())
    }
}

impl<T: ?Sized> Error for MissingContextError<T> {}

/// Reference to the global context used to retrieve data within it.
///
/// Instances of this type can be obtained by using [`context_scope`](crate::context_scope). The
/// [`ContextRef::get`] method can then be used to get references to data provided to this
/// (de)serialization process in [`serialize_with_context`](crate::serialize_with_context) or
/// [`deserialize_with_context`](crate::deserialize_with_context). See these items respective
/// documentations for more informations.
#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct ContextRef<'cx>(&'cx ContextCell);

impl<'cx> ContextRef<'cx> {
    /// Attempts to get a reference to some value passed to this (de)serialization's context.
    ///
    /// # Error
    ///
    /// Returns a [`MissingContextError`] when no `T` was found in context.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use serde_context::{context_scope, MissingContextError};
    /// #
    /// struct Foo;
    ///
    /// # fn returns_error() -> anyhow::Result<()> {
    /// context_scope(|cx| {
    ///     let foo = cx.get::<Foo>()?;
    ///     // Do something with foo...
    ///     # _ = foo;
    ///     # Ok(())
    /// })
    /// # }
    ///
    /// # assert!(matches!(returns_error(), Err(_)))
    /// ```
    pub fn get<T: 'static + ?Sized>(self) -> Result<&'cx T, MissingContextError<T>> {
        let ptr = self.0.get(TypeId::of::<T>()).ok_or(MissingContextError(PhantomData))?;
        // SAFETY: we know this erased ref was build from a `&T` that is still valid.
        Ok(unsafe { ptr.as_ref::<'cx, T>() })
    }
}

thread_local! {
    static CONTEXT: ContextCell = const { ContextCell::new() };
}

#[repr(transparent)]
pub(crate) struct ContextSwapGuard(Option<ErasedRefMap>);

impl Drop for ContextSwapGuard {
    fn drop(&mut self) {
        CONTEXT.with(|cx| {
            // Restore previous context, if any, and drop the current one, or just clear the current one.
            match self.0.take() {
                Some(prev_context) => cx.replace_with(prev_context),
                None => cx.clear(),
            }
        })
    }
}

pub(crate) fn swap_global_context<C: Context>(context: C) -> ContextSwapGuard {
    CONTEXT.with(|cx| {
        let guard = ContextSwapGuard(cx.take_if_empty());
        context.__insert(cx);
        guard
    })
}

pub(crate) fn with_global_context<R>(f: impl FnOnce(ContextRef) -> R) -> R {
    CONTEXT.with(|cx| f(ContextRef(cx)))
}
