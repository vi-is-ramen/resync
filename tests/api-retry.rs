//! Characterization: retry policies (Busy, Yield, DefaultRetry).

use core::convert::Infallible;
use resync::api::RetryPolicy;
use resync::retry::Busy;

// Compile-time: Busy never aborts (Error = Infallible).
#[allow(dead_code)]
fn _assert_busy_error()
{
    fn check<R: RetryPolicy<Error = Infallible>>() {}
    check::<Busy>();
}

#[test]
fn busy_never_aborts()
{
    let busy = Busy;
    for i in 0..16
    {
        assert!(busy.retry(i).is_ok());
    }
}

#[test]
fn busy_default_and_debug()
{
    #[allow(clippy::default_constructed_unit_structs)]
    let busy = Busy::default();
    let _ = format!("{busy:?}");
    assert!(busy.retry(0).is_ok());
}

#[cfg(feature = "std")]
#[test]
fn yield_never_aborts()
{
    let y = resync::retry::Yield::new();
    for i in 0..16
    {
        assert!(y.retry(i).is_ok());
    }
    let _ = format!("{y:?}");
}

#[test]
fn default_retry_is_usable()
{
    // DefaultRetry resolves to Yield (std) or Busy (no_std); both never abort.
    let r = <resync::retry::DefaultRetry as Default>::default();
    assert!(r.retry(0).is_ok());
}
