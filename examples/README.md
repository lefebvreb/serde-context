# Examples

Contextful serialization examples:
* `sum.rs`: serialize a struct containing integers while summing all those that are serialized while skipping the `#[serde(skip)]`ed ones.

Contextful deserialization examples:
* `flatten_seq.rs`: deserialize a sequence of sequences of values in a single flat vec without intermediary allocations.
* `ids.rs`: deserialize a sequence of strings, associating a unique sequential ID to each distinct strings.
