default:
    @just --list

# Run all standard checks
check:
    cargo fmt --all -- --check
    cargo clippy --all-targets --all-features -- -D warnings

# Run tests
test:
    cargo test --all-features

# Generate local docs
docs:
    RUSTDOCFLAGS="--cfg docsrs" cargo doc --no-deps --all-features --open
