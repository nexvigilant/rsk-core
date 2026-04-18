# rsk-core

Microgram decision tree runtime for pharmacovigilance.

Sub-microsecond deterministic decision programs with built-in self-testing, chain composition, and MCP server integration. Part of the [NexVigilant](https://nexvigilant.com) stack powering [mcp.nexvigilant.com](https://mcp.nexvigilant.com).

## At a Glance

| Metric | Count |
|--------|-------|
| Micrograms | 1,587 |
| Heligrams | 673 |
| Chains | 200 |
| Self-tests | 11,117 |

*Last measured: 2026-04-16. Run `./target/release/rsk mcg count` to re-measure.*

## Quick Start

```bash
cargo build -p rsk --release
./target/release/rsk mcg test-all rsk/micrograms
```

## Key Commands

```bash
# Run a microgram with input
./target/release/rsk mcg run rsk/micrograms/prr-signal.yaml -i '{"a": 50, "b": 1000, "c": 200, "d": 50000}'

# Self-test a single microgram
./target/release/rsk mcg test rsk/micrograms/naranjo-quick.yaml

# Run a chain
./target/release/rsk mcg chain "prr-signal -> signal-to-causality" -d rsk/micrograms -i '{"a": 50, "b": 1000, "c": 200, "d": 50000}'

# Test all chain definitions
./target/release/rsk mcg chain-test rsk/chains

# Forge a heligram
./target/release/rsk heligram forge rsk/heligrams/my-heligram.yaml

# Test all heligrams
./target/release/rsk heligram test-all rsk/heligrams
```

## Workspace

| Crate | What |
|-------|------|
| `rsk` | CLI binary + library: microgram runtime, decision engine, chain executor, graph ops, statistics |
| `rsk-mcp` | MCP server exposing 16 tools for AI agent consumption (stdio transport) |

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

NexVigilant Source Available License v1.0. Copyright (c) 2026 Matthew Campion / NexVigilant.

Personal non-commercial use only. Organizational use requires written permission from matthew@camp-corp.com. See [LICENSE](LICENSE) for full terms.

## Contributing

See the NexVigilant [contributing guidelines](https://github.com/nexvigilant/.github/blob/main/CONTRIBUTING.md). Quality gates: `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test -p rsk --lib`.
