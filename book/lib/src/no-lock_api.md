# Why Not `lock_api`?

> **ATTENTION**
>
> This chapter explains a deliberate architectural decision, not an oversight.
> Resync **cannot** and **will not** implement `lock_api`'s traits unless the
> Rust language and the `lock_api` crate itself undergo fundamental changes.
> If you are looking for a quick "just add an adapter" solution, this chapter
> will explain why that approach is a trap.

## The Ecosystem Standard

The [`lock_api`](https://docs.rs/lock_api) crate is the de facto standard for
abstracting over mutex implementations in the Rust ecosystem. Crates like
`parking_lot`, `spin`, and dozens of others implement its `RawMutex`,
`RawRwLock`, and related traits. If you write a generic data structure that
accepts "any mutex," you almost certainly depend on `lock_api`.

This makes Resync's incompatibility conspicuous. Users arriving from
`parking_lot` or `spin` naturally expect to plug a Resync lock into any
`lock_api`-based generic container. **They cannot.** This chapter explains
exactly why.

## The Core Conflict: Infallibility vs. Granular Errors

The `lock_api::RawMutex` trait defines acquisition as an **infallible**
operation:

```rust
// lock_api's model (simplified)
pub unsafe trait RawMutex {
    const INIT: Self;
    fn lock(&self);       // returns ()
    fn try_lock(&self) -> bool;
    fn unlock(&self);
}
```

There is no `Result`. There is no error type. The contract is: *call `lock()`,
and you will eventually hold the lock, or your program will hang forever.*

Resync's entire philosophy is built on the opposite premise:

```rust
// Resync's model (simplified)
pub unsafe trait LockPolicy: Sync {
    type Error: core::error::Error;
    type Meta;

    unsafe fn try_lock(
        &self,
        current_iteration: usize,
    ) -> LockResult<Self::Meta, Self::Error>;

    unsafe fn free(&self, meta: &Self::Meta);
}
```

Every acquisition can fail with a **typed, inspectable error**. The
`LockResult<M, E>` type distinguishes between:

| Variant | Meaning |
| :--- | :--- |
| `Ok(LockStatus::Done(meta))` | Lock acquired successfully. |
| `Ok(LockStatus::Fail)` | Contention — retry via `RetryPolicy`. |
| `Err(e)` | Fatal, unrecoverable failure in the lock backend. |

At the higher level, `AcquireError` further separates **Poisoned**, **Lock**,
and **Retry** variants. This taxonomy is not a luxury — it is the mechanism
that allows:

- A `RetryPolicy` to enforce **timeouts** without panicking.
- An `Os` lock backend to propagate **kernel resource exhaustion**.
- A `PoisonPolicy` to signal **data inconsistency** after a thread panic.

### What an Adapter Would Require

To implement `lock_api::RawMutex` for a Resync lock, you would need to write
something like this:

```rust
// HYPOTHETICAL ADAPTER — DO NOT USE
unsafe impl lock_api::RawMutex for ResyncMutexAdapter {
    fn lock(&self) {
        // What do we do with AcquireError::Retry(timeout)?
        // What do we do with AcquireError::Lock(os_error)?
        // What do we do with AcquireError::Poisoned(guard)?
        self.inner.lock().unwrap() // ← panics on ALL of the above
    }
}
```

Every recoverable error becomes a **panic**. Every timeout becomes an
**abort**. Every poisoned lock becomes **undefined behavior** from the
caller's perspective, because `lock_api` provides no mechanism to inspect
or recover from these states.

This is not a limitation of our implementation. It is a limitation of the
`lock_api` trait's **type signature**. You cannot return information that
the return type does not have room for.

> **WARNING**
>
> An adapter that calls `.unwrap()` internally is not "compatibility."
> It is a **soundness hole** dressed up as an API bridge. We refuse to
> ship it.

## Lock Poisoning Has No Home in `lock_api`

Resync's `PoisonPolicy` trait allows a lock to detect thread panics and mark
itself as poisoned:

```rust
pub trait PoisonPolicy {
    fn is_poisoned(&self) -> bool;
    fn on_drop(&self);
    unsafe fn clear_poison(&self);
}
```

The `lock_api` crate has **no concept of poisoning**. Its `RawMutex` trait
cannot express the state "this lock was held by a thread that panicked, and
the protected data may be inconsistent." There is no associated error type,
no guard introspection, no recovery path.

If Resync implemented `lock_api::RawMutex`, poisoning would have to be
**silently discarded**. A thread panicking inside a critical section would
leave the lock in a state indistinguishable from a clean release. Subsequent
threads would read potentially corrupted data with no warning.

This violates Resync's core safety guarantee. We will not trade data
integrity for ecosystem convenience.

## The `const_trait_impl` Problem

Resync leverages nightly Rust features — specifically `const_trait_impl` and
`const_default` — to enable zero-cost `const` initialization of locks in
`static` variables:

```rust
#![cfg_attr(nightly, feature(const_trait_impl))]
```

The `lock_api` crate's traits are not `const`-compatible. Even if the
infallibility and poisoning problems were somehow solved, implementing
`lock_api::RawMutex` would prevent Resync's lock policies from being used in
`const` contexts on nightly, regressing a core feature of the crate.

The relevant note from the crate root:

> Until Rust's `const_traits` and `const_trait_impl` are stabilized and
> `lock_api` adopts a compatible model, we are not compatible.

This is not a permanent refusal. It is a **conditional gate**. If the Rust
language stabilizes const traits and `lock_api` restructures its API to
accommodate them, the conversation can be reopened.

## What Resync Offers Instead: `resync::api`

Rather than forcing compatibility with an incompatible trait, Resync provides
its own abstraction layer in the `resync::api` module. These traits solve the
same problem as `lock_api` — *writing generic code over any mutex* — but
preserve Resync's error taxonomy:

```rust
pub trait Mutex<'a, T, G, TryE, E>
where
    Self: Sync,
    G: GuardMut<T>,
    TryE: Display,
    E: Display,
{
    fn try_lock(&'a self) -> Result<G, TryE>;
    fn lock(&'a self) -> Result<G, E>;
}
```

Key differences from `lock_api::RawMutex`:

| Aspect | `lock_api::RawMutex` | `resync::api::Mutex` |
| :--- | :--- | :--- |
| **Return type** | `()` (infallible) | `Result<G, E>` (fallible) |
| **Error granularity** | None | `AcquireError` / `TryLockError` |
| **Poisoning** | Not representable | First-class via `PoisonError` |
| **Timeouts** | Not representable | `AcquireError::Retry` |
| **Dyn-compatibility** | Yes | Yes (by design) |
| **Guard type** | Fixed (`MutexGuard`) | Generic parameter `G` |

Because the error types are **generic parameters** (`TryE`, `E`) rather than
hardcoded types, `resync::api::Mutex` can be implemented for:

- Resync's own `Mutex` and `Sharex` (with full error taxonomy).
- `std::sync::Mutex` (with `PoisonError` and `TryLockError`).
- Any third-party lock that returns a `Result`.

This makes `resync::api` strictly **more expressive** than `lock_api`, not
less. You lose nothing by using it, and you gain the ability to handle
timeouts, poisoning, and OS errors in generic code.

## Practical Implications for Users

### "I have a generic container that uses `lock_api`. Can I use Resync?"

**Not directly.** You have three options:

1. **Migrate the container to `resync::api`.** If you control the generic
   code, switch from `lock_api::RawMutex` to `resync::api::Mutex`. This is
   the recommended path.

2. **Use Resync's concrete types directly.** If you don't need the container
   to be generic over the lock type, use `resync::Mutex<T>` directly.

3. **Use `parking_lot` or `spin` for that specific container.** If migration
   is impractical, use a `lock_api`-compatible lock for the container and
   Resync for everything else. The two can coexist in the same binary.

### "Will Resync ever support `lock_api`?"

The door is not permanently closed, but three conditions must be met
**simultaneously**:

1. Rust stabilizes `const_trait_impl` (or an equivalent mechanism).
2. `lock_api` restructures its traits to support fallible acquisition
   (returning `Result` instead of `()`).
3. `lock_api` adds a mechanism to represent poisoning or defers it to the
   guard type.

Until all three are true, implementing `lock_api` would require Resync to
**delete features** that define its identity. That is a trade we are not
willing to make.

## Summary

The incompatibility between Resync and `lock_api` is not a bug, not an
oversight, and not a missing feature. It is a **fundamental architectural
divergence** between two philosophies:

- `lock_api` says: *"Locking always succeeds. If it can't, your program
  is broken."*
- Resync says: *"Locking can fail in many distinct ways, and your program
  should be able to react to each one."*

These two statements cannot be reconciled within the same type signature.
We chose the second philosophy because it enables timeouts, `no_std`
environments, kernel development, and graceful degradation — scenarios where
a panic is not an acceptable response to a contended lock.

If the ecosystem evolves to embrace fallible, granular lock acquisition,
Resync will be ready. Until then, `resync::api` is our answer.
