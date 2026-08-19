# The Resync Guidebook

Welcome to the guide for Resync. This document covers the library's philosophy,
core concepts, advanced usage patterns, design decisions, and inherent
limitations.

Resync is not just another mutex crate. It is a **LEGO-like toolkit** for
synchronization. Whether you are building a high-throughput user-space server,
a bare-metal embedded kernel, or a complex distributed system, Resync gives you
the raw materials to build exactly the synchronization primitive you need,
while enforcing safe, race-free API boundaries.

In this book, you will learn how to decouple *acquisition* from *waiting*, how
to prevent deadlocks at compile time, and how to leverage Resync's impressive
suite of built-in batteries like `Gate`, `Semaphore`, and `Shield`.
