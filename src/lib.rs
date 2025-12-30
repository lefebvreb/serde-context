//! [`serde_context`](crate) provides convenient contextful (de)serialization compatible with the [`serde`](https://docs.rs/serde/latest/serde/index.html) ecosystem.
//!
//! # Basic usage
//!
//! Any type that implements [`Context`] (blanket implementations are provided) can be passed to [`serialize_with_context`]
//! or [`deserialize_with_context`] and then accessed by using [`context_scope`] within
//! [`Serialize`](https://docs.rs/serde/latest/serde/trait.Serialize.html) and
//! [`Deserialize`](https://docs.rs/serde/latest/serde/trait.Deserialize.html) implementations. See these items
//! respective documentations for more information.
//!
//! # Features
//!
//! This crate provides no conditional compilation features. It requires the standard library
//! because it uses [`thread_local`]s to store and access context.
//!
//! # Limitations
//!
//! There are a few limitations to this crate:
//!
//! * Only immutable references may be passed as context. You may still use interior mutability ([`Cell`](std::cell::Cell)s,
//!   [`RefCell`](std::cell::RefCell)s, [`UnsafeCell`](std::cell::UnsafeCell)s...) to mutate state.
//! * Only types implementing `'static` can be used as context for (de)serialization.
//!   Unsized types are supported, though.
//! * There is no type-checking of required context. Trying to (de)serialize a type that requires some context
//!   but failing to provide it will result in a runtime error.
//!
//! # Alternatives
//!
//! There are a few alternatives to this crate you might want to consider, each with their pros and cons:
//!
//! * [`DeserializeSeed`](https://docs.rs/serde/latest/serde/de/trait.DeserializeSeed.html): the official
//!   serde stateful deserialization-only API. Requires implementing that trait for each type you are interested
//!   in deserializing, and no derive macro is provided (nor are there any plans to ever make an official one).
//! * [`serde_state`](https://docs.rs/serde_state/latest/serde_state): a fork of serde that supports stateful
//!   (de)serialization. Unfortunately, it looks to be unmaintained.
//! * [`serde_seeded`](https://docs.rs/serde-seeded/latest/serde_seeded): a crate providing an alternative version
//!   of the `DeserializeSeed` trait with a derive macro for it. Relatively young but very promising.
//!
//! # Example
//!
//! This minimal, contrived example showcases the basics of this crate: passing context
//! to a serialization process and accessing it within a [`Serialize`](https://docs.rs/serde/latest/serde/trait.Serialize.html)
//! implementation. You can find more serious examples in this crate's
//! [examples directory](https://codeberg.org/blefebvre/serde_context/src/branch/main/examples).
//!
//! More concretely here, we are inserting a multiplier (`MyMultiplier`) inside of
//! this serialization's global context, and then using it to multiply every numbers
//! (`MyNumber`) by it before serialization.
//!
//! ```rust
//! # use serde::{Serialize, Serializer};
//! # use serde::ser::Error;
//! # use serde_context::{context_scope, serialize_with_context};
//! #
//! struct MyMultiplier(i32);
//!
//! struct MyNumber(i32);
//!
//! impl Serialize for MyNumber {
//!     fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
//!         context_scope(|cx| {
//!             let mul = cx.get::<MyMultiplier>().map_err(S::Error::custom)?;
//!             (self.0 * mul.0).serialize(serializer)
//!         })
//!     }
//! }
//!
//! # fn main() -> anyhow::Result<()> {
//! let mut bytes = Vec::<u8>::new();
//! let mut serializer = serde_json::Serializer::new(&mut bytes);
//! serialize_with_context(&MyNumber(6), &mut serializer, &MyMultiplier(7))?;
//! assert_eq!(bytes, b"42");
//! # Ok(())
//! # }
//! ```

#![cfg_attr(docsrs, allow(internal_features), feature(rustdoc_internals))]

use serde_core::{Deserialize, Deserializer, Serialize, Serializer};

pub use crate::core::{Context, ContextRef, MissingContextError};

mod core;

/// Serialize a value with some provided context.
///
/// Provided `context` can then be accessed using [`context_scope`] in implementations of
/// [`Serialize`]. This function is meant to be the contextful version of `value.serialize(serializer)`.
///
/// # Example
///
/// This is only a minimal example for this function's usage. You can find more in this [`crate`]'s
/// top-level documentation as well as [its examples directory](https://codeberg.org/blefebvre/serde_context/src/branch/main/examples).
///
/// ```rust
/// # use serde_context::serialize_with_context;
/// #
/// # fn main() -> anyhow::Result<()> {
/// let mut bytes = Vec::<u8>::new();
/// let mut serializer = serde_json::Serializer::new(&mut bytes);
/// serialize_with_context(&vec![1, 2, 3], &mut serializer, ())?;
/// assert_eq!(bytes, b"[1,2,3]");
/// # Ok(())
/// # }
/// ```
pub fn serialize_with_context<T, S, C>(value: &T, serializer: S, context: C) -> Result<S::Ok, S::Error>
where
    T: Serialize + ?Sized,
    S: Serializer,
    C: Context,
{
    let _guard = core::swap_global_context(context);
    value.serialize(serializer)
}

/// Deserialize a value with some provided context.
///
/// Provided `context` can then be accessed using [`context_scope`] in implementations of
/// [`Deserialize`]. This function is meant to be the contextful version of `T::deserialize(deserializer)`.
///
/// # Example
///
/// This is only a minimal example for this function's usage. You can find more in this [`crate`]'s
/// top-level documentation as well as [its examples directory](https://codeberg.org/blefebvre/serde_context/src/branch/main/examples).
///
/// ```rust
/// # use serde_context::deserialize_with_context;
/// #
/// # fn main() -> anyhow::Result<()> {
/// let mut deserializer = serde_json::Deserializer::from_str("[1,2,3]");
/// let ids: Vec<i32> = deserialize_with_context(&mut deserializer, ())?;
/// assert_eq!(ids, vec![1, 2, 3]);
/// # Ok(())
/// # }
/// ```
pub fn deserialize_with_context<'de, T, D, C>(deserializer: D, context: C) -> Result<T, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
    C: Context,
{
    let _guard = core::swap_global_context(context);
    T::deserialize(deserializer)
}

/// This is the contextful version of [`Deserialize::deserialize_in_place`], the hidden method
/// that provides in-place deserialization.
///
/// See [`deserialize_with_context`].
#[doc(hidden)]
pub fn deserialize_in_place_with_context<'de, T, D, C>(
    deserializer: D,
    place: &mut T,
    context: C,
) -> Result<(), D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
    C: Context,
{
    let _guard = core::swap_global_context(context);
    T::deserialize_in_place(deserializer, place)
}

/// Access context provided to [`serialize_with_context`] or [`deserialize_with_context`].
///
/// This function is meant to be used within [`Serialize`] and [`Deserialize`] implementations
/// to access context for contextful (de)serialization.
///
/// # Example
///
/// This is only a minimal example for this function's usage. You can find more in this [`crate`]'s
/// top-level documentation as well as [its examples directory](https://codeberg.org/blefebvre/serde_context/src/branch/main/examples).
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
#[inline(always)]
pub fn context_scope<R>(f: impl for<'cx> FnOnce(ContextRef<'cx>) -> R) -> R {
    core::with_global_context(f)
}
