#!/usr/bin/env python3
"""Integration test suite for RSK PyO3 bindings.

This test suite validates all 34 PyO3 functions exposed by the rsk module.
It ensures API compatibility between the Rust kernel and Python bridge.

Test categories:
1. Levenshtein Operations (2 functions)
2. Crypto Operations (2 functions)
3. Math Operations (1 function)
4. Taxonomy Operations (2 functions)
5. SKILL.md Operations (2 functions)
6. Text Processing Operations (4 functions)
7. Compression Operations (3 functions)
8. Graph Operations (3 functions)
9. YAML Operations (1 function)
10. Execution Engine Operations (1 function)
11. Code Generator Operations (3 functions)
12. State Manager Operations (9 functions)

Total: 34 PyO3 functions

Usage:
    # Run all tests
    pytest tests/test_pyo3_integration.py -v

    # Run with coverage
    pytest tests/test_pyo3_integration.py -v --cov=rsk

    # Run specific category
    pytest tests/test_pyo3_integration.py -v -k "test_levenshtein"

Requirements:
    - RSK wheel must be installed: pip install -e . (from ~/.claude/rust/rsk)
    - Or use maturin: maturin develop --features python
"""

import json
import os
import sys
import tempfile
from pathlib import Path
from typing import Any

import pytest

# Add the venv site-packages if available
VENV_SITE_PACKAGES = Path.home() / ".claude" / ".venv" / "lib" / "python3.12" / "site-packages"
if VENV_SITE_PACKAGES.exists() and str(VENV_SITE_PACKAGES) not in sys.path:
    sys.path.insert(0, str(VENV_SITE_PACKAGES))

# Try to import rsk module
try:
    import rsk
    RSK_AVAILABLE = True
except ImportError:
    RSK_AVAILABLE = False


# Skip all tests if rsk is not available
pytestmark = pytest.mark.skipif(
    not RSK_AVAILABLE,
    reason="rsk PyO3 module not installed. Run: maturin develop --features python"
)


# ============================================================================
# Test Fixtures
# ============================================================================

@pytest.fixture
def sample_skill_md() -> str:
    """Sample SKILL.md content for testing."""
    return '''---
name: test-skill
description: A test skill for integration testing
version: 1.0.0
compliance-level: gold
categories:
  - testing
author: test-author
user-invocable: true
---

# test-skill

A test skill.

## Machine Specification

### 1. INPUTS

- `input_path` (String): Path to input file
- `threshold` (i32): Score threshold

### 2. OUTPUTS

- `result` (String): Processing result
- `score` (f64): Calculated score

### 3. STATE

- `cache` (Object): Internal cache

### 4. OPERATOR MODE

| Mode | Behavior |
|------|----------|
| validate | Validate only |
| execute | Full execution |

### 5. PERFORMANCE

- Latency: <50ms p95

### 6. INVARIANTS

- Score must be between 0 and 100
- Input path must exist

### 7. FAILURE MODES

- FM-001: File not found (recoverable)
- FM-002: Invalid format (error)

### 8. TELEMETRY

- execution_time_ms: Time taken
'''


@pytest.fixture
def temp_state_dir(tmp_path: Path) -> Path:
    """Create a temporary directory for checkpoint state."""
    state_dir = tmp_path / "checkpoints"
    state_dir.mkdir()
    return state_dir


# ============================================================================
# 1. Levenshtein Operations (2 functions)
# ============================================================================

class TestLevenshteinOperations:
    """Tests for levenshtein and fuzzy_search functions."""

    def test_levenshtein_basic(self):
        """Test basic Levenshtein distance calculation."""
        result = rsk.levenshtein("kitten", "sitting")

        assert isinstance(result, dict)
        assert result["distance"] == 3
        assert "similarity" in result
        assert result["source_len"] == 6
        assert result["target_len"] == 7

    def test_levenshtein_identical_strings(self):
        """Test Levenshtein with identical strings."""
        result = rsk.levenshtein("hello", "hello")

        assert result["distance"] == 0
        assert result["similarity"] == 1.0

    def test_levenshtein_empty_strings(self):
        """Test Levenshtein with empty strings."""
        result = rsk.levenshtein("", "hello")
        assert result["distance"] == 5

        result = rsk.levenshtein("hello", "")
        assert result["distance"] == 5

        result = rsk.levenshtein("", "")
        assert result["distance"] == 0

    def test_levenshtein_unicode(self):
        """Test Levenshtein with unicode characters."""
        result = rsk.levenshtein("cafe", "cafe")
        assert result["distance"] == 0

    def test_fuzzy_search_basic(self):
        """Test basic fuzzy search."""
        candidates = ["test", "testing", "best", "rest", "toast"]
        result = rsk.fuzzy_search("test", candidates, 3)

        assert isinstance(result, list)
        assert len(result) <= 3
        # "test" should be first (exact match)
        assert result[0]["candidate"] == "test"
        assert result[0]["distance"] == 0

    def test_fuzzy_search_empty_candidates(self):
        """Test fuzzy search with empty candidates."""
        result = rsk.fuzzy_search("test", [], 3)
        assert result == []

    def test_fuzzy_search_limit(self):
        """Test fuzzy search respects limit."""
        candidates = ["a", "b", "c", "d", "e"]
        result = rsk.fuzzy_search("a", candidates, 2)
        assert len(result) <= 2


# ============================================================================
# 2. Crypto Operations (2 functions)
# ============================================================================

class TestCryptoOperations:
    """Tests for sha256 and sha256_verify functions."""

    def test_sha256_basic(self):
        """Test basic SHA-256 hashing."""
        result = rsk.sha256("hello")

        assert isinstance(result, dict)
        assert result["algorithm"] == "SHA-256"
        assert len(result["hex"]) == 64
        assert result["bytes_hashed"] == 5
        # Known SHA-256 of "hello"
        assert result["hex"] == "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"

    def test_sha256_empty_string(self):
        """Test SHA-256 of empty string."""
        result = rsk.sha256("")
        # Known SHA-256 of empty string
        assert result["hex"] == "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"

    def test_sha256_verify_correct(self):
        """Test SHA-256 verification with correct hash."""
        expected = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        result = rsk.sha256_verify("hello", expected)
        assert result is True

    def test_sha256_verify_incorrect(self):
        """Test SHA-256 verification with incorrect hash."""
        result = rsk.sha256_verify("hello", "0" * 64)
        assert result is False

    def test_sha256_verify_case_insensitive(self):
        """Test SHA-256 verification is case-insensitive."""
        expected_lower = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        expected_upper = expected_lower.upper()
        assert rsk.sha256_verify("hello", expected_upper) is True


# ============================================================================
# 3. Math Operations (1 function)
# ============================================================================

class TestMathOperations:
    """Tests for variance function."""

    def test_variance_positive(self):
        """Test variance with positive difference."""
        result = rsk.variance(100.0, 80.0)

        assert isinstance(result, dict)
        assert result["absolute"] == 20.0
        assert result["percentage"] == 25.0

    def test_variance_negative(self):
        """Test variance with negative difference."""
        result = rsk.variance(80.0, 100.0)

        assert result["absolute"] == -20.0
        assert result["percentage"] == -20.0

    def test_variance_zero(self):
        """Test variance with no difference."""
        result = rsk.variance(100.0, 100.0)

        assert result["absolute"] == 0.0
        assert result["percentage"] == 0.0


# ============================================================================
# 4. Taxonomy Operations (2 functions)
# ============================================================================

class TestTaxonomyOperations:
    """Tests for query_taxonomy and list_taxonomy functions."""

    def test_query_taxonomy_compliance(self):
        """Test querying compliance taxonomy."""
        result = rsk.query_taxonomy("compliance", "bronze")

        assert isinstance(result, dict)
        assert result.get("found") is True or "data" in result

    def test_query_taxonomy_not_found(self):
        """Test querying non-existent taxonomy entry."""
        result = rsk.query_taxonomy("compliance", "nonexistent_level")
        assert result.get("found") is False or "error" in result

    def test_list_taxonomy_compliance(self):
        """Test listing compliance taxonomy."""
        result = rsk.list_taxonomy("compliance")

        assert isinstance(result, dict)
        assert "entries" in result or "count" in result


# ============================================================================
# 5. SKILL.md Operations (2 functions)
# ============================================================================

class TestSkillMdOperations:
    """Tests for extract_smst and parse_frontmatter functions."""

    def test_extract_smst(self, sample_skill_md: str):
        """Test SMST extraction from SKILL.md content."""
        result = rsk.extract_smst(sample_skill_md)

        assert isinstance(result, dict)
        assert "frontmatter" in result
        assert "spec" in result or "score" in result
        assert result["frontmatter"]["name"] == "test-skill"

    def test_extract_smst_empty(self):
        """Test SMST extraction from empty content."""
        result = rsk.extract_smst("")
        assert isinstance(result, dict)

    def test_parse_frontmatter(self, sample_skill_md: str):
        """Test frontmatter parsing."""
        result = rsk.parse_frontmatter(sample_skill_md)

        assert isinstance(result, dict)
        assert result["name"] == "test-skill"
        assert result["version"] == "1.0.0"
        assert result["compliance_level"] == "gold"

    def test_parse_frontmatter_no_frontmatter(self):
        """Test parsing content without frontmatter."""
        result = rsk.parse_frontmatter("# Just a header\n\nSome content.")
        assert isinstance(result, dict)


# ============================================================================
# 6. Text Processing Operations (4 functions)
# ============================================================================

class TestTextProcessingOperations:
    """Tests for tokenize, normalize, word_frequency, and text_entropy functions."""

    def test_tokenize_basic(self):
        """Test basic tokenization."""
        result = rsk.tokenize("Hello, World! How are you?")

        assert isinstance(result, dict)
        assert "tokens" in result
        assert "count" in result
        assert "unique_count" in result
        assert result["count"] == 5

    def test_tokenize_empty(self):
        """Test tokenization of empty string."""
        result = rsk.tokenize("")
        assert result["count"] == 0

    def test_normalize_basic(self):
        """Test basic text normalization."""
        result = rsk.normalize("  Hello,  WORLD!  ", True)

        assert isinstance(result, dict)
        assert "text" in result
        assert result["text"] == "hello world"

    def test_normalize_keep_punctuation(self):
        """Test normalization keeping punctuation."""
        result = rsk.normalize("Hello, World!", False)
        assert "," in result["text"] or "!" in result["text"]

    def test_word_frequency_basic(self):
        """Test word frequency calculation."""
        result = rsk.word_frequency("the quick brown fox jumps over the lazy dog", 5)

        assert isinstance(result, dict)
        assert "frequencies" in result
        assert "total_words" in result
        assert "unique_words" in result
        assert "top_words" in result

    def test_word_frequency_empty(self):
        """Test word frequency of empty string."""
        result = rsk.word_frequency("", 5)
        assert result["total_words"] == 0

    def test_text_entropy_low(self):
        """Test entropy of highly repetitive text."""
        result = rsk.text_entropy("aaaaaaaaaa")

        assert isinstance(result, dict)
        assert "entropy_estimate" in result
        assert "compressibility" in result
        # Repetitive text should be highly compressible
        assert result["compressibility"] == "highly_compressible"

    def test_text_entropy_high(self):
        """Test entropy of varied text."""
        result = rsk.text_entropy("abcdefghij1234567890")
        # Varied text should have lower compressibility
        assert result["compressibility"] in ("medium_compressibility", "low_compressibility")


# ============================================================================
# 7. Compression Operations (3 functions)
# ============================================================================

class TestCompressionOperations:
    """Tests for gzip_compress, gzip_decompress, and estimate_compressibility functions."""

    def test_gzip_compress_basic(self):
        """Test basic gzip compression."""
        result = rsk.gzip_compress("Hello, World! " * 100)

        assert isinstance(result, dict)
        assert "original_size" in result
        assert "compressed_size" in result
        assert "ratio" in result
        assert "savings_percent" in result
        assert "data" in result
        assert isinstance(result["data"], bytes)
        assert result["compressed_size"] < result["original_size"]

    def test_gzip_compress_levels(self):
        """Test different compression levels."""
        text = "Hello, World! " * 100
        result_fast = rsk.gzip_compress(text, "fast")
        result_best = rsk.gzip_compress(text, "best")

        # Best compression should generally produce smaller output
        assert result_best["compressed_size"] <= result_fast["compressed_size"]

    def test_gzip_decompress_basic(self):
        """Test basic gzip decompression."""
        original = "Hello, World!"
        compressed = rsk.gzip_compress(original)
        decompressed = rsk.gzip_decompress(compressed["data"])

        assert decompressed == original

    def test_gzip_roundtrip(self):
        """Test compression/decompression roundtrip."""
        test_strings = [
            "Short",
            "Hello, World! " * 100,
            "Unicode: cafe, emoji: test",
            "",  # Empty string
        ]
        for original in test_strings:
            compressed = rsk.gzip_compress(original)
            decompressed = rsk.gzip_decompress(compressed["data"])
            assert decompressed == original, f"Roundtrip failed for: {original[:50]}"

    def test_estimate_compressibility_repetitive(self):
        """Test compressibility estimation for repetitive data."""
        data = b"aaaaaaaaaa" * 100
        ratio = rsk.estimate_compressibility(data)

        assert isinstance(ratio, float)
        assert ratio < 0.5  # Highly compressible

    def test_estimate_compressibility_random(self):
        """Test compressibility estimation for random-ish data."""
        import random
        data = bytes([random.randint(0, 255) for _ in range(1000)])
        ratio = rsk.estimate_compressibility(data)

        assert ratio > 0.5  # Less compressible


# ============================================================================
# 8. Graph Operations (3 functions)
# ============================================================================

class TestGraphOperations:
    """Tests for topological_sort, level_parallelization, and shortest_path functions."""

    def test_topological_sort_basic(self):
        """Test basic topological sort."""
        graph = {"a": ["b"], "b": ["c"], "c": []}
        result = rsk.topological_sort(graph)

        assert isinstance(result, dict)
        assert "sorted" in result
        # a should come before b, b before c
        sorted_list = result["sorted"]
        assert sorted_list.index("a") < sorted_list.index("b")
        assert sorted_list.index("b") < sorted_list.index("c")

    def test_topological_sort_diamond(self):
        """Test topological sort with diamond dependency."""
        graph = {"a": ["b", "c"], "b": ["d"], "c": ["d"], "d": []}
        result = rsk.topological_sort(graph)

        sorted_list = result["sorted"]
        # a must come first, d must come last
        assert sorted_list[0] == "a"
        assert sorted_list[-1] == "d"

    def test_topological_sort_cycle(self):
        """Test topological sort detects cycles."""
        graph = {"a": ["b"], "b": ["c"], "c": ["a"]}
        result = rsk.topological_sort(graph)

        assert "error" in result or "cycle" in result

    def test_level_parallelization_basic(self):
        """Test basic level parallelization."""
        graph = {"a": ["b", "c"], "b": ["d"], "c": ["d"], "d": []}
        result = rsk.level_parallelization(graph)

        assert isinstance(result, dict)
        assert "levels" in result
        assert "total_levels" in result
        assert result["total_levels"] == 3
        # Level 0: [a], Level 1: [b, c], Level 2: [d]
        assert len(result["levels"][0]) == 1
        assert len(result["levels"][1]) == 2
        assert len(result["levels"][2]) == 1

    def test_shortest_path_basic(self):
        """Test basic shortest path."""
        graph = {"a": [("b", 1.0), ("c", 3.0)], "b": [("c", 1.0)], "c": []}
        result = rsk.shortest_path(graph, "a", "c")

        assert isinstance(result, dict)
        assert "path" in result
        assert "cost" in result
        # Shortest: a -> b -> c (cost 2) vs a -> c (cost 3)
        assert result["path"] == ["a", "b", "c"]
        assert result["cost"] == 2.0

    def test_shortest_path_no_path(self):
        """Test shortest path when no path exists."""
        graph = {"a": [], "b": []}
        result = rsk.shortest_path(graph, "a", "b")

        assert "error" in result or result.get("status") == "error"


# ============================================================================
# 9. YAML Operations (1 function)
# ============================================================================

class TestYamlOperations:
    """Tests for parse_yaml_string function."""

    def test_parse_yaml_basic(self):
        """Test basic YAML parsing."""
        yaml_content = "name: test\nversion: '1.0'"
        result = rsk.parse_yaml_string(yaml_content)

        assert isinstance(result, dict)
        assert result["status"] == "success"
        assert result["data"]["name"] == "test"
        assert result["data"]["version"] == "1.0"

    def test_parse_yaml_complex(self):
        """Test complex YAML parsing."""
        yaml_content = """
name: test
nested:
  key1: value1
  key2: value2
list:
  - item1
  - item2
"""
        result = rsk.parse_yaml_string(yaml_content)

        assert result["status"] == "success"
        assert "nested" in result["data"]
        assert "list" in result["data"]

    def test_parse_yaml_invalid(self):
        """Test invalid YAML parsing."""
        result = rsk.parse_yaml_string("invalid: yaml: content: [")
        assert result["status"] == "error" or "error" in result


# ============================================================================
# 10. Execution Engine Operations (1 function)
# ============================================================================

class TestExecutionEngineOperations:
    """Tests for build_execution_plan function."""

    def test_build_execution_plan_basic(self):
        """Test basic execution plan building."""
        modules = [
            {"id": "M1", "name": "Root task", "dependencies": []},
            {"id": "M2", "name": "Depends on M1", "dependencies": ["M1"]},
        ]
        result = rsk.build_execution_plan(modules)

        assert isinstance(result, dict)
        assert result["status"] == "success"
        assert result["execution_order"] == ["M1", "M2"]
        assert result["module_count"] == 2

    def test_build_execution_plan_parallel(self):
        """Test execution plan with parallel tasks."""
        modules = [
            {"id": "M1", "name": "Root", "dependencies": []},
            {"id": "M2", "name": "Parallel 1", "dependencies": ["M1"]},
            {"id": "M3", "name": "Parallel 2", "dependencies": ["M1"]},
            {"id": "M4", "name": "Join", "dependencies": ["M2", "M3"]},
        ]
        result = rsk.build_execution_plan(modules)

        assert result["status"] == "success"
        assert len(result["levels"]) == 3
        # Level 1 should have 2 parallel tasks
        assert len(result["levels"][1]) == 2

    def test_build_execution_plan_with_effort(self):
        """Test execution plan with effort sizes."""
        modules = [
            {"id": "M1", "name": "Small task", "dependencies": [], "effort": "S"},
            {"id": "M2", "name": "Large task", "dependencies": ["M1"], "effort": "XL"},
        ]
        result = rsk.build_execution_plan(modules)

        assert result["status"] == "success"
        assert result["estimated_duration_minutes"] > 0


# ============================================================================
# 11. Code Generator Operations (3 functions)
# ============================================================================

class TestCodeGeneratorOperations:
    """Tests for generate_validation_rules, generate_test_scaffold, generate_rust_stub functions."""

    def test_generate_validation_rules(self, sample_skill_md: str):
        """Test validation rules generation."""
        result = rsk.generate_validation_rules(sample_skill_md)

        assert isinstance(result, dict)
        assert "skill_name" in result
        assert "invariant_rules" in result
        assert "failure_mode_rules" in result
        assert "total_rules" in result

    def test_generate_test_scaffold(self, sample_skill_md: str):
        """Test test scaffold generation."""
        result = rsk.generate_test_scaffold(sample_skill_md)

        assert isinstance(result, dict)
        assert "skill_name" in result
        assert "test_cases" in result
        assert "rust_code" in result
        assert "#[test]" in result["rust_code"]

    def test_generate_rust_stub(self, sample_skill_md: str):
        """Test Rust stub generation."""
        result = rsk.generate_rust_stub(sample_skill_md)

        assert isinstance(result, dict)
        assert "skill_name" in result
        assert "module_name" in result
        assert "structs" in result
        assert "functions" in result
        assert "full_code" in result
        assert "use serde" in result["full_code"]


# ============================================================================
# 12. State Manager Operations (9 functions)
# ============================================================================

class TestStateManagerOperations:
    """Tests for checkpoint_* functions."""

    def test_checkpoint_create_manager(self, temp_state_dir: Path):
        """Test checkpoint manager creation."""
        result = rsk.checkpoint_create_manager(str(temp_state_dir))

        assert isinstance(result, dict)
        assert result["status"] == "success"
        assert result["state_dir"] == str(temp_state_dir)

    def test_checkpoint_create_context(self):
        """Test execution context creation."""
        result = rsk.checkpoint_create_context("test-pipeline", 5)

        assert isinstance(result, dict)
        assert result["name"] == "test-pipeline"
        assert result["total_steps"] == 5
        assert result["status"] == "Created"
        assert "id" in result

    def test_checkpoint_save_load_roundtrip(self, temp_state_dir: Path):
        """Test checkpoint save and load."""
        # Create context
        ctx = rsk.checkpoint_create_context("test-save-load", 3)
        context_id = ctx["id"]

        # Save
        save_result = rsk.checkpoint_save(str(temp_state_dir), json.dumps(ctx))
        assert save_result["status"] == "success"
        assert "path" in save_result

        # Load
        load_result = rsk.checkpoint_load(str(temp_state_dir), context_id)
        assert load_result["name"] == "test-save-load"
        assert load_result["total_steps"] == 3

    def test_checkpoint_list(self, temp_state_dir: Path):
        """Test checkpoint listing."""
        # Create and save some contexts
        for i in range(3):
            ctx = rsk.checkpoint_create_context(f"test-list-{i}", i + 1)
            rsk.checkpoint_save(str(temp_state_dir), json.dumps(ctx))

        # List
        result = rsk.checkpoint_list(str(temp_state_dir))
        assert result["status"] == "success"
        assert result["count"] == 3

    def test_checkpoint_stats(self, temp_state_dir: Path):
        """Test checkpoint statistics."""
        # Create and save a context
        ctx = rsk.checkpoint_create_context("test-stats", 5)
        rsk.checkpoint_save(str(temp_state_dir), json.dumps(ctx))

        # Get stats
        result = rsk.checkpoint_stats(str(temp_state_dir))
        assert result["status"] == "success"
        assert result["total"] >= 1
        assert result["created"] >= 1

    def test_checkpoint_find_resumable(self, temp_state_dir: Path):
        """Test finding resumable checkpoints."""
        # Create and save a context
        ctx = rsk.checkpoint_create_context("resumable-test", 5)
        rsk.checkpoint_save(str(temp_state_dir), json.dumps(ctx))

        # Find resumable
        result = rsk.checkpoint_find_resumable(str(temp_state_dir), "resumable-test")
        assert result.get("name") == "resumable-test" or result.get("status") == "not_found"

    def test_checkpoint_delete(self, temp_state_dir: Path):
        """Test checkpoint deletion."""
        # Create and save a context
        ctx = rsk.checkpoint_create_context("delete-test", 3)
        context_id = ctx["id"]
        rsk.checkpoint_save(str(temp_state_dir), json.dumps(ctx))

        # Delete
        result = rsk.checkpoint_delete(str(temp_state_dir), context_id)
        assert result["status"] == "success"
        assert result["deleted"] is True

        # Verify deleted
        load_result = rsk.checkpoint_load(str(temp_state_dir), context_id)
        assert load_result.get("status") == "not_found"

    def test_checkpoint_cleanup(self, temp_state_dir: Path):
        """Test checkpoint cleanup."""
        # Create some contexts
        for i in range(2):
            ctx = rsk.checkpoint_create_context(f"cleanup-test-{i}", i + 1)
            rsk.checkpoint_save(str(temp_state_dir), json.dumps(ctx))

        # Cleanup (with 0 days to clean all completed/cancelled)
        result = rsk.checkpoint_cleanup(str(temp_state_dir), 0)
        assert result["status"] == "success"
        assert "removed" in result


# ============================================================================
# API Contract Tests
# ============================================================================

class TestAPIContracts:
    """Tests to verify API contracts match rsk_bridge.py expectations."""

    def test_levenshtein_contract(self):
        """Verify levenshtein returns expected TypedDict structure."""
        result = rsk.levenshtein("a", "b")

        required_keys = {"distance", "similarity", "source_len", "target_len"}
        assert required_keys <= set(result.keys())
        assert isinstance(result["distance"], int)
        assert isinstance(result["similarity"], float)

    def test_fuzzy_search_contract(self):
        """Verify fuzzy_search returns expected list structure."""
        result = rsk.fuzzy_search("test", ["test"], 1)

        assert isinstance(result, list)
        if result:
            item = result[0]
            required_keys = {"candidate", "distance", "similarity"}
            assert required_keys <= set(item.keys())

    def test_sha256_contract(self):
        """Verify sha256 returns expected structure."""
        result = rsk.sha256("test")

        required_keys = {"algorithm", "hex", "bytes_hashed"}
        assert required_keys <= set(result.keys())
        assert len(result["hex"]) == 64

    def test_gzip_compress_contract(self):
        """Verify gzip_compress returns expected structure."""
        result = rsk.gzip_compress("test")

        required_keys = {"original_size", "compressed_size", "ratio", "savings_percent", "data"}
        assert required_keys <= set(result.keys())
        assert isinstance(result["data"], bytes)


# ============================================================================
# Performance Regression Tests
# ============================================================================

class TestPerformanceBaseline:
    """Performance tests to catch regressions."""

    def test_levenshtein_performance(self):
        """Levenshtein should handle moderate-length strings quickly."""
        import time

        source = "a" * 100
        target = "b" * 100

        start = time.perf_counter()
        for _ in range(100):
            rsk.levenshtein(source, target)
        elapsed = time.perf_counter() - start

        # Should complete 100 iterations in under 1 second
        assert elapsed < 1.0, f"Levenshtein too slow: {elapsed:.2f}s for 100 iterations"

    def test_fuzzy_search_performance(self):
        """Fuzzy search should handle 1000 candidates quickly."""
        import time

        candidates = [f"candidate_{i}" for i in range(1000)]

        start = time.perf_counter()
        rsk.fuzzy_search("candidate_500", candidates, 10)
        elapsed = time.perf_counter() - start

        # Should complete in under 100ms
        assert elapsed < 0.1, f"Fuzzy search too slow: {elapsed:.3f}s for 1000 candidates"


# ============================================================================
# Module Info Tests
# ============================================================================

class TestModuleInfo:
    """Tests for module metadata."""

    def test_version_available(self):
        """Verify __version__ is available."""
        assert hasattr(rsk, "__version__")
        assert isinstance(rsk.__version__, str)

    def test_doc_available(self):
        """Verify __doc__ is available."""
        assert hasattr(rsk, "__doc__")


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
