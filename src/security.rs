//! Security helpers: token generation, HMAC verification, audit hashing.

use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

/// Generate a cryptographically random hex token of `bytes * 2` chars.
pub fn random_token_hex(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    rand::thread_rng().fill_bytes(&mut buf);
    hex::encode(buf)
}

/// Compute the HMAC-SHA256 of `payload` with `secret`, returned as hex.
pub fn hmac_sha256_hex(secret: &[u8], payload: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret).expect("hmac key");
    mac.update(payload);
    hex::encode(mac.finalize().into_bytes())
}

/// Constant-time comparison.
pub fn ct_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.bytes().zip(b.bytes()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Current epoch seconds (u64).
pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Build a tamper-evident hash chain entry: SHA256(prev_hash || row_json).
/// `prev_hash` is the hex digest of the previous entry, or 64 zeros for
/// the first row.
pub fn chain_hash(prev_hash: &str, row_json: &str) -> String {
    use sha2::Digest;
    let mut h = Sha256::new();
    h.update(prev_hash.as_bytes());
    h.update(b"|");
    h.update(row_json.as_bytes());
    hex::encode(h.finalize())
}

/// The genesis hash (64 zeros) used as the predecessor of the first
/// audit-log entry.
pub const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_unique() {
        let a = random_token_hex(16);
        let b = random_token_hex(16);
        assert_ne!(a, b);
        assert_eq!(a.len(), 32);
    }

    #[test]
    fn hmac_stable() {
        let h = hmac_sha256_hex(b"k", b"hello");
        assert_eq!(h.len(), 64);
    }

    #[test]
    fn ct_eq_works() {
        assert!(ct_eq("abc", "abc"));
        assert!(!ct_eq("abc", "abd"));
        assert!(!ct_eq("abc", "abcd"));
    }

    #[test]
    fn chain_stable() {
        let h1 = chain_hash(GENESIS_HASH, r#"{"a":1}"#);
        let h2 = chain_hash(GENESIS_HASH, r#"{"a":1}"#);
        assert_eq!(h1, h2);
        let h3 = chain_hash(GENESIS_HASH, r#"{"a":2}"#);
        assert_ne!(h1, h3);
    }
}
