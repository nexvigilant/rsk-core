//! Compression utilities for RSK
//!
//! Provides gzip compression/decompression with performance metrics.

use flate2::Compression;
use flate2::read::{GzDecoder, GzEncoder};
use serde::{Deserialize, Serialize};
use std::io::Read;

/// Result of compression operation
#[derive(Debug, Serialize, Deserialize)]
pub struct CompressionResult {
    pub original_size: usize,
    pub compressed_size: usize,
    pub ratio: f64,
    pub savings_percent: f64,
    pub data: Vec<u8>,
}

/// Result of decompression operation
#[derive(Debug, Serialize, Deserialize)]
pub struct DecompressionResult {
    pub compressed_size: usize,
    pub decompressed_size: usize,
    pub expansion_ratio: f64,
    pub data: Vec<u8>,
}

/// Compression level preset
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CompressionLevel {
    /// Fastest compression, lowest ratio
    Fast,
    /// Balanced speed and ratio
    Default,
    /// Best compression ratio, slowest
    Best,
}

impl CompressionLevel {
    fn to_flate2(self) -> Compression {
        match self {
            CompressionLevel::Fast => Compression::fast(),
            CompressionLevel::Default => Compression::default(),
            CompressionLevel::Best => Compression::best(),
        }
    }
}

/// Compress data using gzip
pub fn gzip_compress(data: &[u8], level: CompressionLevel) -> CompressionResult {
    let original_size = data.len();

    let mut encoder = GzEncoder::new(data, level.to_flate2());
    let mut compressed = Vec::new();
    encoder.read_to_end(&mut compressed).unwrap_or_default();

    let compressed_size = compressed.len();
    #[allow(clippy::as_conversions)] // usize→f64 for ratio
    let compressed_f = compressed_size as f64;
    #[allow(clippy::as_conversions)] // usize→f64 for ratio
    let original_f = original_size as f64;
    let ratio = if original_size > 0 {
        compressed_f / original_f
    } else {
        1.0
    };
    // Handle case where compressed is larger than original (negative savings)
    let savings_percent = if original_size > 0 {
        (1.0 - (compressed_f / original_f)) * 100.0
    } else {
        0.0
    };

    CompressionResult {
        original_size,
        compressed_size,
        ratio: (ratio * 1000.0).round() / 1000.0,
        savings_percent: (savings_percent * 100.0).round() / 100.0,
        data: compressed,
    }
}

/// Compress string using gzip
pub fn gzip_compress_string(text: &str, level: CompressionLevel) -> CompressionResult {
    gzip_compress(text.as_bytes(), level)
}

/// Decompress gzip data
pub fn gzip_decompress(data: &[u8]) -> Result<DecompressionResult, String> {
    let compressed_size = data.len();

    let mut decoder = GzDecoder::new(data);
    let mut decompressed = Vec::new();

    decoder
        .read_to_end(&mut decompressed)
        .map_err(|e| format!("Decompression failed: {e}"))?;

    let decompressed_size = decompressed.len();
    #[allow(clippy::as_conversions)] // usize→f64 for ratio
    let decompressed_f = decompressed_size as f64;
    #[allow(clippy::as_conversions)] // usize→f64 for ratio
    let compressed_f = compressed_size as f64;
    let expansion_ratio = if compressed_size > 0 {
        decompressed_f / compressed_f
    } else {
        1.0
    };

    Ok(DecompressionResult {
        compressed_size,
        decompressed_size,
        expansion_ratio: (expansion_ratio * 1000.0).round() / 1000.0,
        data: decompressed,
    })
}

/// Decompress gzip data to string
pub fn gzip_decompress_string(data: &[u8]) -> Result<String, String> {
    let result = gzip_decompress(data)?;
    String::from_utf8(result.data).map_err(|e| format!("Invalid UTF-8 in decompressed data: {e}"))
}

/// Analyze compressibility of data without actually compressing
/// Returns estimated compression ratio based on byte frequency analysis
pub fn estimate_compressibility(data: &[u8]) -> f64 {
    if data.is_empty() {
        return 1.0;
    }

    // Count byte frequencies
    let mut freq = [0usize; 256];
    for &byte in data {
        freq[usize::from(byte)] += 1;
    }

    // Calculate Shannon entropy
    #[allow(clippy::as_conversions)] // usize→f64 for entropy calculation
    let total = data.len() as f64;
    let entropy: f64 = freq
        .iter()
        .filter(|&&count| count > 0)
        .map(|&count| {
            #[allow(clippy::as_conversions)] // usize→f64 for probability
            let p = count as f64 / total;
            -p * p.log2()
        })
        .sum();

    // Estimate compression ratio (entropy / 8 bits per byte)
    // Lower entropy = better compression
    let estimated_ratio = entropy / 8.0;
    let rounded = (estimated_ratio * 1000.0).round() / 1000.0;
    // Avoid -0.0
    if rounded == 0.0 { 0.0 } else { rounded }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gzip_compress_decompress() {
        let original = "Hello, World! This is a test string for compression.";
        let compressed = gzip_compress_string(original, CompressionLevel::Default);

        assert!(compressed.compressed_size > 0);
        assert!(compressed.original_size == original.len());

        let decompressed = gzip_decompress_string(&compressed.data).unwrap();
        assert_eq!(decompressed, original);
    }

    #[test]
    fn test_gzip_compress_repetitive() {
        // Highly repetitive data should compress well
        let repetitive = "a".repeat(1000);
        let result = gzip_compress_string(&repetitive, CompressionLevel::Default);

        assert!(result.ratio < 0.1); // Should be very compressible
        assert!(result.savings_percent > 90.0);
    }

    #[test]
    fn test_gzip_compress_empty() {
        let result = gzip_compress_string("", CompressionLevel::Default);
        assert_eq!(result.original_size, 0);
    }

    #[test]
    fn test_gzip_levels() {
        let data = "test data ".repeat(100);

        let fast = gzip_compress_string(&data, CompressionLevel::Fast);
        let best = gzip_compress_string(&data, CompressionLevel::Best);

        // Best compression should produce smaller or equal output
        assert!(best.compressed_size <= fast.compressed_size);
    }

    #[test]
    fn test_estimate_compressibility_low_entropy() {
        let data = "aaaaaaaaaa".as_bytes();
        let ratio = estimate_compressibility(data);
        assert!(ratio < 0.2); // Low entropy = highly compressible
    }

    #[test]
    fn test_estimate_compressibility_high_entropy() {
        let data: Vec<u8> = (0..=255).collect();
        let ratio = estimate_compressibility(&data);
        assert!(ratio > 0.9); // High entropy = not very compressible
    }

    #[test]
    fn test_decompression_invalid_data() {
        let invalid = vec![1, 2, 3, 4, 5];
        let result = gzip_decompress(&invalid);
        assert!(result.is_err());
    }
}
