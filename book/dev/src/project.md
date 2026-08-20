# Resync: Dev's POV

Welcome to the bird's-eye view of the Resync repository. Before you start
editing code, it is worth taking a step back and understanding **what lives
where** and **why**. This chapter will walk you through the repository layout,
the source code architecture, and the CI/CD pipelines that keep the project
healthy.

If you are a first-time contributor, consider this your orientation day.

---

## 1. Repository Structure

The root of the repository is split into several well-defined areas. Each one
has a distinct responsibility. Let's break down into meaningful groups.

### Meta files

These files define the project's identity and development environment:

- **`Cargo.toml`** — The manifest of the crate. It declares the package name
- (`resync`), version, dual MIT/Apache-2.0 license, feature flags, and
dependencies like `libc`, `reta`, and `windows-sys`.
- **`rust-toolchain.toml`** — Pins the project development to a `nightly`
toolchain.
- **`rustfmt.toml`** — Custom formatting rules (`brace_style = "AlwaysNextLine"`,
`max_width = 80`, etc.). Running `cargo fmt` will apply these strictly. **Do not
fight the formatter.**
- **`.editorconfig`** — Ensures consistent line endings (`LF`), indentation (4
spaces), and UTF-8 encoding across all editors.
- **`Needsfile`** — A simple list of external tools the project expects to be
installed: `rustup` and `python3`.
- **`commit`** — A tiny shell wrapper that runs `just pre-commit` (format +
clippy + tests) before actually committing. Use it like:
`./commit -m "feat: add ticket lock"`.
- **`justfile`** — Defines the standard development commands. More on this below.

### GitHub Configuration (`.github/`)

This folder hosts everything GitHub needs to manage community interactions and automation:

- **`CODE_OF_CONDUCT.md`** — The Contributor Covenant 3.0. Please read it.
- **`CONTRIBUTING.md`** — Explains the
[Conventional Commits](https://www.conventionalcommits.org/) format required for
all commit messages. Without the right prefix (`feat:`, `fix:`, `chore:`, etc.),
the automated release pipeline will fail.
- **`SECURITY.md`** — Instructions for privately reporting vulnerabilities (data
races, UB, soundness holes). **Never open a public issue for a security bug.**
- **`ISSUE_TEMPLATE/`** — Structured templates for bug reports, feature requests,
and questions. They help gather all the required context upfront.
- **`PULL_REQUEST_TEMPLATE.md`** — A checklist that reminds you to run
`cargo fmt`, `clippy`, update docs, and use conventional commits.
- **`dependabot.yml`** — Keeps GitHub Actions dependencies up-to-date
automatically.
- **`workflows/`** — The GitHub Actions workflows. Covered in detail in
[Section 3](#3-cicd-pipelines).

### The Two Books (`book/`)

Resync ships with **two** mdBook documentation sites, deployed separately:

- **`book/lib/`** — The **Library Book**. Written for *users* of the crate.
Explains the LEGO philosophy, core traits, built-in batteries, design decisions,
and usage examples. This is the public-facing guide linked from `docs.rs`.
- **`book/dev/`** — The **Developer Guide**. Written for *contributors* and
*maintainers*. This is the book you are reading right now. It covers the project
layout, contribution workflow, feature lifecycle, and regression testing.

Both books share the same structure (`SUMMARY.md`, chapters) but target very
different audiences.

### Tests and Benchmarks

- **`tests/`** — Integration tests that exercise the public API from an external
crate's perspective. These catch "leakage" issues that unit tests inside `src/`
would miss.
- **`benchmarks/`** — [Criterion](https://github.com/bheisler/criterion.rs)
benchmarks comparing Resync primitives against `std::sync::Mutex`, `parking_lot`,
and others. Run them with `cargo bench`.

### Python Scripts (`scripts/`)

Despite being a Rust crate, Resync uses Python for release automation:

- **`release.py`** — Checks if the version in `Cargo.toml` is new, publishes to
crates.io, and creates a GitHub tag.
- **`chlog.py`** — Parses conventional commits since the last tag and generates
release notes.
- **`test.py`** — Orchestrates the test suite across feature combinations.
- **`lib.py`** — A shared utility module exposing `Cargo.manifest()`,
`subprocess`, and `tomllib`.

---

## 2. Source Code Layout

Now let's look inside `src/`. The layout is not accidental — it reflects
Resync's core philosophy of **separating *acquisition* from *waiting***.

```text
src/
+-- lib.rs          # Crate root (untouchable)
+-- result.rs       # Core result types (untouchable)
+-- util.rs         # Internal helpers (untouchable)
+-- api/            # Behavior-driven public traits
|   +-- mutex.rs
|   +-- sharex.rs
|   \-- mod.rs
+-- batteries/      # Ready-to-use implementations
|   +-- lock/       #   \- LockPolicy backends
|   +-- retry/      #   \- RetryPolicy backends
|   +-- poison/     #   \- PoisonPolicy backends
|   \-- primitives/ #   \- High-level structs (Mutex, Gate, Semaphore...)
\-- traits/         # Foundational trait definitions
    +-- lock_policy.rs
    +-- sharing_policy.rs
    +-- retry_policy.rs
    +-- new_locked.rs
    \-- poison_policy.rs
```

### Root files — the untouchable zone

Three files live directly in `src/`:

- **`lib.rs`** — The crate root. It re-exports everything, declares features
(`#![no_std]`, `#![cfg_attr(nightly, feature(const_trait_impl))]`), and wires
the modules together.
- **`result.rs`** — Defines the universal result types used throughout the crate:
`LockResult<M, E>`, `RetryResult<E>`, `AcquireError`, `TryLockError`, and
`PoisonError`.
- **`util.rs`** — Tiny internal helpers (like `random_hex_16()` for generating
unique temp file names in the `Fs` lock backend).

> **ATTENTION**
>
> These three files define the **public surface** and the **error taxonomy**
> of the crate. Changing them is a breaking change for downstream users and may
> invalidate every existing test. Edit them only with a very good reason and a
> maintainer's approval.

### `traits/` — the foundational layer

This module is the **bedrock** of Resync. Every synchronization primitive in the
crate is built on top of these traits:

| Trait | Purpose |
| :--- | :--- |
| **`LockPolicy`** | The atomic acquisition strategy. Defines `try_lock`, `free`, and `wake_all`. Unsafe to implement — you are responsible for memory ordering. |
| **`SharingPolicy`** | Extends `LockPolicy` with `try_share` / `free_share` for read-write locks. |
| **`RetryPolicy`** | The waiting strategy. Called repeatedly when `try_lock` returns `LockStatus::Fail`. |
| **`NewLocked`** | Optional extension for locks that can be created in the *already-acquired* state. Essential for `Gate` to prevent TOCTOU races. |
| **`PoisonPolicy`** | Defines how to detect panics and mark a lock as poisoned. |

> **NOTE**
>
> The list of traits may change, for further information take look at
> `src/traits` directory.

When you contribute a new primitive, you will almost always be composing these existing traits rather than inventing new ones.

### `batteries/` — ready-to-use implementations

The `batteries` module is where the LEGO blocks actually live. It is subdivided into four logical areas:

- **`batteries/lock/`** — Concrete `LockPolicy` and `SharingPolicy` implementations;
- **`batteries/retry/`** — Concrete `RetryPolicy` implementations;
- **`batteries/poison/`** — `PoisonPolicy` implementations;
- **`batteries/primitives/`** — The user-facing high-level structs.

When you add a new primitive, it will almost certainly go into
`batteries/primitives/`.

> **NOTE**
>
> The list of traits may change, for further information take look at
> `src/batteries` directory.

### `api/` — behavior-driven contracts

The `api` module is Resync's answer to the `lock_api` crate (sort of). It
defines **behavior-driven traits** that abstract over any lock implementation:

- **`api::Mutex<'a, T, TryR, R>`** — Any struct that provides `try_lock()` and
`lock()` methods.
- **`api::Sharex<'a, T, TryR, R>`** — Any struct that provides `try_read()` and
`read()` methods.

Unlike `lock_api::RawMutex` (which assumes infallible acquisition), Resync's API
traits accept arbitrary result types. This allows generic code to work with
`Mutex`, `Sharex`, or even third-party locks, **while preserving poisoning and
timeout semantics**.

The `api` module also re-exports everything from `traits/` so that downstream
code can write `use resync::api::{LockPolicy, RetryPolicy, Mutex};`.

---

## 3. CI/CD Pipelines

Resync's automation lives in `.github/workflows/`. Don't be intimidated — the
pipelines are your friends. They catch bugs before they reach `main` and handle
releases automatically.

### The main workflow: `ci.yml`

The `ci.yml` workflow runs on every push and pull request. It consists of five
jobs executed in a specific order:

````yaml
jobs:
  check:     # 1. Format & Clippy (must pass first)
  build:     # 2. cargo build --all-features
  miri:      # 3. UB and data-race detection
  test:      # 4. Cross-platform test matrix
  release:   # 5. Publish to crates.io (only on main)
````

Let's walk through each one.

#### `check` — Format & Clippy

This job runs first. If it fails, nothing else runs.

- `cargo fmt --all -- --check` — Verifies that the codebase is formatted
according to `rustfmt.toml`.
- `cargo clippy --all-targets --all-features -- -D warnings` — Runs Clippy with
all features enabled and **treats every warning as an error**.

> **TIP**
>
> Running `just check` locally before pushing will save you a CI roundtrip.

#### `build` — Compile everything

A simple `cargo build --all-features` to ensure the crate compiles with every
feature flag enabled simultaneously. This catches feature-interaction bugs that
would otherwise slip through.

#### `miri` — Catching the invisible

This is the **most important** job for a concurrency library.
[Miri](https://github.com/rust-lang/miri) is an interpreter for Rust's Mid-level
Intermediate Representation that detects:

- **Data races**
- **Undefined behavior** (dangling pointers, unaligned reads, invalid enum
discriminants)
- **Memory leaks** in unsafe code
- **Stack borrows** violations

Since Resync relies heavily on `unsafe` atomic operations, Miri is our best line
of defense against soundness holes. If Miri fails, the PR is blocked — no
exceptions.

#### `test` — The cross-platform matrix

This job runs the test suite across a **3D matrix**:

- **Operating systems:** `ubuntu-latest`, `macos-latest`, `windows-latest`
- **Rust channels:** `stable`, `beta`, `nightly`
- **Feature combinations:** `""`, `"std"`, `"dev"`, `"std,dev"`

The matrix ensures that a feature-gated piece of code (like `Condvar`, which
requires `std`) is tested in isolation and in combination. The Python script
`scripts/test.py` orchestrates the feature toggles.

> **WARNING**
>
> If you add a new feature-gated item, remember to update the matrix logic in
> `scripts/test.py`. A missing combination means a missing test.

#### `release` — Automated publishing

This is the CD part, and it only runs **on the `main` branch** after all other
jobs pass.

The job executes `python scripts/release.py`, which does the following:

1. Reads the version from `Cargo.toml`.
2. Queries the crates.io index to check if this version is already published.
3. **If the version is new:** publishes the crate via `cargo publish`.
4. Creates a Git tag `vX.Y.Z` via the GitHub API.

This means that **bumping the version in `Cargo.toml` and merging to `main` is
the only action required to publish a release.** No manual `cargo publish`, no
manual tagging.

> **Security**
>
> The `CARGO_REGISTRY_TOKEN` and `GITHUB_TOKEN` are stored as repository secrets.
Contributors cannot access them — only the automation can.

### Documentation deployment

Two additional workflows deploy the mdBooks to GitHub Pages:

- **`pages-dev.yml`** — Builds `book/dev/` and deploys it to
`https://vi-is-ramen.github.io/resync/dev/`.
- **`pages-lib.yml`** — Builds `book/lib/` and deploys it to
`https://vi-is-ramen.github.io/resync/lib/`.

These run independently of `ci.yml` and can be triggered manually via
`workflow_dispatch` if you need to force a docs rebuild.

---

## 4. The Development Loop

With everything in place, the typical development cycle looks like this:

````bash
# 1. Make your changes
$ nvim src/batteries/primitives/my_new_gate.rs

# 3. Commit using the wrapper (runs pre-commit by self)
$ ./commit -m "feat(gate): add TimedGate primitive"

# 4. Push and let CI do the heavy lifting
$ git push
````

If everything is green on CI, open a PR. The PR template will remind you of the
checklist. Once merged to `main`, the release pipeline takes over if there is a
version bump.

That's the big picture. In the next chapters we will zoom in on specific
processes: how to add a new feature, how to report a security issue, and how to
write regression tests that actually catch bugs.

---

Ready to learn how to contribute without getting lost in the codebase? Continue
to [No Need to Deep Dive!](./contribution.md).
