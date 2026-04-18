//! Compression utilities handler.

use crate::cli::actions::CompressAction;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde_json::json;
use std::fs;

/// Handle compress subcommands.
pub fn handle_compress(action: &CompressAction) {
    match action {
        CompressAction::Gzip { text, level } => {
            let compression_level = match level.as_str() {
                "fast" => rsk::CompressionLevel::Fast,
                "best" => rsk::CompressionLevel::Best,
                _ => rsk::CompressionLevel::Default,
            };
            let result = rsk::gzip_compress_string(text, compression_level);
            println!(
                "{}",
                json!({
                    "original_size": result.original_size,
                    "compressed_size": result.compressed_size,
                    "ratio": result.ratio,
                    "savings_percent": result.savings_percent,
                    "data_base64": BASE64.encode(&result.data),
                })
            );
        }
        CompressAction::Gunzip { data } => match BASE64.decode(data) {
            Ok(bytes) => match rsk::gzip_decompress_string(&bytes) {
                Ok(text) => println!(
                    "{}",
                    json!({
                        "status": "success",
                        "text": text,
                    })
                ),
                Err(e) => println!(
                    "{}",
                    json!({
                        "status": "error",
                        "message": e,
                    })
                ),
            },
            Err(e) => println!(
                "{}",
                json!({
                    "status": "error",
                    "message": format!("Invalid base64: {}", e),
                })
            ),
        },
        CompressAction::File {
            path,
            output,
            level,
        } => {
            let compression_level = match level.as_str() {
                "fast" => rsk::CompressionLevel::Fast,
                "best" => rsk::CompressionLevel::Best,
                _ => rsk::CompressionLevel::Default,
            };
            match fs::read(path) {
                Ok(data) => {
                    let result = rsk::gzip_compress(&data, compression_level);
                    let out_path = output.clone().unwrap_or_else(|| format!("{path}.gz"));
                    match fs::write(&out_path, &result.data) {
                        Ok(_) => println!(
                            "{}",
                            json!({
                                "status": "success",
                                "input_path": path,
                                "output_path": out_path,
                                "original_size": result.original_size,
                                "compressed_size": result.compressed_size,
                                "ratio": result.ratio,
                                "savings_percent": result.savings_percent,
                            })
                        ),
                        Err(e) => println!(
                            "{}",
                            json!({
                                "status": "error",
                                "message": format!("Failed to write output: {}", e),
                            })
                        ),
                    }
                }
                Err(e) => println!(
                    "{}",
                    json!({
                        "status": "error",
                        "message": format!("Failed to read input: {}", e),
                    })
                ),
            }
        }
        CompressAction::Estimate { text } => {
            let ratio = rsk::estimate_compressibility(text.as_bytes());
            let compressibility = if ratio < 0.3 {
                "highly_compressible"
            } else if ratio < 0.6 {
                "moderately_compressible"
            } else if ratio < 0.8 {
                "low_compressibility"
            } else {
                "incompressible"
            };
            #[allow(
                clippy::as_conversions,
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss
            )]
            // usize→f64 for ratio multiplication, f64→usize for display — safe for text lengths
            let estimated_compressed_size = (text.len() as f64 * ratio).round() as usize;
            println!(
                "{}",
                json!({
                    "estimated_ratio": ratio,
                    "compressibility": compressibility,
                    "input_size": text.len(),
                    "estimated_compressed_size": estimated_compressed_size,
                })
            );
        }
    }
}
