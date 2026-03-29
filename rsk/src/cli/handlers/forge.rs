//! Forge pipeline handler (feature-gated).

#[cfg(feature = "forge")]
use crate::cli::actions::ForgeAction;
#[cfg(feature = "forge")]
use forge_spec::{load_spec, parse_spec};
#[cfg(feature = "forge")]
use serde_json::json;
#[cfg(feature = "forge")]
use std::fs;

/// Handle forge subcommands (feature-gated).
#[cfg(feature = "forge")]
pub fn handle_forge(action: &ForgeAction) {
    match action {
        ForgeAction::Validate { path } => match fs::read_to_string(path) {
            Ok(content) => match load_spec(&content) {
                Ok(spec) => {
                    let ingest_count = spec.ingest.len();
                    let transform_count = spec.transform.len();
                    let sink_count = spec.sink.len();
                    println!(
                        "{}",
                        json!({
                            "status": "valid",
                            "pipeline": spec.pipeline.name,
                            "version": spec.pipeline.version,
                            "ingest_sources": ingest_count,
                            "transforms": transform_count,
                            "sinks": sink_count,
                        })
                    );
                }
                Err(e) => {
                    println!(
                        "{}",
                        json!({
                            "status": "invalid",
                            "error": e.to_string(),
                        })
                    );
                    std::process::exit(1);
                }
            },
            Err(e) => {
                println!(
                    "{}",
                    json!({
                        "status": "error",
                        "message": format!("Failed to read file: {}", e),
                    })
                );
                std::process::exit(1);
            }
        },
        ForgeAction::Parse { path } => match fs::read_to_string(path) {
            Ok(content) => match parse_spec(&content) {
                Ok(spec) => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&spec).unwrap_or_default()
                    );
                }
                Err(e) => {
                    println!("{}", json!({"status": "error", "message": e.to_string()}));
                    std::process::exit(1);
                }
            },
            Err(e) => {
                println!("{}", json!({"status": "error", "message": e.to_string()}));
                std::process::exit(1);
            }
        },
        ForgeAction::Graph { path } => match fs::read_to_string(path) {
            Ok(content) => match parse_spec(&content) {
                Ok(spec) => {
                    // Build a simple graph representation
                    let mut nodes = Vec::new();
                    let mut edges = Vec::new();

                    // Add ingest nodes
                    for ingest in &spec.ingest {
                        nodes.push(json!({
                            "id": &ingest.id,
                            "type": "ingest",
                            "source_type": format!("{:?}", ingest.source_type),
                        }));
                    }

                    // Add transform nodes and edges
                    let mut prev_id: Option<&str> = spec.ingest.first().map(|i| i.id.as_str());
                    for transform in &spec.transform {
                        nodes.push(json!({
                            "id": &transform.id,
                            "type": "transform",
                            "operation": format!("{:?}", transform.operation),
                        }));
                        if let Some(prev) = prev_id {
                            edges.push(json!({
                                "from": prev,
                                "to": &transform.id,
                            }));
                        }
                        prev_id = Some(&transform.id);
                    }

                    // Connect last node to sinks
                    if let Some(last_id) = prev_id {
                        for sink in &spec.sink {
                            edges.push(json!({
                                "from": last_id,
                                "to": &sink.id,
                            }));
                        }
                    }

                    // Add sink nodes
                    for sink in &spec.sink {
                        nodes.push(json!({
                            "id": &sink.id,
                            "type": "sink",
                            "sink_type": format!("{:?}", sink.sink_type),
                        }));
                    }

                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json!({
                            "pipeline": spec.pipeline.name,
                            "nodes": nodes,
                            "edges": edges,
                        }))
                        .unwrap_or_default()
                    );
                }
                Err(e) => {
                    println!("{}", json!({"status": "error", "message": e.to_string()}));
                    std::process::exit(1);
                }
            },
            Err(e) => {
                println!("{}", json!({"status": "error", "message": e.to_string()}));
                std::process::exit(1);
            }
        },
        ForgeAction::Sources => {
            println!(
                "{}",
                json!({
                    "sources": [
                        {"type": "stdin", "description": "Standard input"},
                        {"type": "http_json", "description": "HTTP endpoint returning JSON"},
                        {"type": "http_csv", "description": "HTTP endpoint returning CSV"},
                        {"type": "s3_parquet", "description": "S3 bucket with Parquet files"},
                        {"type": "s3_json", "description": "S3 bucket with JSON files"},
                        {"type": "postgres", "description": "PostgreSQL database"},
                        {"type": "mysql", "description": "MySQL database"},
                        {"type": "sqlite", "description": "SQLite database"},
                    ]
                })
            );
        }
        ForgeAction::Transforms => {
            println!(
                "{}",
                json!({
                    "transforms": [
                        {"operation": "filter", "description": "Filter rows by expression"},
                        {"operation": "select", "description": "Select/rename columns"},
                        {"operation": "aggregate", "description": "Group and aggregate data"},
                        {"operation": "join", "description": "Join with another source"},
                        {"operation": "deduplicate", "description": "Remove duplicate rows"},
                        {"operation": "chunk", "description": "Split into chunks for batching"},
                        {"operation": "embed", "description": "Generate embeddings for text"},
                        {"operation": "signal_detect_prr", "description": "PRR signal detection (pharmacovigilance)"},
                    ]
                })
            );
        }
        ForgeAction::Run {
            path,
            input,
            dry_run,
        } => {
            use std::io::{self, Read as IoRead, Write as IoWrite};

            match fs::read_to_string(path) {
                Ok(content) => match load_spec(&content) {
                    Ok(spec) => {
                        // Validate we can run this pipeline
                        let source = spec.ingest.first();
                        let sink = spec.sink.first();

                        let source_type = source.map(|s| format!("{:?}", s.source_type));
                        let sink_type = sink.map(|s| format!("{:?}", s.sink_type));

                        if *dry_run {
                            println!(
                                "{}",
                                json!({
                                    "status": "dry_run",
                                    "pipeline": spec.pipeline.name,
                                    "source": source_type,
                                    "transforms": spec.transform.len(),
                                    "sink": sink_type,
                                    "would_execute": true,
                                })
                            );
                            return;
                        }

                        // Get input data
                        let data: String = if let Some(input_str) = input {
                            input_str.clone()
                        } else if source
                            .is_some_and(|s| matches!(s.source_type, forge_spec::SourceType::Stdin))
                        {
                            let mut buffer = String::new();
                            io::stdin().read_to_string(&mut buffer).unwrap_or_default();
                            buffer
                        } else {
                            eprintln!(
                                "{}",
                                json!({
                                    "status": "error",
                                    "message": "Source type not supported for direct execution. Use --input or stdin source.",
                                })
                            );
                            std::process::exit(1);
                        };

                        // Apply transforms (basic JSON-aware processing)
                        let mut result = data;
                        for transform in &spec.transform {
                            match &transform.operation {
                                forge_spec::Operation::Deduplicate => {
                                    // For JSON arrays, deduplicate
                                    if let Ok(json_val) =
                                        serde_json::from_str::<serde_json::Value>(&result)
                                        && let Some(arr) = json_val.as_array()
                                    {
                                        let mut seen = std::collections::HashSet::new();
                                        let deduped: Vec<_> = arr
                                            .iter()
                                            .filter(|item| {
                                                let key = item.to_string();
                                                seen.insert(key)
                                            })
                                            .cloned()
                                            .collect();
                                        result = serde_json::to_string_pretty(&deduped)
                                            .unwrap_or(result);
                                    }
                                }
                                _ => {
                                    // Other transforms: pass through (log for debug)
                                    eprintln!(
                                        "Transform {:?} not yet implemented, passing through",
                                        transform.operation
                                    );
                                }
                            }
                        }

                        // Output to sink
                        if sink.is_some_and(|s| matches!(s.sink_type, forge_spec::SinkType::Stdout))
                        {
                            if let Err(e) = io::stdout().write_all(result.as_bytes()) {
                                eprintln!("stdout write error: {e}");
                                std::process::exit(1);
                            }
                            if !result.ends_with('\n') {
                                if let Err(e) = io::stdout().write_all(b"\n") {
                                    eprintln!("stdout write error: {e}");
                                    std::process::exit(1);
                                }
                            }
                        } else if let Some(s) = sink {
                            match &s.sink_type {
                                forge_spec::SinkType::JsonFile => {
                                    if let Some(ref file_config) = s.file {
                                        fs::write(&file_config.path, &result).unwrap_or_else(|e| {
                                            eprintln!("Failed to write JSON: {}", e);
                                        });
                                        println!(
                                            "{}",
                                            json!({
                                                "status": "success",
                                                "output_path": file_config.path,
                                                "bytes_written": result.len(),
                                            })
                                        );
                                    }
                                }
                                _ => {
                                    eprintln!(
                                        "{}",
                                        json!({
                                            "status": "error",
                                            "message": format!("Sink type {:?} not yet supported for execution", s.sink_type),
                                        })
                                    );
                                    std::process::exit(1);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "{}",
                            json!({
                                "status": "error",
                                "message": e.to_string(),
                            })
                        );
                        std::process::exit(1);
                    }
                },
                Err(e) => {
                    eprintln!(
                        "{}",
                        json!({
                            "status": "error",
                            "message": format!("Failed to read file: {}", e),
                        })
                    );
                    std::process::exit(1);
                }
            }
        }
        ForgeAction::Sinks => {
            println!(
                "{}",
                json!({
                    "sinks": [
                        {"type": "stdout", "description": "Standard output"},
                        {"type": "parquet", "description": "Parquet file"},
                        {"type": "json", "description": "JSON file"},
                        {"type": "csv", "description": "CSV file"},
                        {"type": "postgres", "description": "PostgreSQL database"},
                        {"type": "mysql", "description": "MySQL database"},
                        {"type": "sqlite", "description": "SQLite database"},
                        {"type": "qdrant", "description": "Qdrant vector database"},
                    ]
                })
            );
        }
    }
}
