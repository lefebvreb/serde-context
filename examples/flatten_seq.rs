//! In this example, we deserialize a sequence of sequences of numbers
//! into a single flat vec, without intermediary allocations.
//!
//! This examples mirrors [the one](https://docs.rs/serde/latest/serde/de/trait.DeserializeSeed.html#example)
//! provided for [`DeserializeSeed`](serde::de::DeserializeSeed)'s in the offical [`serde`] crate.

use std::cell::RefCell;
use std::fmt;
use std::marker::PhantomData;

use anyhow::Result;
use serde::de::{SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_context::{context_scope, deserialize_with_context};

// This is the source JSON document.
const JSON_NUMBERS: &str = "[[1, 2], [3, 4, 5], [6]]";

// A type that deserializes a `T` but then inserts it into context
// of type `RefCell<Vec<T>>` instead of returning it.
struct Value<T>(PhantomData<T>);

impl<'de, T: Deserialize<'de> + 'static> Deserialize<'de> for Value<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Open context scope.
        context_scope(|cx| {
            // Deserialize the value.
            let value = T::deserialize(deserializer)?;
            // If a context of type `RefCell<Vec<T>>` was provided, insert the value into it.
            if let Ok(vec) = cx.get::<RefCell<Vec<T>>>() {
                vec.borrow_mut().push(value);
            }
            Ok(Self(PhantomData))
        })
    }
}

// A type that deserializes a sequence of `T`s, but does not store the values.
struct Collect<T>(PhantomData<T>);

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Collect<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct CollectVisitor<T>(PhantomData<T>);

        impl<'de, T: Deserialize<'de>> Visitor<'de> for CollectVisitor<T> {
            type Value = Collect<T>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "a sequence")
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                // Just deserialize the value without storing them.
                while let Ok(Some(_)) = seq.next_element::<T>() {}
                Ok(Collect(PhantomData))
            }
        }

        deserializer.deserialize_seq(CollectVisitor::<T>(PhantomData))
    }
}

fn main() -> Result<()> {
    // Create the context type that will receive our values.
    let nums = RefCell::new(Vec::<i32>::new());

    // Deserialize the JSON source as a sequence of sequences of `T`s,
    // but storing the elements in the context of type `RefCell<Vec<T>>` instead
    // or returning them directly.
    let mut deserializer = serde_json::Deserializer::from_str(JSON_NUMBERS);
    let _: Collect<Collect<Value<i32>>> = deserialize_with_context(&mut deserializer, &nums)?;

    // Extract the flatten vec from its refcell.
    let nums = nums.into_inner();
    assert_eq!(nums, vec![1, 2, 3, 4, 5, 6]);

    Ok(())
}
