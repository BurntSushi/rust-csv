#!/bin/sh

set -ex

cargo build --verbose
cargo doc --verbose
cargo test --verbose
cargo test --verbose --manifest-path csv-core/Cargo.toml
cargo test --verbose --manifest-path csv-index/Cargo.toml
if [ "$TRAVIS_RUST_VERSION" = "stable" ]; then
  rustup component add rustfmt
  cargo fmt -- --check
fi
if [ "$TRAVIS_RUST_VERSION" = "nightly" ]; then
  ci/check-copy cookbook
  ci/check-copy tutorial
  cargo bench --verbose --no-run
fi
