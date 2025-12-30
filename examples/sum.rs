//! In this example, we serialize a struct containing several sequences of integers,
//! some skipped during serialization. We used contextful serialization to sum those
//! integers that are effectively serialized, and only those ones.

use std::cell::Cell;

use anyhow::Result;
use serde::ser::Error;
use serde::{Serialize, Serializer};
use serde_context::{context_scope, serialize_with_context};

// Type used as context to collect the summation result.
#[derive(Default)]
struct Total(Cell<i32>);

// A wrapper around an integer, that gets added to the total during serialization.
struct Int(i32);

impl Serialize for Int {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // Open context scope.
        context_scope(|cx| {
            // Extract and borrow total value from context, return an error if missing.
            let total = cx.get::<Total>().map_err(S::Error::custom)?;
            // Add this value to the total.
            total.0.set(total.0.get() + self.0);
            // Serialize this value as in integer.
            self.0.serialize(serializer)
        })
    }
}

// Type we want to deserialize.
#[derive(Serialize)]
struct MyInts {
    serialized: Vec<Int>,
    #[serde(skip)]
    _skipped: Vec<Int>,
}

fn main() -> Result<()> {
    // Initialize our total, that will be passed as context.
    let total = Total::default();

    // Values to serialize.
    let value = MyInts {
        serialized: vec![Int(1), Int(2), Int(3)],
        _skipped: vec![Int(4), Int(5), Int(6)],
    };

    // Serialize our value to text.
    let mut bytes = Vec::<u8>::new();
    let mut serializer = serde_json::Serializer::new(&mut bytes);
    serialize_with_context(&value, &mut serializer, &total)?;

    // We only serialized the first sequence, and also only summed it.
    assert_eq!(bytes, br#"{"serialized":[1,2,3]}"#);
    assert_eq!(total.0.get(), 6);

    Ok(())
}
