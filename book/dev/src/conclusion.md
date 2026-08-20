# Summary

You have reached the end of the **Resync Developer's Guide**.

Throughout this book, we have covered the essential workflows, tools, and
philosophies that keep the Resync project healthy, safe, and evolving. Let's
briefly recap the core principles that define our development process.

---

## 1. The Core Principles

Whether you are fixing a typo, optimizing a futex slow-path, or implementing a
brand new `LockPolicy`, these rules always apply:

- **Compose, don't hardcode.** Resync is a LEGO-like library. Always decouple
  *acquisition* (`LockPolicy`) from *waiting* (`RetryPolicy`) and *poisoning*
  (`PoisonPolicy`).
- **Safety first.** Concurrency is unforgiving. Rely on Miri to catch data
  races, write characterization tests to protect the public API, and follow the
  Security Patch Lifecycle for soundness holes.
- **Respect the contract.** Follow Semantic Versioning and Conventional Commits.
  The automated release pipeline depends entirely on your commit messages.
- **Communicate early.** Never start coding a feature without an approved issue.
  Never report a security vulnerability in a public issue.

> **ATTENTION**
>
> The `src/lib.rs`, `src/result.rs`, and `src/util.rs` files are the untouchable
> zone. Changes to the core error taxonomy or crate root are breaking changes
> that affect the entire ecosystem. Edit them only with a maintainer's explicit
> approval.

---

## 2. The Developer's Toolkit

Keep this command close at hand during your development cycle:

```bash
./commit -m "feat(scope): your message here"
```

If `just pre-commit` passes, your commit message follows the Conventional
Commits specification, and your PR description links to the originating issue,
your contribution is already in excellent shape.

---

## 3. Where to Go Next

This guide focused strictly on the *contributor's* perspective. Depending on
what you want to do next, here are your next steps:

- **Learn how to use Resync:** Head over to the
  [Resync Library Book](https://vi-is-ramen.github.io/resync/lib/) to explore
  the philosophy, design decisions, and advanced usage patterns like `Shield`
  for writer-fairness or `Gate` for thread-pool initialization.
- **Read the API Reference:** Visit [docs.rs/resync](https://docs.rs/resync)
  for the exhaustive, auto-generated documentation of every trait, struct, and
  method.
- **Join the Discussion:** Open an issue on
  [GitHub](https://github.com/vi-is-ramen/resync/issues) to ask questions,
  propose features, or report bugs.
- **Track Releases:** Watch the repository on GitHub to get notified when your
  merged PRs are published to [crates.io](https://crates.io/crates/resync).

---

## Final Words

Open-source software thrives because people like you decide to give back.
Whether you are adding a new synchronization primitive, improving a benchmark,
or simply fixing a broken link in the documentation, your contribution makes
the Rust concurrency ecosystem a little bit better, a little bit safer, and a
little bit more flexible.

Thank you for reading, and welcome to the Resync team.

-- Ivan
