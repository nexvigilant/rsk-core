# RSK - Rust Skill Kernel

High-performance computation kernel for Claude Code skills.

## Overview

RSK provides Rust-native implementations for common skill operations, achieving 10-100x performance improvements over Python equivalents.

## Installation

```bash
cargo install --path .
# Or: cp target/release/rsk ~/.local/bin/
```

## Modules

| Module | Purpose | Key Functions |
|--------|---------|---------------|
| **math** | Statistics | `calculate_variance()` |
| **graph** | DAG ops | `topological_sort()`, `level_parallelization()` |
| **text_processor** | SKILL.md | `parse_skill_md()`, `extract_smst()` |
| **levenshtein** | Fuzzy match | `levenshtein()`, `fuzzy_search()` |
| **crypto** | Hashing | `sha256_hash()`, `sha256_verify()` |
| **code_generator** | Codegen | `generate_validation_rules()`, `generate_rust_stub()` |
| **yaml_processor** | Config | `parse_yaml()`, `parse_toml()`, `validate_schema()` |

## CLI Commands

```text
# Validation
rsk verify <path>                 Diamond v2 validation
rsk yaml validate <path>          Schema validation

# Parsing
rsk text parse <path>             Parse SKILL.md structure
rsk text smst <path>              Extract SMST with scoring
rsk yaml parse <path>             YAML to JSON
rsk yaml toml <path>              TOML to JSON
rsk yaml frontmatter <path>       Parse frontmatter

# Code Generation
rsk generate rules <path>         Validation rules from SMST
rsk generate tests <path>         Test scaffolds
rsk generate stub <path>          Rust module stub

# Algorithms
rsk levenshtein <src> <tgt>       Edit distance (63x faster)
rsk fuzzy <query> --candidates    Fuzzy search
rsk graph topsort --input <json>  Topological sort
rsk graph levels --input <json>   Parallel execution groups
rsk sha256 hash <input>           SHA-256 hash

# Analysis
rsk yaml decision-tree <path>     Analyze decision tree YAML
rsk yaml taxonomy <path>          Extract taxonomy schema
rsk build <path>                  Analyze build artifacts
```

## Performance

| Operation | Python | Rust | Speedup |
|-----------|--------|------|---------|
| Levenshtein | 63ms | 1ms | **63x** |
| SMST Parse | 50ms | 5ms | **10x** |
| SHA-256 | 10ms | 0.5ms | **20x** |
| YAML Parse | 15ms | 2ms | **7x** |

## Library Usage

```rust
use rsk::{extract_smst, levenshtein, parse_yaml};

// Parse and validate SKILL.md
let content = std::fs::read_to_string("SKILL.md")?;
let smst = extract_smst(&content);
println!("Diamond score: {:.1}%", smst.score.total_score);

// Fuzzy string matching
let result = levenshtein("kitten", "sitting");
println!("Edit distance: {}", result.distance);

// Parse YAML config
let yaml = "name: test\nversion: '1.0'";
let parsed = parse_yaml(yaml)?;
println!("Keys: {:?}", parsed.keys);
```

## Test Coverage

```bash
cargo test          # 168 tests (154 unit + 14 integration)
cargo test -- --nocapture  # With output
```

## Python Bindings (PyPI)

Install from PyPI:

```bash
pip install rsk
```

Usage:

```python
import rsk

# String similarity
result = rsk.levenshtein("hello", "hallo")
print(f"Distance: {result['distance']}, Similarity: {result['similarity']}")

# Fuzzy search
matches = rsk.fuzzy_search("test", ["test", "testing", "best", "rest"])
print(f"Matches: {matches}")

# SHA-256 hashing
hash_result = rsk.sha256("my data")
print(f"Hash: {hash_result['hex']}")

# SKILL.md parsing
smst = rsk.extract_smst(open("SKILL.md").read())
print(f"Score: {smst['score']['total_score']}")

# Frontmatter extraction
frontmatter = rsk.parse_frontmatter(open("SKILL.md").read())
print(f"Name: {frontmatter.get('name')}")
```

### Available Python Functions

| Function | Description |
|----------|-------------|
| `levenshtein(src, tgt)` | Edit distance with similarity score |
| `fuzzy_search(query, candidates)` | Find closest matches |
| `sha256(data)` | SHA-256 hash |
| `sha256_verify(data, expected)` | Verify hash |
| `variance(numbers)` | Calculate variance |
| `extract_smst(content)` | Parse SKILL.md SMST |
| `parse_frontmatter(content)` | Extract YAML frontmatter |
| `query_taxonomy(name, key)` | Query taxonomy data |
| `list_taxonomy(name)` | List taxonomy entries |

## Version History

- **0.4.0** - Python bindings via PyO3, VerifyBase/BuildBase RSK integration
- **0.3.0** - Added yaml_processor module (YAML/TOML parsing)
- **0.2.0** - Added code_generator, level_parallelization
- **0.1.0** - Initial release (math, graph, text, levenshtein, crypto)

## License

MIT
