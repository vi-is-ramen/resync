default:
    @just --list

check:
    @cargo fmt --all
    @cargo clippy --all-targets --all-features -- -D warnings

test:
    @cargo test --all-features
    @cargo test --no-default-features
    @cargo test

pre-commit: check test

docs:
    @RUSTDOCFLAGS="--cfg docsrs" cargo doc --no-deps --all-features --open
