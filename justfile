default:
    @just --list

check:
    @cargo fmt --all -- --check
    @cargo clippy --all-targets --all-features -- -D warnings

test:
    @cargo test --all-features
    @cargo test --no-default-features
    @cargo test

pre-commit: check test

commit *a: pre-commit
    @git add -A
    @git commit "{{a}}"

docs:
    @RUSTDOCFLAGS="--cfg docsrs" cargo doc --no-deps --all-features --open
