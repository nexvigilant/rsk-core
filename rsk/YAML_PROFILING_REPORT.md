# RSK YAML Parsing Profiling Report

**Date:** 2026-01-14
**Goal:** Profile actual RSK bridge calls to determine if YAML parsing is a bottleneck, and evaluate yaml-rust2 as an alternative to the deprecated serde_yaml.

---

## Executive Summary

### Key Findings

1. **YAML parsing IS a significant bottleneck** - It accounts for 92.5% of time in realistic skill discovery workloads
2. **RSK PyO3 bindings provide 11.88x speedup** over pure Python YAML parsing
3. **RSK CLI is slower than Python** due to subprocess overhead (0.52x slower)
4. **yaml-rust2 offers 10-35% performance improvement** over serde_yaml for raw parsing
5. **When converting to JSON, performance is nearly identical** between parsers

### Recommendations

| Priority | Action | Impact |
|----------|--------|--------|
| HIGH | Fix RSK bridge to use PyO3 for YAML parsing (not CLI) | 11.88x speedup |
| MEDIUM | Migrate from serde_yaml to yaml-rust2 | Future-proofing + 10-35% speedup |
| LOW | Consider lazy frontmatter parsing | Reduce unnecessary parsing |

---

## Part 1: RSK Python Bridge Profiling

### Methodology

Profiled 100 SKILL.md files with 5 iterations across 4 parsing backends:
- Pure Python (yaml.safe_load)
- RSK CLI (subprocess to rsk binary)
- RSK Bridge (auto-detect: PyO3 or CLI)
- RSK PyO3 (direct bindings)

### Results

| Backend | Avg Time (100 files) | Relative Performance |
|---------|---------------------|---------------------|
| Pure Python | 95.54ms | 1.00x (baseline) |
| RSK CLI | 183.63ms | 0.52x (SLOWER) |
| RSK Bridge (rust) | 192.38ms | 0.50x (SLOWER) |
| RSK PyO3 | 8.04ms | **11.88x FASTER** |

### Workload Analysis

Simulated realistic skill discovery workload:
- File I/O: 7.38ms (7.5%)
- YAML Parsing: 91.44ms (92.5%)
- String operations: negligible

**Conclusion:** YAML parsing dominates the workload. Optimization is warranted.

### cProfile Analysis (Top Functions)

```
ncalls  tottime  percall  cumtime  percall filename:lineno(function)
1000    0.001    0.000    2.907    0.003 yaml/__init__.py:117(safe_load)
482180  0.182    0.000    1.954    0.000 yaml/scanner.py:113(check_token)
193810  0.060    0.000    2.471    0.000 yaml/parser.py:94(check_event)
```

The Python YAML library spends most time in:
1. Token scanning (40%)
2. Event checking (25%)
3. Node composition (20%)

### Critical Issue Identified

The RSK bridge is falling back to CLI instead of PyO3:
- Bridge detects `backend: rust` but routes through subprocess
- Subprocess overhead (process spawn, JSON serialization) negates Rust speedup
- **Fix:** Ensure PyO3 bindings are used for `parse_yaml_string()` and `parse_frontmatter_file()`

---

## Part 2: yaml-rust2 Evaluation

### Background

serde_yaml is deprecated and will no longer receive updates. yaml-rust2 is the recommended replacement:

| Attribute | serde_yaml | yaml-rust2 |
|-----------|------------|------------|
| License | MIT/Apache-2.0 | MIT/Apache-2.0 |
| YAML Spec | 1.2 | 1.2 (fully compliant) |
| Status | **Deprecated** | Actively maintained |
| Last Release | 2024 | Sept 2025 (v0.10.4) |
| Downstream Users | ~15,000 | ~4,600 |
| Security | No code execution | No code execution |

### Benchmark Results

#### Raw Parsing Performance

| Workload | serde_yaml | yaml-rust2 | Improvement |
|----------|-----------|------------|-------------|
| Small frontmatter (145 bytes) | 4.62 µs | 3.42 µs | **35% faster** |
| Medium frontmatter (894 bytes) | 26.56 µs | 21.74 µs | **22% faster** |
| Deeply nested (322 bytes) | 8.75 µs | 8.44 µs | **4% faster** |
| Batch (100 files) | 397.67 µs | 359.04 µs | **10% faster** |

#### Full Workflow (Parse + Convert to JSON)

| Workload | serde_yaml | yaml-rust2 | Improvement |
|----------|-----------|------------|-------------|
| Medium frontmatter | 25.57 µs | 25.59 µs | ~0% (equal) |
| Frontmatter extraction | 25.23 µs | 25.33 µs | ~0% (equal) |

### Analysis

1. **Raw parsing:** yaml-rust2 is consistently 10-35% faster
2. **With JSON conversion:** Performance equalizes due to conversion overhead
3. **API difference:** yaml-rust2 returns `Yaml` type requiring manual JSON conversion, while serde_yaml returns `serde_json::Value` directly

### Migration Considerations

**Pros:**
- Future-proofing (serde_yaml is deprecated)
- 10-35% faster raw parsing
- MIT/Apache-2.0 dual license (compatible)
- Active maintenance

**Cons:**
- Requires JSON conversion function (adds code complexity)
- No direct serde integration (can't deserialize to structs directly)
- For full workflow, no significant performance gain

**Alternative:** [serde-yaml-bw](https://github.com/bourumir-wyngs/serde-yaml-bw) - A maintained fork of serde_yaml with serde integration preserved.

---

## Recommendations

### Immediate Action (HIGH PRIORITY)

**Fix RSK Bridge PyO3 routing**

The current bridge shows `backend: rust` but still uses subprocess. Verify:

```python
# In rsk_bridge.py, ensure PyO3 is preferred for YAML operations
if _USE_RUST:
    return _rsk.parse_yaml_string(content)  # This path should be taken
```

Expected improvement: **11.88x speedup** (8ms vs 95ms for 100 files)

### Medium-Term (MEDIUM PRIORITY)

**Migrate to yaml-rust2 or serde-yaml-bw**

Options:
1. **yaml-rust2**: Maximum performance, requires manual JSON conversion
2. **serde-yaml-bw**: Drop-in replacement for serde_yaml, maintained fork

Recommended: Start with **serde-yaml-bw** for minimal code changes, evaluate yaml-rust2 for hot paths.

### Long-Term (LOW PRIORITY)

**Lazy Frontmatter Parsing**

Instead of parsing all SKILL.md files upfront:
1. Cache parsed frontmatter to disk
2. Only re-parse when file modified (check mtime)
3. Use binary serialization (MessagePack/bincode) for cache

---

## Files Created

| File | Purpose |
|------|---------|
| `profile_yaml_bridge.py` | Python profiling script for RSK bridge |
| `benches/yaml_parsers.rs` | Criterion benchmarks for serde_yaml vs yaml-rust2 |
| `profile_results.json` | Raw profiling data |
| `YAML_PROFILING_REPORT.md` | This report |

---

## Sources

- [serde_yaml deprecation discussion](https://users.rust-lang.org/t/serde-yaml-deprecation-alternatives/108868)
- [yaml-rust2 on crates.io](https://crates.io/crates/yaml-rust2)
- [yaml-rust2 on GitHub](https://github.com/Ethiraric/yaml-rust2)
- [serde-yaml-bw fork](https://github.com/bourumir-wyngs/serde-yaml-bw)
