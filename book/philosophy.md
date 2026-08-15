# 1. Philosophy: The "LEGO" Approach

Unlike standard library synchronization primitives that provide a monolithic
`Mutex` or `RwLock`, Resync treats synchronization as a composition of
smaller, independent behaviors.

A blocking mutex is essentially two behaviors combined:

1. **Acquisition:** How do we atomically claim ownership of a resource?
2. **Waiting:** What do we do while the resource is held by someone else?

Resync decouples these concerns into the `LockPolicy` and
`RetryPolicy` traits. This allows you to mix and match atomic acquisition
strategies with different spin-wait strategies at compile time, tailoring
the primitive exactly to your performance and environment constraints.
