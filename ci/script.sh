#!/bin/sh

set -ex

ci/check-copy cookbook
ci/check-copy tutorial
cargo build --verbose
cargo doc --verbose
cargo test --verbose
cargo test --verbose --manifest-path csv-core/Cargo.toml
cargo test --verbose --manifest-path csv-index/Cargo.toml
if [ "$TRAVIS_RUST_VERSION" = "nightly" ]; then
  cargo bench --verbose --no-run
fi
