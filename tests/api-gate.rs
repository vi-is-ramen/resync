//! Characterization: Gate controllable barrier.
#![cfg(feature = "dev")]

use resync::lock::Atomic;
use resync::retry::Busy;
use resync::{Gate, TryLockError};

type TestGate = Gate<Atomic, Busy>;

#[test]
fn gate_new_starts_closed()
{
    let g = TestGate::new();
    assert!(matches!(g.try_wait(), Err(TryLockError::Contention)));
}

#[test]
fn gate_new_open_starts_open()
{
    let g = TestGate::new_open();
    assert!(g.try_wait().is_ok());
}

#[test]
fn gate_open_unblocks_try_wait()
{
    let g = TestGate::new();
    assert!(g.try_wait().is_err());
    g.open();
    assert!(g.try_wait().is_ok());
}

#[test]
fn gate_close_blocks_again()
{
    let g = TestGate::new_open();
    g.close().unwrap();
    assert!(matches!(g.try_wait(), Err(TryLockError::Contention)));
    g.open();
    assert!(g.try_wait().is_ok());
}

#[test]
fn gate_close_is_idempotent()
{
    let g = TestGate::new_open();
    g.close().unwrap();
    g.close().unwrap(); // already closed -> immediate Ok
    assert!(g.try_wait().is_err());
}

#[test]
fn gate_open_is_idempotent()
{
    let g = TestGate::new();
    g.open();
    g.open(); // already open -> no-op
    assert!(g.try_wait().is_ok());
}

#[test]
fn gate_debug()
{
    let g = TestGate::new();
    let _ = format!("{g:?}");
}
