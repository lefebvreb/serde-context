check:
    cargo +nightly fmt
    cargo clippy -- -Wclippy::undocumented_unsafe_blocks
    just doc
    just test

doc *FLAGS:
    RUSTDOCFLAGS="--cfg docsrs -Z unstable-options --generate-link-to-definition" cargo +nightly doc {{FLAGS}}

test:
    cargo +nightly miri test
    for file in examples/*.rs; do cargo +nightly miri run --example "$(basename "$file" .rs)"; done