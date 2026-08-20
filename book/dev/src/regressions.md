# Attention! API Breaks? Be preventive!

Breaking changes are the silent killers of open-source projects. A single
renamed method, a changed generic bound, or a moved module can cascade through
the ecosystem, breaking downstream crates and stalling production deployments.

Resync is a foundational concurrency library. Downstream users rely on its API
stability to build kernels, embedded systems, and high-throughput servers. A
careless breaking change doesn't just annoy users — it forces them to fork the
crate or abandon it entirely.

This chapter describes how to evolve Resync's API safely, without breaking the
code that depends on it.

---

## 1. The Cost of Breaking Changes

Before discussing prevention, it is worth understanding what a breaking change
actually costs:

- **Downstream breakage.** Every crate that depends on Resync must be updated.
  For a foundational library, this can mean dozens or hundreds of dependent
  crates.
- **Lost trust.** Users who experience breakage are less likely to upgrade in
  the future, fragmenting the user base across old versions.
- **Maintenance burden.** Maintainers may be forced to support multiple major
  versions simultaneously, doubling the surface area for bugs and security
  patches.
- **Ecosystem friction.** Breaking changes force the entire ecosystem to
  coordinate upgrades, slowing adoption of new features.

> **ATTENTION**
>
> In Rust, breaking changes are not just inconvenient — they cause **compile
> errors** that block CI pipelines and prevent releases. A single breaking
> change in Resync can cascade through the entire dependency graph.

---

## 2. Characterization Tests

The first line of defense against API regressions is **characterization tests**.

### What Are Characterization Tests?

Characterization tests are tests that **document the current behavior** of the
system, regardless of whether that behavior is "correct." They capture the
*observable* API surface — method signatures, return types, error variants,
trait bounds, and panic conditions.

Unlike unit tests (which verify *correctness*), characterization tests verify
*compatibility*. They answer the question: **"Does this code still compile and
behave exactly as it did before my change?"**

### Why They Matter for Resync

Resync's API is heavily generic:

````rust
pub struct Mutex<T, L = DefaultLock, R = retry::Yield, P = poison::DefaultPoison>
where
    L: LockPolicy,
    R: RetryPolicy,
    P: PoisonPolicy,
{ /* ... */ }
````

A change as small as adding a new trait bound to `L` (e.g., `L: Send`) can
silently break downstream code that uses a non-`Send` lock policy.
Characterization tests catch these invisible breaks.

### Writing Characterization Tests

Characterization tests live in `tests/` and are structured to exercise the
**public API surface** exactly as a downstream user would. They should:

1. **Use the public re-exports** (`use resync::Mutex;`), not internal paths.
2. **Avoid `cfg(test)` shortcuts** — test the exact types users will use.
3. **Assert on types, not just values** — use compile-time assertions to verify
   trait implementations.

#### Example: A Characterization Test for `Mutex`

````rust
//! tests/api-battery-mutex.rs
//!
//! Characterization test for the public `Mutex` API.
//! This test ensures that the generic bounds, default parameters,
//! and method signatures remain stable across releases.

use resync::{Mutex, lock::Atomic, retry::Busy, poison::NoPoison};
use resync::traits::{LockPolicy, RetryPolicy, PoisonPolicy};

// Compile-time assertion: verify default type parameters
fn _assert_defaults() {
    // If this compiles, the defaults are stable
    let _: Mutex<i32> = Mutex::new(42);
    let _: Mutex<i32, Atomic> = Mutex::new(42);
    let _: Mutex<i32, Atomic, Busy> = Mutex::new(42);
}

// Compile-time assertion: verify trait bounds
fn _assert_bounds<T, L, R, P>()
where
    T: Send,
    L: LockPolicy + Default + Send + Sync,
    R: RetryPolicy + Default + Send,
    P: PoisonPolicy + Default,
{
    // If this compiles, the trait bounds are stable
    let _: Mutex<T, L, R, P> = Mutex::new(unsafe { core::mem::zeroed() });
}

// Runtime assertion: verify method signatures
#[test]
fn test_mutex_api_surface() {
    let m = Mutex::<i32, Atomic, Busy, NoPoison>::new(0);

    // try_lock returns TryLockError variants
    let _ = m.try_lock();

    // lock returns AcquireError variants
    let _ = m.lock();

    // exchange and take are available
    let guard = m.lock().unwrap();
    drop(guard);
}
````

### The Golden Rule

> **ATTENTION**
>
> Every public type, trait, and method must have at least one characterization
> test. If you add a new public API, add a characterization test for it. If
> you change an existing API, the characterization test will fail, alerting you
> to the breakage.

---

## 3. Semantic Versioning and Conventional Commits

Resync strictly follows [Semantic Versioning](https://semver.org/) and
[Conventional Commits](https://www.conventionalcommits.org/). These are not
bureaucratic rules — they are the contract between Resync and its users.

### The SemVer Contract

| Change Type | Version Bump | Example |
| :--- | :--- | :--- |
| **Breaking change** | Major (`X.0.0`) | Removing a method, changing a generic bound |
| **New feature** | Minor (`0.X.0`) | Adding a new method, a new battery |
| **Bug fix** | Patch (`0.0.X`) | Fixing a deadlock, correcting a doc-comment |

### How Conventional Commits Enforce SemVer

The `scripts/chlog.py` script parses commit messages and automatically generates
release notes grouped by type.

If you introduce a breaking change but forget to add the `!` marker:

````text
# BAD: Breaking change without marker
feat(mutex): change lock() to return Result<(), Error>

# GOOD: Breaking change with marker
feat(mutex)!: change lock() to return Result<(), Error>

BREAKING CHANGE: Mutex::lock() now returns a Result instead of panicking.
````

...maintainer will reject your contribution immediately. Thanks Resync's release
process which requires human's aprovement before PR.

> **WARNING**
>
> A SemVer violation is a form of supply-chain attack. Downstream users who
> run `cargo update` expect minor and patch versions to be safe. Violating this
> contract is a breach of trust.

---

## 4. Automated Testing as a Safety Net

Characterization tests catch *your* mistakes, but they do not catch every
accidental break. Resync relies on its test infrastructure to catch regressions
before they ship.

### The Cross-Feature Test Matrix

The `scripts/test.py` script (invoked by `just test`) runs the test suite
against every feature combination:

```bash
cargo test --no-default-features --all-targets
cargo test --no-default-features dev --all-targets
cargo test --no-default-features std --all-targets
cargo test --no-default-features std,dev --all-targets
...
cargo test --doc --all-features
```

This ensures that a change gated behind `#[cfg(feature = "std")]` does not
accidentally break the `no_std` build, and that a change in a rarely-used
feature combination does not slip through.

### Miri for Soundness Regressions

Miri is not just for finding new bugs — it is also a regression test for
**soundness**. If a PR introduces a data race or undefined behavior that Miri
catches, the CI pipeline blocks the merge.

Any change to `unsafe` code, atomic operations, or memory ordering must pass
`cargo miri test` before merging.

### Integration Tests with Downstream Patterns

For critical API changes, maintainers manually test Resync against known
downstream usage patterns (e.g., a custom kernel or embedded framework) to
ensure compatibility. This is especially important for changes to core traits
like `LockPolicy` or `RetryPolicy`.

---

## 5. The Art of Deprecation

When you *must* break an API, do it gracefully. The standard approach is the
**Deprecation Bridge**:

### Step 1: Add the New API

Introduce the new, improved API alongside the old one. Do not remove the old
API yet.

````rust
// Old API
pub fn lock(&self) -> ExGuard<'_, T, L, P> { /* ... */ }

// New API
pub fn lock_v2(&self) -> Result<ExGuard<'_, T, L, P>, AcquireError<...>> { /* ... */ }
````

### Step 2: Deprecate the Old API

Mark the old API with `#[deprecated]` and point users to the new one.

```rust
#[deprecated(
    since = "0.11.0",
    note = "Use `lock_v2()` instead. This method will be removed in 0.12.0."
)]
pub fn lock(&self) -> ExGuard<'_, T, L, P> { /* ... */ }
```

### Step 3: Wait

Leave the deprecated API in place for **at least one minor version** (ideally
two). This gives users time to migrate.

### Step 4: Remove the Old API

In the next major version bump, remove the deprecated API entirely.

```rust
// Old API is gone
// pub fn lock(&self) -> ExGuard<'_, T, L, P> { /* ... */ } // REMOVED

// New API is now the standard
pub fn lock(&self) -> Result<ExGuard<'_, T, L, P>, AcquireError<...>> { /* ... */ }
```

> **WARNING**
>
> Never skip the deprecation step. Removing an API without warning is a
> hostile act that will generate angry issues and lost users.

---

## 6. Feature Flags as an Evolution Tool

Sometimes, you need to introduce a breaking change that cannot be done
gradually. In these cases, **feature flags** provide a safe migration path.

### The Pattern

1. Add a new feature flag (e.g., `future-v2_api`).
2. Gate the new API behind the feature flag.
3. Keep the old API as the default.
4. In a future major version, make the new API the default and deprecate the
   flag.

#### Example: Evolving `RetryPolicy`

Suppose we want to change `RetryPolicy::retry` to take `&self` instead of
consuming `self`. This is a breaking change for implementors.

```rust
// In Cargo.toml
[features]
v2_retry = []

// In src/traits/retry_policy.rs
pub trait RetryPolicy {
    type Error: core::error::Error;

    #[cfg(not(all(feature = "future-retry", feature = "dev")))]
    fn retry(self, current_iteration: usize) -> RetryResult<Self::Error>;

    #[cfg(any(feature = "future-retry", feature = "dev"))]
    fn retry(&self, current_iteration: usize) -> RetryResult<Self::Error>;
}
```

Downstream users can opt into the new API by adding `features = ["v2_retry"]`
to their `Cargo.toml`. Once the ecosystem has migrated, we remove the old API
in the next major version.

> **TIP**
>
> If your name of feature was already used before (e. g., 1.x versions had
> `future-retry` feature for some range of versions), it's better to change
> your feature name. If you won't, discuss with maintainer. They will respect
> your wish and provide compromises.

---

## 7. The Checklist for API Changes

Before merging any PR that touches the public API, verify the following:

| Check | Method |
| :--- | :--- |
| **Characterization tests pass** | `cargo test --test api-stability` |
| **Cross-feature builds succeed** | `just test` (runs all feature combos) |
| **Miri is clean** | `cargo miri test` |
| **Deprecation warnings added** | `#[deprecated(since = "...", note = "...")]` |
| **Conventional commit marked** | `feat(api)!: ...` with `BREAKING CHANGE` footer |
| **Library Book updated** | `book/lib/src/` chapters reflect new API |

> **ATTENTION**
>
> Do not bump the version in `Cargo.toml` yourself. Version bumping is the
> maintainer's responsibility and is scheduled based on release readiness.
> Bumping the version without approval will lead to PR rejection.

---

## Summary

API stability is not a constraint — it is a **feature**. By treating the public
API as a contract and using characterization tests, the cross-feature test
matrix, and graceful deprecation, Resync can evolve without betraying its users.

Remember the hierarchy of API changes:

1. **Additive changes** (new methods, new types) — Minor version, no warning.
2. **Deprecations** (old API marked for removal) — Minor version, with warning.
3. **Breaking changes** (removal, bound changes) — Major version, with
   `BREAKING CHANGE` footer.

Follow this hierarchy, and Resync will remain a trusted foundation for Rust's
concurrency ecosystem for years to come.

---

You have now completed the core developer guide. To wrap up your journey,
continue to the [Summary](./conclusion.md) for final thoughts and next steps.
