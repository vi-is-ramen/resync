## Description

<!-- Please include a summary of the change and which issue is fixed. Please also include relevant motivation and context. -->

Fixes # (issue number)

## Type of Change

<!-- Please delete options that are not relevant. -->

- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update (changes to `README.md` or `src/guide.rs`)
- [ ] Refactoring (no functional changes, no API changes)

## Checklist

Before submitting this PR, please ensure you have completed the following:

- [ ] **Conventional Commits:** My commit messages follow the [Conventional Commits](https://www.conventionalcommits.org/) specification (e.g., `feat: add ticket lock`, `fix: prevent deadlock in nested lock`). This is required for our automated release pipeline.
- [ ] **Formatting:** I have run `cargo fmt --all` and `just check` (if using `just`).
- [ ] **Linting:** I have run `cargo clippy --all-targets --all-features -- -D warnings` and resolved all warnings.
- [ ] **Testing:** I have added tests that prove my fix is effective or that my feature works.
- [ ] **Documentation:** I have updated the guidebook (`src/guide.rs`) and inline `///` doc-comments if I added or changed public APIs.

## Additional Context

<!-- Add any other context or screenshots about the pull request here. -->
