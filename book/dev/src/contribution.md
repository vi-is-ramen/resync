# No Need to Deep Dive!

You don't need to read the entire codebase to make a meaningful contribution.
Whether you want to fix a typo, add a missing test, or implement a small battery,
the Resync project is designed to let you drop in, do your work, and submit a PR
without getting lost in the weeds.

This chapter is your survival guide for contributing safely and efficiently.

---

## 1. Finding Something to Work On

Before touching any code, find a task. We use GitHub Issues to track bugs,
feature requests, and ideas.

- **Bug Reports**: Look for issues labeled `bug`. These usually include a
  reproduction snippet and expected behavior.
- **Feature Requests**: Look for issues labeled `enhancement`. These describe
  new primitives or batteries to add.
- **Good First Issues**: If you are new to Resync, look for the
  `good first issue` label.

> **TIP**
>
> If you have a new idea that isn't tracked yet, please open a **Feature Request**
> issue first. Discussing the design before writing code saves everyone's time.

---

## 2. Setting Up Your Environment

Resync relies on a specific set of tools. You don't need to install everything
manually — just check the `Needsfile` in the repository root:

```text
rustup
python3
```

Make sure you have `rustup` and `python3` installed. Next, install `just`, our
command runner (similar to `make`, but better (kinda)):

```bash
cargo install just
# or use OS' package manager if it provides Just
```

Finally, let `rustup` install the pinned nightly toolchain:

```bash
rustup show
```

This reads `rust-toolchain.toml` and ensures you are on the exact nightly
version required to compile Resync's unstable features (like `const_trait_impl`).

---

## 3. Making Your Changes

Create a new branch for your work. Never commit directly to `main`.

```bash
git checkout -b feat/issue-12345
```

### Writing the Code

If you are adding a new primitive, it belongs in `src/batteries/primitives/`.
If you are adding a new lock backend, it goes in `src/batteries/lock/`.

Remember the golden rules:
1. **Compose, don't hardcode.** Use existing `LockPolicy` and `RetryPolicy`
   traits.
2. **Respect the error taxonomy.** Use `AcquireError`, `TryLockError`, and
   `LockStatus` correctly.
3. **Write doc-tests.** Every public struct and method needs a `///` doc-comment
   with an executable `rust` code block.

### Running Tests Locally

Don't wait for CI to tell you something is broken. Run the test suite locally:

```bash
just test
```

This command runs unit tests across all feature combinations (using the Python
script `scripts/test.py` under the hood). If you are working on a specific
feature gate, you can test just that:

```bash
cargo test std # and other required feature flags
```

---

## 4. The Magic of `just pre-commit`

Before you even think about committing, you must pass the local quality gates.
We have a single command that runs everything CI will check:

```bash
just pre-commit
```

This command sequentially runs:
1. **`cargo fmt --all`** — Formats your code according to `rustfmt.toml`.
2. **`cargo clippy --all-targets --all-features -- -D warnings`** — Lints your
   code. **Warnings are treated as errors.**
3. **`just test`** — Runs the entire test matrix.

If `just pre-commit` passes, your code is 99% ready for CI.

> **WARNING**
>
> Never fight the formatter. If `cargo fmt` changes your code in a way you
> don't like, adjust `rustfmt.toml` (**with a maintainer's approval**) or
> refactor your code to fit the style (made automatically by Just).

---

## 5. Committing with Conventional Commits

Resync uses an automated release pipeline that generates changelogs and bumps
versions based entirely on your commit messages. This means **commit message
formatting is strictly enforced**.

We follow the [Conventional Commits](https://www.conventionalcommits.org/)
specification:

````text
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
````

### Common Types

- **`feat`**: A new feature (e.g., a new primitive like `TimedGate`).
- **`fix`**: A bug fix.
- **`docs`**: Documentation-only changes.
- **`style`**: Formatting, missing semi-colons, etc.
- **`refactor`**: Code change that neither fixes a bug nor adds a feature.
- **`perf`**: A code change that improves performance.
- **`test`**: Adding missing tests or correcting existing tests.
- **`chore`**: Changes to the build process, CI, or auxiliary tools.

### Breaking Changes

If your PR introduces a breaking API change, you must add a `!` after the
type/scope and include `BREAKING CHANGE:` in the footer:

```text
feat(api)!: change Mutex::lock to return AcquireError

BREAKING CHANGE: Mutex::lock now returns a Result instead of panicking on
poisoning.
```

### The `./commit` Wrapper

To make this foolproof, use the provided `./commit` wrapper script instead of
`git commit`:

````bash
./commit -m "feat(gate): add TimedGate primitive"
````

This script automatically runs `just pre-commit`, stages all changes
(`git add -A`), and then commits with your message. If any check fails, the
commit is aborted.

> **ATTENTION**
>
> If `./commit` aborted your commit, **do not** perform it manually. This
> script made specially to make development workflow foolproof.

---

## 6. Opening a Pull Request

Once your branch is pushed to your fork, open a PR against `main`.

The PR template will automatically populate with a checklist:

- [ ] **Conventional Commits:** My commit messages follow the specification.
- [ ] **Standards:** I used `./commit` to perform all commits.
- [ ] **Green CI:** I have checked the whole CI pipeline to be successful.
- [ ] **Testing:** I have added tests that prove my fix/feat/refactor is
correct.
- [ ] **Benching:** I have wrote new benchmarks (if applicable).
- [ ] **Documentation:** I have updated inline `///` doc-comments and books.


### What Maintainers Look For

1. **Miri is clean.** If your PR touches `unsafe` code or atomic operations,
   maintainers will run `cargo miri test`. Ensure your code is free of data
   races and UB before submitting.
2. **No `#![no_std]` regressions.** If you added a feature that requires `std`,
   it must be properly gated behind `#[cfg(feature = "std")]`.
3. **Clear doc-tests.** Every public API addition must have a runnable example
   in its doc-comment.

---

## Summary

You don't need to understand how `futex_wait` is implemented in `lock/linux.rs`
to fix a bug in `Gate`. You just need to:

1. Find an issue.
2. Write your code in the right `batteries/` subfolder.
3. Run `just pre-commit`.
4. Use `./commit` with a Conventional Commit message.
5. Open a PR.

That's it! The CI pipelines and the trait system will handle the rest, keeping
the codebase safe and consistent.

Ready to build something new? Let's explore the
[Feature Lifecycle](./feature.md) to see how a new primitive goes from an idea
to a stable, published API.
