use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::SystemTime;

/// Generates a random string of 16 hexadecimal digits.
///
/// The result is a `String` of exactly 16 characters, each in `0-9a-f`.
/// This is not cryptographically secure; it combines system time and hashing.
///
/// # Example
/// ```ignore
/// # use crate::util::random_hex_16;
/// let id = random_hex_16();
/// assert_eq!(id.len(), 16);
/// assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
/// ```
#[cfg_attr(feature = "__lint", reta::pub_)]
pub(crate) fn random_hex_16() -> String
{
    // Use current time as a source of entropy.
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("system time before UNIX epoch?")
        .as_nanos();

    // Hash the timestamp to produce a deterministic pseudo‑random u64.
    let mut hasher = DefaultHasher::new();
    now.hash(&mut hasher);
    let hash = hasher.finish();

    // Format as a 16‑digit hex string with leading zeros.
    format!("{:016x}", hash)
}
