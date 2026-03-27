# rsk-core

Microgram decision tree runtime and chain composition engine. Sub-millisecond deterministic decision programs with built-in self-testing.

## Workspace

| Crate | What |
|-------|------|
| `rsk` | CLI binary + library: microgram runtime, decision engine, chain executor, graph ops, statistics |
| `rsk-mcp` | MCP server exposing 16 tools for AI agent consumption (stdio transport) |

## Quick Start

```bash
# Build
cargo build -p rsk --release

# Run a microgram
./target/release/rsk mcg run rsk/micrograms/prr-signal.yaml -i '{"a": 50, "b": 1000, "c": 200, "d": 50000}'

# Self-test all micrograms
./target/release/rsk mcg test-all rsk/micrograms

# Test chain definitions
./target/release/rsk mcg chain-test rsk/chains
```

## Micrograms

460 atomic decision programs in `rsk/micrograms/`. Each is a YAML file with a decision tree, typed interface, and embedded test cases. Sub-microsecond execution.

```
rsk/micrograms/
  ├── *.yaml          (279 top-level programs)
  ├── academy/        (2)
  ├── dd/             (3)
  ├── dev/            (3)
  ├── flywheel/       (41)
  ├── pdc/            (92)
  └── station/        (46)
```

34 chain definitions in `rsk/chains/` compose micrograms into multi-step workflows.

## Testing

```bash
cargo test -p rsk --lib              # 559 unit tests
./target/release/rsk mcg test-all rsk/micrograms  # 5113 microgram self-tests
./target/release/rsk mcg chain-test rsk/chains     # Chain integration tests
cargo bench -p rsk                   # Criterion benchmarks
```

## MCP Server

`rsk-mcp` exposes the runtime via MCP (stdio transport, rmcp SDK):

- `mcg_run`, `mcg_test`, `mcg_test_all`, `mcg_chain`, `mcg_chain_test`, `mcg_list`, `mcg_info`, `mcg_coverage`
- `stats_chi_square`, `stats_t_test`, `stats_proportion_test`, `stats_correlation`
- `decision_tree_run`
- `graph_topsort`, `graph_parallel_levels`
- `rsk_health`

## Performance

| Operation | Latency |
|-----------|---------|
| Microgram evaluation | < 0.1ms per node |
| Topological sort | 60x faster than Python |
| Levenshtein distance | 63x faster than Python |

## Toolchain

Rust 1.85+ (Edition 2024). Strict clippy: `unwrap_used`, `expect_used`, `as_conversions` denied.

## License

MIT License. Copyright (c) 2026 Matthew Campion / NexVigilant.
