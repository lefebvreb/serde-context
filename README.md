# Serde Context

[<img alt="crates.io" src="https://img.shields.io/crates/v/serde_context.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/serde_context)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-serde_context-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" height="20">](https://docs.rs/serde_context)

Convenient contextful (de)serialization compatible with the `serde` ecosystem.

You can find examples in the this crate's [examples directory](https://github.com/lefebvreb/serde-context/tree/main/examples).

## Limitations

There are a few limitations to this crate:

* Only immutable references may be passed as context. You may still use interior mutability ([`Cell`](std::cell::Cell)s,
[`RefCell`](std::cell::RefCell)s, [`UnsafeCell`](std::cell::UnsafeCell)s...) to mutate state.
* Only types implementing `'static` can be used as context for (de)serialization. 
Unsized types are supported, though.
* There is no type-checking of required context. Trying to (de)serialize a type that requires some context
but failing to provide it will result in a runtime error.

## Alternatives

There are a few alternatives to this crate you might want to consider, each with their pros and cons:

* [`DeserializeSeed`](https://docs.rs/serde/latest/serde/de/trait.DeserializeSeed.html): the official
serde stateful deserialization-only API. Requires implementing that trait for each type you are interested
in deserializing, and no derive macro is provided (nor are there any plans to ever make an official one).
* [`serde_state`](https://docs.rs/serde_state/latest/serde_state): a fork of serde that supports stateful
(de)serialization. Unfortunately, it looks to be unmaintained.
* [`serde_seeded`](https://docs.rs/serde-seeded/latest/serde_seeded): a crate providing an alternative version
of the `DeserializeSeed` trait with a derive macro for it. Relatively young but very promising.

### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>
