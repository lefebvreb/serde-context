//! In this example, we deserialize a list of fruits given by their names,
//! and give each distinct fruit name a unique ID, with possible duplicates.

use std::cell::RefCell;
use std::collections::HashMap;

use anyhow::Result;
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use serde_context::{context_scope, deserialize_with_context};

// This is the source JSON document.
const JSON_FRUITS: &str = r#"["apple", "banana", "orange", "banana"]"#;

// A type used to allocate new IDs for unique names.
type IdAlloc = RefCell<HashMap<String, Id>>;

// A numerical ID, one per unique name.
#[derive(Copy, Clone, PartialEq, Debug)]
struct Id(usize);

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Open context scope.
        context_scope(|cx| {
            // Extract and borrow ID allocator from context, or return an error if missing.
            let mut alloc = cx.get::<IdAlloc>().map_err(D::Error::custom)?.borrow_mut();
            // Deserialize the fruit's name.
            let name = String::deserialize(deserializer)?;
            // Get ID corresponding to this name, or add a new one.
            let next_index = alloc.len();
            let id = *alloc.entry(name).or_insert(Id(next_index));
            // Return the ID.
            Ok(id)
        })
    }
}

fn main() -> Result<()> {
    // Create the ID allocator.
    let alloc = IdAlloc::default();

    // Deserialize the fruit names into a list of IDs. We can get the names
    // back in the allocator.
    let mut deserializer = serde_json::Deserializer::from_str(JSON_FRUITS);
    let ids: Vec<Id> = deserialize_with_context(&mut deserializer, &alloc)?;

    // Each unique fruit got an ID, with duplicated names sharing a same ID.
    assert_eq!(ids, vec![Id(0), Id(1), Id(2), Id(1)]);

    // Make sure that each fruit got an ID.
    let keys = alloc.into_inner();
    assert_eq!(keys.get("apple"), Some(&Id(0)));
    assert_eq!(keys.get("banana"), Some(&Id(1)));
    assert_eq!(keys.get("orange"), Some(&Id(2)));
    assert_eq!(keys.get("peach"), None);

    Ok(())
}
