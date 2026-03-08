use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Serialize, Deserialize)]
pub struct HashResult {
    pub algorithm: String,
    pub hex: String,
    pub bytes_hashed: usize,
}

/// Compute SHA-256 hash of input string, returning hex digest.
pub fn sha256_hash(input: &str) -> HashResult {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();

    HashResult {
        algorithm: "SHA-256".to_string(),
        hex: format!("{result:x}"),
        bytes_hashed: input.len(),
    }
}

/// Compute SHA-256 hash of raw bytes.
pub fn sha256_bytes(input: &[u8]) -> HashResult {
    let mut hasher = Sha256::new();
    hasher.update(input);
    let result = hasher.finalize();

    HashResult {
        algorithm: "SHA-256".to_string(),
        hex: format!("{result:x}"),
        bytes_hashed: input.len(),
    }
}

/// Verify that a string matches an expected SHA-256 hash.
pub fn sha256_verify(input: &str, expected_hex: &str) -> bool {
    let result = sha256_hash(input);
    result.hex.eq_ignore_ascii_case(expected_hex)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ═══════════════════════════════════════════════════════════════
    // SHA256_HASH: POSITIVE TESTS
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_sha256_empty() {
        let result = sha256_hash("");
        assert_eq!(
            result.hex,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(result.bytes_hashed, 0);
    }

    #[test]
    fn test_sha256_hello() {
        let result = sha256_hash("hello");
        assert_eq!(
            result.hex,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
        assert_eq!(result.bytes_hashed, 5);
    }

    #[test]
    fn test_sha256_hello_world() {
        let result = sha256_hash("hello world");
        assert_eq!(
            result.hex,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_sha256_algorithm_label() {
        let result = sha256_hash("test");
        assert_eq!(result.algorithm, "SHA-256");
    }

    #[test]
    fn test_sha256_deterministic() {
        // Same input always produces same output
        let result1 = sha256_hash("reproducible");
        let result2 = sha256_hash("reproducible");
        assert_eq!(result1.hex, result2.hex);
    }

    // ═══════════════════════════════════════════════════════════════
    // SHA256_HASH: EDGE CASES
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_sha256_single_char() {
        let result = sha256_hash("a");
        assert_eq!(
            result.hex,
            "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb"
        );
    }

    #[test]
    fn test_sha256_whitespace() {
        let result = sha256_hash(" ");
        assert_eq!(result.bytes_hashed, 1);
        // Space has a different hash than empty
        assert_ne!(
            result.hex,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_sha256_newlines() {
        let result1 = sha256_hash("hello\n");
        let result2 = sha256_hash("hello");
        assert_ne!(result1.hex, result2.hex);
    }

    #[test]
    fn test_sha256_unicode() {
        let result = sha256_hash("日本語");
        assert_eq!(result.bytes_hashed, 9); // 3 chars * 3 bytes each in UTF-8
        assert_eq!(result.hex.len(), 64); // SHA-256 always 64 hex chars
    }

    #[test]
    fn test_sha256_emoji() {
        let result = sha256_hash("🎉");
        assert_eq!(result.bytes_hashed, 4); // Emoji is 4 bytes in UTF-8
    }

    #[test]
    fn test_sha256_hex_length() {
        // SHA-256 always produces 256 bits = 64 hex characters
        let result = sha256_hash("any input at all");
        assert_eq!(result.hex.len(), 64);
    }

    // ═══════════════════════════════════════════════════════════════
    // SHA256_BYTES: TESTS
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_sha256_bytes_empty() {
        let result = sha256_bytes(&[]);
        assert_eq!(
            result.hex,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_sha256_bytes_hello() {
        let result = sha256_bytes(b"hello");
        assert_eq!(
            result.hex,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn test_sha256_bytes_binary() {
        // Binary data that's not valid UTF-8
        let result = sha256_bytes(&[0x00, 0xFF, 0x01, 0xFE]);
        assert_eq!(result.bytes_hashed, 4);
        assert_eq!(result.hex.len(), 64);
    }

    #[test]
    fn test_sha256_bytes_matches_string() {
        // Same content via bytes and string should match
        let str_result = sha256_hash("test");
        let bytes_result = sha256_bytes(b"test");
        assert_eq!(str_result.hex, bytes_result.hex);
    }

    // ═══════════════════════════════════════════════════════════════
    // SHA256_VERIFY: TESTS
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_sha256_verify_correct() {
        assert!(sha256_verify(
            "hello",
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        ));
    }

    #[test]
    fn test_sha256_verify_wrong() {
        assert!(!sha256_verify("hello", "wrong_hash"));
    }

    #[test]
    fn test_sha256_verify_case_insensitive() {
        // Should match regardless of case
        assert!(sha256_verify(
            "hello",
            "2CF24DBA5FB0A30E26E83B2AC5B9E29E1B161E5C1FA7425E73043362938B9824"
        ));
    }

    #[test]
    fn test_sha256_verify_mixed_case() {
        assert!(sha256_verify(
            "hello",
            "2cF24DbA5fB0a30E26e83b2aC5B9e29E1b161E5c1Fa7425e73043362938b9824"
        ));
    }

    #[test]
    fn test_sha256_verify_empty() {
        assert!(sha256_verify(
            "",
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        ));
    }

    #[test]
    fn test_sha256_verify_partial_match() {
        // Partial hash should not match
        assert!(!sha256_verify("hello", "2cf24dba5fb0a30e"));
    }

    // ═══════════════════════════════════════════════════════════════
    // STRESS TESTS
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_sha256_large_input() {
        let large_input = "x".repeat(100_000);
        let result = sha256_hash(&large_input);
        assert_eq!(result.bytes_hashed, 100_000);
        assert_eq!(result.hex.len(), 64);
    }

    #[test]
    fn test_sha256_bytes_large_binary() {
        let large_binary: Vec<u8> = (0..10_000).map(|i| (i % 256) as u8).collect();
        let result = sha256_bytes(&large_binary);
        assert_eq!(result.bytes_hashed, 10_000);
    }
}
