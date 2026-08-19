- Write benches for everything.
- Write CI script to automatically generate GitHub Releases.
- Own `lock_api` alternative (in `resync::api`, I guess) -> partially done.
- fix integration tests (in some reason, they have not access to Resync's public
API; Idk why is it happening, more investigation required, but code of Resync
itself is OK; anyway we can schedule this task until we are 1.0).
- Add `Counter` trait for counters, `Guard` trait for guards.
- Implement custom reference counting with `Counter` parameter.
