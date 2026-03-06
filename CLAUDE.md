# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## rsk-core

Single-crate Rust workspace producing a CLI binary (`rsk`) — microgram decision tree runtime and chain composition engine.

## Build & Test

```bash
cargo build -p rsk --release                        # Build binary (required before mcg commands)
cargo test -p rsk --lib                              # Unit tests
cargo test -p rsk -- test_name                       # Run a single test by name
cargo test -p rsk --tests                            # Integration tests
cargo clippy -p rsk -- -D warnings                   # Lint
cargo bench -p rsk                                   # Benchmarks (criterion)

# Microgram operations (require release binary)
./target/release/rsk mcg test-all rsk/micrograms     # Self-test all micrograms
./target/release/rsk mcg test <path.yaml>            # Self-test one microgram
./target/release/rsk mcg run <path.yaml> -i '<json>' # Execute with input
./target/release/rsk mcg chain "a -> b" -d rsk/micrograms -i '<json>'  # Run chain inline
./target/release/rsk mcg chain-test rsk/chains                         # Test all chain definitions
./target/release/rsk mcg generate <name> -d "<desc>" -v <var> -o <op> -t <val>  # New microgram
```

## Architecture

**Binary entry:** `rsk/src/bin/rsk_dev.rs` → CLI router at `rsk/src/cli/mod.rs` (clap)

**Two key layers:**

1. **CLI layer** (`rsk/src/cli/`) — `actions.rs` defines clap subcommand enums, `handlers/` has one file per subcommand (microgram, guardian, graph, yaml, etc.)

2. **Module layer** (`rsk/src/modules/`) — all re-exported via `lib.rs`. Six subdirectory modules: `chain/`, `guardian/`, `hooks/`, `microgram/`, `text_processor/`, `tov/`.

**Critical modules:**

| Module | Role |
|--------|------|
| `decision_engine.rs` | Core type system: `Value` enum (Null/Bool/Int/Float/String/Array/Object), `Operator` enum, tree node evaluation |
| `execution_engine.rs` | DAG-based execution planner with checkpointing (not the microgram runtime) |
| `microgram/` | Microgram runtime: load/run/test/chain/evolve/stress/compose/coverage/pipe/snapshot |
| `microgram/chain.rs` | Chain executor: `chain()`, `chain_accumulate()`, `chain_resilient()` |
| `yaml_processor.rs` | YAML/TOML parsing, schema validation, frontmatter extraction |
| `graph.rs` | DAG operations: topsort, parallel levels, shortest path |
| `guardian/` | Risk scoring, signal detection, IAIR incident reports |

**Data flow for microgram execution:**

```
YAML file → Microgram::load() → Microgram struct (tree + tests + interface)
                                      ↓
Input JSON → HashMap<String, Value> → mg.run(variables) → RunResult { output, duration_us }
                                      ↓
Chain mode: step₁.output merges into step₂.input (accumulate: true preserves all fields)
```

**Adding a new CLI subcommand:** Add variant to `cli/actions.rs`, create handler in `cli/handlers/`, wire in `cli/handlers/mod.rs`.

## Microgram Conventions

- Operators: `eq`, `neq`, `gt`, `gte`, `lt`, `lte`, `contains`, `not_contains`, `matches`, `is_null`, `is_not_null`
- Null safety: missing/null inputs must produce safe defaults. Every microgram needs a test with `input: {}`
- Chains reference micrograms by **name** (not path), resolved from `micrograms_dir`
- `accumulate: true` in chains preserves all upstream outputs; without it, only the last step's output passes forward

## Conventions

- `anyhow::Error` for binary/CLI, `thiserror` for library errors
- All CLI output is JSON (for machine consumption by MCP tools and Claude Code)

## Key Gotchas

- Binary is at `./target/release/rsk` — must `cargo build -p rsk --release` first
- Microgram `test-all` and `chain-test` use the **release binary**, not `cargo test`
- `python_bindings` module requires `python` feature flag (disabled by default)
- Microgram subdirectories (`pdc/`, `flywheel/`, `dev/`) are scanned recursively by `test-all`
