default:
    @just --list

check:
    @cargo fmt --all
    @cargo clippy --all-targets --all-features -- -D warnings

test:
    @cargo test --all-features --all-targets
    @cargo test --no-default-features --all-targets
    @cargo test --doc
    @python scripts/test.py

chlog *a:
    @python scripts/chlog.py {{a}}

clean:
    @cargo clean

pre-commit: check test

docs:
    @RUSTDOCFLAGS="--cfg docsrs" cargo doc --no-deps --all-features --open
