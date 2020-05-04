#!/bin/sh

set -ex

cargo build --features async --verbose
cargo doc --features async --verbose

# Our dev dependencies want newer versions of Rust. Instead of bumping our
# MSRV, we just don't test on our MSRV.
if [ "$TRAVIS_RUST_VERSION" = "1.33.0" ]; then
  exit 0
fi

cargo test --features async --verbose
cargo test --features async --verbose --manifest-path csv-core/Cargo.toml
cargo test --features async --verbose --manifest-path csv-index/Cargo.toml
if [ "$TRAVIS_RUST_VERSION" = "stable" ]; then
  rustup component add rustfmt
  cargo fmt -- --check

  ci/check-copy cookbook
  ci/check-copy tutorial
fi
if [ "$TRAVIS_RUST_VERSION" = "nightly" ]; then
  cargo bench --features async --verbose --no-run
fi
