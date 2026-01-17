# Gemini Rust Workspace

Establishing a robust Rust development environment for exponential capability development.

## Project Structure

- **rsk (Rust Skill Kernel)**: High-performance computation kernel for Claude Code skills.
  - `decision_engine`: Deterministic execution engine for logic trees.
  - `skill_registry`: Automated discovery and routing for 250+ skills.
  - `intrinsics`: High-performance Rust implementations of core algorithms.
  - `python_bindings`: Seamless integration with existing Python runtime via PyO3.
- **rust-forge**: Pipeline compilation framework for high-throughput data processing.

## Current State

- **Unit Tests**: 300+ passing.
- **Integration Tests**: 16 passing (including full skill migration flows).
- **Toolchain**: Pinned to Rust 1.92.0.
- **Migration Progress**: 255 skills analyzed; `is-prime` migrated to 100% Rust deterministic execution.

## Usage

### Skill Registry & Execution
```bash
# Scan all skills
rsk skills scan ~/.claude/skills --output registry.json

# List skills with deterministic logic
rsk skills list --registry registry.json --strategy Deterministic

# Execute a skill natively in Rust
rsk skills execute is-prime --input '{"n": 17}' --registry registry.json
```

### Python Integration
```python
import rsk

# Generate logic from Markdown
logic = rsk.generate_logic(skill_md_content)

# Execute logic tree
result = rsk.execute_logic(logic, {"input": "data"})
```

## Performance
- **Decision Evaluation**: < 0.1ms per node.
- **Topological Sort**: 60x faster than Python.
- **Levenshtein**: 63x faster than Python.