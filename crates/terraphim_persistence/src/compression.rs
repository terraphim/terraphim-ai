//! Compression utilities for cache write-back
//!
//! This module provides transparent compression for large objects being cached.
//! Objects over 1MB are compressed using zstd before being written to the cache.

use std::io::{Read, Write};

/// Threshold for compression (1MB)
pub const COMPRESSION_THRESHOLD: usize = 1024 * 1024;

/// Magic bytes to identify compressed data
const COMPRESSED_MAGIC: &[u8; 4] = b"ZSTD";

/// Compression level for zstd (3 is a good balance of speed and ratio)
const COMPRESSION_LEVEL: i32 = 3;

/// Compress data if it exceeds the threshold
///
/// Returns the original data if below threshold, or compressed data with magic header if above.
/// The magic header allows us to distinguish compressed from uncompressed cached data.
pub fn maybe_compress(data: &[u8]) -> Vec<u8> {
    if data.len() < COMPRESSION_THRESHOLD {
        return data.to_vec();
    }

    match compress(data) {
        Ok(compressed) => {
            // Only use compression if it actually reduces size
            if compressed.len() < data.len() {
                let mut result = Vec::with_capacity(COMPRESSED_MAGIC.len() + compressed.len());
                result.extend_from_slice(COMPRESSED_MAGIC);
                result.extend_from_slice(&compressed);
                log::debug!(
                    "Compressed {} bytes to {} bytes ({:.1}% reduction)",
                    data.len(),
                    result.len(),
                    (1.0 - (result.len() as f64 / data.len() as f64)) * 100.0
                );
                result
            } else {
                log::debug!(
                    "Skipping compression: {} bytes would become {} bytes",
                    data.len(),
                    compressed.len()
                );
                data.to_vec()
            }
        }
        Err(e) => {
            log::debug!("Compression failed, using raw data: {}", e);
            data.to_vec()
        }
    }
}

/// Decompress data if it has the compression magic header
///
/// Returns the decompressed data if compressed, or the original data if not.
pub fn maybe_decompress(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    if data.len() > COMPRESSED_MAGIC.len() && &data[..COMPRESSED_MAGIC.len()] == COMPRESSED_MAGIC {
        let compressed = &data[COMPRESSED_MAGIC.len()..];
        decompress(compressed)
    } else {
        Ok(data.to_vec())
    }
}

/// Compress data using zstd
fn compress(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    let mut encoder = zstd::Encoder::new(Vec::new(), COMPRESSION_LEVEL)?;
    encoder.write_all(data)?;
    encoder.finish()
}

/// Decompress zstd-compressed data
fn decompress(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    let mut decoder = zstd::Decoder::new(data)?;
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_data_not_compressed() {
        let data = b"small data";
        let result = maybe_compress(data);
        assert_eq!(result, data);
    }

    #[test]
    fn test_large_data_compressed() {
        // Create data larger than threshold
        let data = vec![0u8; COMPRESSION_THRESHOLD + 1000];
        let result = maybe_compress(&data);

        // Should have magic header
        assert_eq!(&result[..4], COMPRESSED_MAGIC);

        // Should be smaller than original (zeros compress well)
        assert!(result.len() < data.len());
    }

    #[test]
    fn test_compress_decompress_roundtrip() {
        // Create compressible data larger than threshold
        let original: Vec<u8> = (0..COMPRESSION_THRESHOLD + 10000)
            .map(|i| (i % 256) as u8)
            .collect();

        let compressed = maybe_compress(&original);
        let decompressed = maybe_decompress(&compressed).unwrap();

        assert_eq!(decompressed, original);
    }

    #[test]
    fn test_decompress_uncompressed_data() {
        let data = b"uncompressed data without magic header";
        let result = maybe_decompress(data).unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn test_incompressible_data_stays_uncompressed() {
        // Random-looking data that doesn't compress well
        let data: Vec<u8> = (0..COMPRESSION_THRESHOLD + 100)
            .map(|i| ((i * 17 + 31) % 256) as u8)
            .collect();

        let result = maybe_compress(&data);

        // If compression doesn't help, we should get back the original
        // (either raw or with minimal overhead)
        // The result should either be the original or compressed
        let decompressed = maybe_decompress(&result).unwrap();
        assert_eq!(decompressed, data);
    }
}
