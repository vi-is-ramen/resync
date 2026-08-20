# Security Patch Lifecycle

Resync deals with raw pointers, atomic operations, and concurrency. Memory
safety and soundness are not just features — they are the **foundation** of the
entire project. A single data race, undefined behavior, or soundness hole can
undermine the trust of every downstream user.

Because of this, security issues in Resync follow a strict, confidential
process that differs significantly from the regular
[Feature Lifecycle](./feature.md).

**The golden rule is simple: information about a vulnerability must never leave
the circle of Resync developers until a fix is published and all affected
versions are handled.**

This chapter describes the exact steps taken from the moment a vulnerability
is reported to the moment it becomes public knowledge.

---

## Step 1: Private Report

Security vulnerabilities are **never** reported via public GitHub issues. If
you suspect a soundness hole, data race, or any other security-relevant bug,
you must use one of the following private channels:

- GitHub's [private vulnerability reporting](https://github.com/vi-is-ramen/resync/security/advisories/new)
  feature on the repository (preferred);
- A direct email to the project maintainer at
  [vi.is.chapmann@gmail.com](mailto:vi.is.chapmann@gmail.com).

A good security report should include:

- A clear description of the vulnerability (e.g., "data race in `lock::Shield`
  when a writer acquires the lock while readers are mid-acquisition");
- A **minimal reproducible example** (MRE) — ideally a small Rust program or a
  Miri test that demonstrates the issue;
- The affected Resync version(s) (if known);
- Your analysis of the root cause (optional but highly appreciated).

> **SECURITY**
>
> Opening a public issue for a soundness hole puts every Resync user at risk.
> Always report privately.

Upon receipt, the maintainer will acknowledge the report within **48 hours**.

---

## Step 2: Routing to the Author

Once the vulnerability is validated, the maintainer inspects the git history
(using `git blame` and `git log`) to identify the contributor who wrote the
affected code. The maintainer then privately forwards the report to that author
via email or direct message, inviting them to lead the fix.

This step respects the ownership principle: the person who wrote the code is
usually the best person to understand its invariants and fix it correctly.

---

## Step 3: The 3-Day Window

The original author is given **3 calendar days** to respond to the maintainer's
message and confirm whether they intend to work on the fix.

- **If the author responds and agrees**: They take ownership of the fix and
  proceed to Step 4.
- **If the author does not respond within 3 days**: The maintainer assumes the
  author is no longer actively maintaining their contribution. The fix is then
  reassigned.

> **TIP**
>
> This window is short by design. Security vulnerabilities cannot linger in
> limbo — they must be resolved or escalated quickly.

---

## Step 4: Finding a Fixer and Writing the Regression Test

If the original author is unavailable (or declines), the maintainer searches
for a volunteer among trusted contributors. If no volunteer steps forward, the
maintainer **fixes the vulnerability themselves**.

### The Regression Test Comes First

Before a single line of production code is changed, the fixer **must** write a
test that reliably reproduces the vulnerability. This test typically takes one
of two forms:

- A **Miri test** for data races and undefined behavior.
- A **Loom-style concurrency test** for logical soundness holes that Miri
  cannot easily detect.

This test is added to the `tests/` directory and **must fail on the current
`main` branch**. The fix is not considered complete until this test passes.

### Checking Every Released Version

This is the most critical and unique part of Resync's security process.
**The vulnerability is checked against every single git tag**, not just the
current `main` branch.

The fixer (or maintainer) runs the regression test against each historical
version of the crate. For every tag where the test fails (i.e., the
vulnerability is present), the corresponding version on crates.io is marked
as **yanked**.

```bash
# Conceptual workflow — not an actual command
for tag in $(git tag); do
    git checkout $tag
    if cargo test --test security_regression_test; then
        echo "$tag is safe"
    else
        echo "$tag is vulnerable — yanking on crates.io"
        # cargo yank resync@$tag (requires registry token)
    fi
done
git checkout main
```

> **WARNING**
>
> Yanking is a serious action. It prevents new projects from depending on the
> vulnerable version, but does not break existing `Cargo.lock` files. We yank
> aggressively for soundness holes because continuing to use a vulnerable
> version of a concurrency library is a recipe for disaster.

### Confidentiality During the Fix

Throughout this entire process — from the initial report to the final yank —
**no information about the vulnerability is shared publicly**. This includes:

- No commits referencing the vulnerability on public branches.
- No discussions on public forums, Discord, or X.
- No public pull requests. The fix is developed on a private branch or
  via GitHub's private security advisory workflow.

The goal is to prevent malicious actors from exploiting the vulnerability
before a fix is available to all users.

---

## Step 5: Merge and Patch Release

Once the regression test passes and all affected historical versions have been
yanked, the fix is merged into `main` using the standard CI pipeline. The merge
must pass:

- `cargo fmt` and `cargo clippy`
- The full cross-platform test matrix
- **Miri** (critical for any security fix)

Immediately after the merge, the maintainer bumps the **patch version** in
`Cargo.toml` (e.g., `0.10.3` -> `0.10.4`). The automated `release` job in CI
then:

1. Publishes the new patch version to crates.io.
2. Creates a git tag `v0.10.4`.

Users on the affected version line will receive the fix via a routine
`cargo update`.

---

## Step 6: Public Disclosure

Only **after** the patch is published and all vulnerable historical versions
are yanked does the vulnerability become public.

The maintainer opens a GitHub Issue describing:

- The nature of the vulnerability.
- The affected version range(s).
- The fix applied and the PR that resolved it.

This issue is **immediately closed** with a comment pointing to the PR and the
patch release. Its sole purpose is to create a public, searchable record of
the incident for future reference.

A summary is also added to the release notes generated by `scripts/chlog.py`,
typically under a dedicated `## Security` section.

> **SECURITY**
>
> Public disclosure always happens *after* the fix is available. This gives
> users a safe upgrade path before the details of the exploit become widely
> known.

---

## Summary

The security patch lifecycle is designed around three principles:

| Principle | How it's enforced |
| :--- | :--- |
| **Confidentiality** | Private reporting, private development branches, no public discussion until Step 6. |
| **Thoroughness** | Every git tag is checked; every affected version is yanked. |
| **Speed** | 3-day author window, patch release immediately after merge. |

Security is not a feature — it is a continuous obligation. If you ever spot
something suspicious in Resync's code, please report it through the proper
channels. The community, and every downstream user, thanks you.

---

Now that you know how to safely handle the most critical issues, let's turn to
the everyday concern: preventing API regressions. Continue to
[Attention! API Breaks? Be preventive!](./regressions.md).
