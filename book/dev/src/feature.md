# Feature Lifecycle

So you have a brilliant idea for a new synchronization primitive, a novel
lock backend, or a clever retry strategy. How does that idea become a part of
Resync? This chapter walks you through the complete lifecycle of a feature,
from the first spark of inspiration to the moment it lands in `main`.

The process is designed to be **collaborative, safe, and efficient**. Skipping
steps often leads to wasted effort or rejected PRs, so please follow this
workflow carefully.

---

## Step 1: Open an Issue

Every feature starts with an issue. **Never start coding before opening an
issue.** This is the single most important rule for contributing features.

Navigate to the [GitHub Issues](https://github.com/vi-is-ramen/resync/issues)
page and select the **Feature Request** template. This template will
automatically populate with the following sections:

- **Problem**: What pain point are you solving? (e.g., "I need a gate that
  automatically opens after a timeout, but Resync only has manual gates.")
- **Solution**: Describe your proposed API. How would it fit into the existing
  `LockPolicy`, `RetryPolicy`, or `SharingPolicy` traits?
- **Alternatives**: What other approaches did you consider? Why did you reject
  them?
- **Context**: Link to academic papers, benchmarks, or other crates that
  inspired you.

### A Good Issue Example

> **Title**: `feat: add TimedGate primitive`
>
> **Problem**: When implementing thread pool initialization, I often need a gate
> that automatically opens after a timeout if the main thread hasn't explicitly
> opened it. Currently, I have to spawn a watchdog thread that calls `gate.open()`
> after a delay, which is error-prone and adds overhead.
>
> **Solution**: Add a `TimedGate<L, R>` primitive that wraps a `Gate<L, R>` and
> uses a `RetryPolicy` with timeout semantics to automatically transition from
> closed to open after a specified duration.
>
> **Alternatives**: I considered adding a `open_after(Duration)` method to the
> existing `Gate`, but that would require spawning a background thread or using
> `std::thread::sleep`, which doesn't fit Resync's `no_std` philosophy.

---

## Step 2: Discussion

Once the issue is open, the community and maintainers will engage in a
discussion. This is not a rubber-stamp process — it's a design review.

### What Maintainers Look For

1. **Does it fit the LEGO philosophy?** Can it be composed from existing
   `LockPolicy` and `RetryPolicy` traits, or does it require hardcoding behavior?
2. **Is it `no_std`-compatible?** If it requires `std`, is that justified?
3. **Does it introduce breaking changes?** Will it change the public API of
   existing primitives?
4. **Is it testable?** Can we write deterministic tests for it using the `Fake`
   policy?

### Be Patient and Responsive

The discussion might take days or even weeks. Maintainers may ask probing
questions like:

- "What happens if the `RetryPolicy` aborts during `Gate::close()`?"
- "How does this interact with `Shield` for writer fairness?"
- "Can you provide a benchmark comparing this to `std::sync::Barrier`?"

**Answer every question.** If you don't know the answer, say so and propose a
way to find out. Silence is often interpreted as abandonment.

---

## Step 3: Get the Green Light

After the discussion converges on a design, a maintainer will explicitly
**approve** the feature. Look for comments like:

> "This looks good. Let's proceed with the implementation."
>
> "LGTM. Please open a PR when ready."

**Do not start coding until you see this approval.** If you open a PR without
prior discussion, it may be closed with a request to open an issue first.

> **WARNING**
>
> Features implemented without prior approval are almost always rejected. The
> maintainers' time is valuable — please respect the design review process.

---

## Step 4: Create a Branch

Once approved, create a new branch for your work. The branch name should follow
the pattern `feature-name-<NUMBER>`:

````bash
git checkout -b timed-gate-12345
````

Where `12345` is the issue number from GitHub. This naming convention makes it
trivial to trace any branch back to its originating discussion.

### Why Not `feat/my-new-feature`?

While descriptive branch names are nice, they don't link back to the issue
tracker. If someone reviews your PR six months from now, they need to understand
*why* this feature was added. The issue number is the permanent record of that
decision.

---

## Step 5: Do the Work

Now you can finally write code! Depending on what you're adding, your work will
live in one of these locations:

| What you're adding | Where it goes |
| :--- | :--- |
| A new high-level primitive (e.g., `TimedGate`) | `src/batteries/primitives/` |
| A new lock backend (e.g., `TicketLock`) | `src/batteries/lock/` |
| A new retry strategy (e.g., `ExponentialBackoff`) | `src/batteries/retry/` |
| A new poison policy | `src/batteries/poison.rs` |
| A new behavior-driven API trait | `src/api/` |

### The Checklist for Implementation

As you write your code, ensure you are:

1. **Composing existing traits.** Your new primitive should accept generic
   `L: LockPolicy`, `R: RetryPolicy`, and `P: PoisonPolicy` parameters.
2. **Writing doc-tests.** Every public struct and method needs a `///`
   doc-comment with an executable `rust` code block.
3. **Respecting the error taxonomy.** Use `AcquireError`, `TryLockError`, and
   `LockStatus` correctly. Never panic on recoverable errors.
4. **Adding tests.** Place tests in `tests/[my-new-feature]-[case-name].rs`.
5. **Updating the Library Book.** If your feature is user-facing, add a section
   to `book/lib/src/batteries.md` or create a new chapter.

### Running Tests Locally

Don't wait for CI to tell you something is broken. Run the full test suite with
commit:

````bash
./commit
````

This runs `cargo fmt`, `clippy`, and the entire test suite. If it passes, you're
99% ready for a PR.

---

## Step 6: Open a Pull Request

Push your branch to your fork and open a PR against `main`:

````bash
git push origin timed-gate-12345
````

The PR template will automatically populate with a checklist. ***Fill out every
item.***

### Linking the Issue

In the PR description, reference the originating issue:

````markdown
## Description

Implements `TimedGate`.

This primitive wraps a `Gate<L, R>` and uses a timeout-based `RetryPolicy`
to automatically transition from closed to open after a specified duration.

Fixes #12345
````

The `Fixes #12345` line is magic — when the PR is merged, GitHub will
automatically close the linked issue.

---

## Step 7: Code Review

Once the PR is open, maintainers and other contributors will review your code.
This is not a personal critique — it's a collaborative effort to ensure the
codebase remains safe, consistent, and performant.

### What Reviewers Check

1. **Miri is clean.** If your code touches `unsafe` or atomic operations,
   reviewers will run `cargo miri test` to check for data races and UB.
2. **No `#![no_std]` regressions.** If you added a feature that requires `std`,
   it must be properly gated behind `#[cfg(feature = "std")]`.
3. **Trait bounds are correct.** Are you requiring `L: SharingPolicy` when
   `LockPolicy` would suffice? Are you missing `Send` or `Sync` bounds?
4. **Error handling is granular.** Are you using `AcquireError::Retry` for
   timeouts instead of panicking?
5. **Doc-tests actually run.** Reviewers will click the "play" button in the
   rendered docs to ensure your examples compile and pass.

### Responding to Feedback

- **Address every comment.** If you disagree, explain why. If you agree, push
  a fix.
- **Don't force-push during review.** Force-pushing rewrites history and makes
  it hard for reviewers to see what changed. Just push new commits. Refactoring
  non-"ideal" code is much better than attempts to erase mistakes from world's
  history.
- **Mark resolved comments.** Once you've addressed a comment, click "Resolve
  conversation" to signal that the thread is done.

### The Approval Threshold

A PR typically requires **at least one maintainer approval** before merging.
Complex features (like new lock backends or changes to core traits) may require
two approvals.

---

## Step 8: Merge and Close

Once approved and all CI checks are green, a maintainer will merge your PR
into `main`. The merge commit will look like this:

````text
Merge pull request #12400 from vi-is-ramen/timed-gate-12345

feat(gate): add TimedGate primitive
````

### What Happens Next?

1. **The issue closes automatically.** Because you included `Fixes #12345` in
   the PR description, GitHub closes the issue immediately.
2. **CI runs on `main`.** The full test matrix runs again to ensure nothing
   broke during the merge.

> **ATTENTION**
>
> If you want your feature to be part of the next release, say it in Issue's
> discussion - maintainer would respect your wish when scheduling versions.
>
> Bumping the version in `Cargo.toml` without maintainers' approval lead PR to
> be rejected.

---

## Step 9: PROFIT!

Your feature is now live in Resync! Users can add it to their `Cargo.toml` and
start using it immediately.

### After the Merge

- **Celebrate!** You just made a lasting contribution to the Rust ecosystem.
- **Consider writing a blog post** or sharing your experience on social media.
  Tag the Resync repository — we love seeing what you build!

---

## Summary

The feature lifecycle is not bureaucracy for its own sake. Each step serves a
purpose:

| Step | Purpose |
| :--- | :--- |
| **1. Issue** | Creates a permanent record of the design decision. |
| **2. Discussion** | Catches design flaws before code is written. |
| **3. Approval** | Ensures maintainers are aligned on the direction. |
| **4. Branch** | Links code to the originating issue. |
| **5. Work** | Implements the feature safely and idiomatically. |
| **6. PR** | Packages the work for review with proper metadata. |
| **7. Review** | Catches bugs, UB, and API inconsistencies. |
| **8. Merge** | Lands the code and triggers automation. |
| **9. PROFIT** | Delivers value to the community. |

Following this process is the fastest way to get your feature merged and to
build trust with the maintainers. Skip steps, and you'll find yourself
rewriting code or having PRs closed.

Ready to handle something more sensitive? Continue to the
[Security Patch Lifecycle](./sechole.md) to learn how to responsibly report
and fix soundness holes and data races.
