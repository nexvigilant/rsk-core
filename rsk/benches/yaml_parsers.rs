//! Benchmarks comparing YAML parsers for RSK.
//!
//! Compares:
//! - serde_yaml (current implementation, deprecated)
//! - yaml-rust2 (MIT/Apache-2.0, actively maintained alternative)
//!
//! Run with: cargo bench --bench yaml_parsers --features yaml-rust2
//!
//! Performance targets for skill frontmatter parsing:
//! - Small frontmatter (10 keys): < 10us
//! - Medium frontmatter (50 keys): < 50us
//! - Complex frontmatter (nested): < 100us

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

/// Sample YAML content representing typical SKILL.md frontmatter
const SMALL_FRONTMATTER: &str = r#"
name: test-skill
version: "1.0.0"
compliance-level: Gold
description: A test skill for benchmarking
triggers:
  - pattern: test
  - pattern: benchmark
"#;

const MEDIUM_FRONTMATTER: &str = r#"
name: complex-skill
version: "2.0.0"
compliance-level: Diamond
description: A complex skill with many fields
author: benchmarks
created: 2025-01-01
updated: 2025-06-15
category: testing
subcategory: performance
tags:
  - benchmark
  - performance
  - yaml
  - parsing
triggers:
  - pattern: complex
    weight: 1.0
  - pattern: benchmark
    weight: 0.8
  - pattern: test
    weight: 0.5
inputs:
  - name: input1
    type: string
    required: true
  - name: input2
    type: number
    required: false
    default: 0
outputs:
  - name: result
    type: object
    schema: ResultSchema
invariants:
  - description: "Input must be valid"
    condition: "input1.length > 0"
  - description: "Output must be non-null"
    condition: "result != null"
failure_modes:
  - id: FM-1
    description: Invalid input
    severity: error
  - id: FM-2
    description: Timeout
    severity: warning
dependencies:
  - rust: "1.75"
  - python: "3.12"
"#;

const DEEPLY_NESTED: &str = r#"
root:
  level1:
    level2:
      level3:
        level4:
          level5:
            value: deep
            array:
              - item1
              - item2
              - nested:
                  key: value
    other:
      - a
      - b
      - c
  config:
    settings:
      enabled: true
      timeout: 30
      retries: 3
"#;

/// Benchmark serde_yaml parsing
fn bench_serde_yaml(c: &mut Criterion) {
    let mut group = c.benchmark_group("serde_yaml");

    // Small frontmatter
    group.throughput(Throughput::Bytes(SMALL_FRONTMATTER.len() as u64));
    group.bench_function("small_frontmatter", |b| {
        b.iter(|| {
            let result: serde_json::Value =
                serde_yaml::from_str(black_box(SMALL_FRONTMATTER)).unwrap();
            black_box(result)
        })
    });

    // Medium frontmatter
    group.throughput(Throughput::Bytes(MEDIUM_FRONTMATTER.len() as u64));
    group.bench_function("medium_frontmatter", |b| {
        b.iter(|| {
            let result: serde_json::Value =
                serde_yaml::from_str(black_box(MEDIUM_FRONTMATTER)).unwrap();
            black_box(result)
        })
    });

    // Deeply nested
    group.throughput(Throughput::Bytes(DEEPLY_NESTED.len() as u64));
    group.bench_function("deeply_nested", |b| {
        b.iter(|| {
            let result: serde_json::Value =
                serde_yaml::from_str(black_box(DEEPLY_NESTED)).unwrap();
            black_box(result)
        })
    });

    group.finish();
}

/// Benchmark yaml-rust2 parsing (requires yaml-rust2 feature)
#[cfg(feature = "yaml-rust2")]
fn bench_yaml_rust2(c: &mut Criterion) {
    use yaml_rust2::YamlLoader;

    let mut group = c.benchmark_group("yaml_rust2");

    // Small frontmatter
    group.throughput(Throughput::Bytes(SMALL_FRONTMATTER.len() as u64));
    group.bench_function("small_frontmatter", |b| {
        b.iter(|| {
            let docs = YamlLoader::load_from_str(black_box(SMALL_FRONTMATTER)).unwrap();
            black_box(docs)
        })
    });

    // Medium frontmatter
    group.throughput(Throughput::Bytes(MEDIUM_FRONTMATTER.len() as u64));
    group.bench_function("medium_frontmatter", |b| {
        b.iter(|| {
            let docs = YamlLoader::load_from_str(black_box(MEDIUM_FRONTMATTER)).unwrap();
            black_box(docs)
        })
    });

    // Deeply nested
    group.throughput(Throughput::Bytes(DEEPLY_NESTED.len() as u64));
    group.bench_function("deeply_nested", |b| {
        b.iter(|| {
            let docs = YamlLoader::load_from_str(black_box(DEEPLY_NESTED)).unwrap();
            black_box(docs)
        })
    });

    group.finish();
}

/// Benchmark comparison: parsing + JSON conversion
/// (measures complete workflow including conversion to serde_json::Value)
fn bench_full_workflow(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_workflow");

    // serde_yaml full workflow
    group.bench_function("serde_yaml_to_json", |b| {
        b.iter(|| {
            let result: serde_json::Value =
                serde_yaml::from_str(black_box(MEDIUM_FRONTMATTER)).unwrap();
            // Simulate depth calculation
            let depth = calculate_depth(&result);
            black_box((result, depth))
        })
    });

    // yaml-rust2 full workflow (requires conversion to JSON)
    #[cfg(feature = "yaml-rust2")]
    {
        use yaml_rust2::YamlLoader;

        group.bench_function("yaml_rust2_to_json", |b| {
            b.iter(|| {
                let docs = YamlLoader::load_from_str(black_box(MEDIUM_FRONTMATTER)).unwrap();
                let json = yaml_to_json(&docs[0]);
                let depth = calculate_depth(&json);
                black_box((json, depth))
            })
        });
    }

    group.finish();
}

/// Benchmark frontmatter extraction (real-world scenario)
fn bench_frontmatter_extraction(c: &mut Criterion) {
    let skill_md = format!(
        "---\n{}\n---\n\n# Test Skill\n\nMarkdown content here...\n",
        MEDIUM_FRONTMATTER
    );

    let mut group = c.benchmark_group("frontmatter_extraction");

    group.bench_function("serde_yaml", |b| {
        b.iter(|| {
            let content = black_box(&skill_md);
            if content.starts_with("---") {
                let parts: Vec<&str> = content.splitn(3, "---").collect();
                if parts.len() >= 3 {
                    let result: serde_json::Value = serde_yaml::from_str(parts[1]).unwrap();
                    return black_box(result);
                }
            }
            black_box(serde_json::Value::Null)
        })
    });

    #[cfg(feature = "yaml-rust2")]
    {
        use yaml_rust2::YamlLoader;

        group.bench_function("yaml_rust2", |b| {
            b.iter(|| {
                let content = black_box(&skill_md);
                if content.starts_with("---") {
                    let parts: Vec<&str> = content.splitn(3, "---").collect();
                    if parts.len() >= 3 {
                        let docs = YamlLoader::load_from_str(parts[1]).unwrap();
                        let json = yaml_to_json(&docs[0]);
                        return black_box(json);
                    }
                }
                black_box(serde_json::Value::Null)
            })
        });
    }

    group.finish();
}

/// Benchmark batch parsing (100 files scenario)
fn bench_batch_parsing(c: &mut Criterion) {
    let files: Vec<String> = (0..100)
        .map(|i| {
            format!(
                r#"
name: skill-{}
version: "1.0.0"
compliance-level: Gold
description: Skill number {} for benchmarking
triggers:
  - pattern: skill{}
"#,
                i, i, i
            )
        })
        .collect();

    let mut group = c.benchmark_group("batch_parsing_100");

    group.bench_function("serde_yaml", |b| {
        b.iter(|| {
            let mut results = Vec::with_capacity(100);
            for content in black_box(&files) {
                let result: serde_json::Value = serde_yaml::from_str(content).unwrap();
                results.push(result);
            }
            black_box(results)
        })
    });

    #[cfg(feature = "yaml-rust2")]
    {
        use yaml_rust2::YamlLoader;

        group.bench_function("yaml_rust2", |b| {
            b.iter(|| {
                let mut results = Vec::with_capacity(100);
                for content in black_box(&files) {
                    let docs = YamlLoader::load_from_str(content).unwrap();
                    let json = yaml_to_json(&docs[0]);
                    results.push(json);
                }
                black_box(results)
            })
        });
    }

    group.finish();
}

/// Simulation of realistic skill discovery workload (read -> parse -> trigger match)
fn bench_skill_discovery_simulation(c: &mut Criterion) {
    let files: Vec<String> = (0..50)
        .map(|i| {
            format!(
                r#"---
name: skill-{}
version: "1.0.0"
compliance-level: Gold
description: A realistic skill for discovery simulation
triggers:
  - pattern: trigger-{}
  - pattern: common-trigger
---
# Skill Content {}
"#,
                i, i, i
            )
        })
        .collect();

    let mut group = c.benchmark_group("skill_discovery_sim");

    group.bench_function("discovery_sim_50_files", |b| {
        b.iter(|| {
            let mut matched_count = 0;
            for content in black_box(&files) {
                // 1. Regex find frontmatter
                if content.starts_with("---") {
                    let parts: Vec<&str> = content.splitn(3, "---").collect();
                    if parts.len() >= 3 {
                        // 2. Parse YAML
                        let result: serde_json::Value = serde_yaml::from_str(parts[1]).unwrap();
                        
                        // 3. String operation (simulated trigger match)
                        if let Some(triggers) = result.get("triggers").and_then(|t| t.as_array()) {
                            for t in triggers {
                                if let Some(pattern) = t.get("pattern").and_then(|p| p.as_str()) {
                                    if pattern.contains("common") {
                                        matched_count += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            black_box(matched_count)
        })
    });

    group.finish();
}

// Helper functions

fn calculate_depth(value: &serde_json::Value) -> usize {
    match value {
        serde_json::Value::Object(map) => 1 + map.values().map(calculate_depth).max().unwrap_or(0),
        serde_json::Value::Array(arr) => 1 + arr.iter().map(calculate_depth).max().unwrap_or(0),
        _ => 0,
    }
}

#[cfg(feature = "yaml-rust2")]
fn yaml_to_json(yaml: &yaml_rust2::Yaml) -> serde_json::Value {
    use yaml_rust2::Yaml;

    match yaml {
        Yaml::Real(s) => s
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::String(s.clone())),
        Yaml::Integer(i) => serde_json::Value::Number((*i).into()),
        Yaml::String(s) => serde_json::Value::String(s.clone()),
        Yaml::Boolean(b) => serde_json::Value::Bool(*b),
        Yaml::Array(arr) => serde_json::Value::Array(arr.iter().map(yaml_to_json).collect()),
        Yaml::Hash(hash) => {
            let map: serde_json::Map<String, serde_json::Value> = hash
                .iter()
                .filter_map(|(k, v)| {
                    let key = match k {
                        Yaml::String(s) => s.clone(),
                        Yaml::Integer(i) => i.to_string(),
                        _ => return None,
                    };
                    Some((key, yaml_to_json(v)))
                })
                .collect();
            serde_json::Value::Object(map)
        }
        Yaml::Null => serde_json::Value::Null,
        Yaml::BadValue => serde_json::Value::Null,
        Yaml::Alias(_) => serde_json::Value::Null,
    }
}

#[cfg(feature = "yaml-rust2")]
criterion_group!(
    benches,
    bench_serde_yaml,
    bench_yaml_rust2,
    bench_full_workflow,
    bench_frontmatter_extraction,
    bench_batch_parsing,
    bench_skill_discovery_simulation,
);

#[cfg(not(feature = "yaml-rust2"))]
criterion_group!(
    benches,
    bench_serde_yaml,
    bench_full_workflow,
    bench_frontmatter_extraction,
    bench_batch_parsing,
    bench_skill_discovery_simulation,
);

criterion_main!(benches);
