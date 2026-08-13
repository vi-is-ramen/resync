default:
    @just --list

check:
    @cargo fmt --all
    @cargo clippy --all-targets --all-features -- -D warnings

test:
    @cargo test --all-features
    @cargo test --no-default-features
    @python scripts/test.py

clean:
    @cargo clean

pre-commit: clean check test

docs:
    @RUSTDOCFLAGS="--cfg docsrs" cargo doc --no-deps --all-features --open
