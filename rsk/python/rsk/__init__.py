"""RSK - Rust Skill Kernel

High-performance Python bindings for Claude Code skills.

This package provides Rust-native implementations achieving 10-100x speedups
over pure Python equivalents.

Functions:
    levenshtein(source, target) -> dict
        Calculate edit distance between two strings.
        Returns: {"distance": int, "similarity": float, "source_len": int, "target_len": int}

    fuzzy_search(query, candidates, limit) -> list[dict]
        Find best matches for query among candidates.
        Returns: [{"candidate": str, "distance": int, "similarity": float}, ...]

    sha256(input) -> dict
        Calculate SHA-256 hash of a string.
        Returns: {"algorithm": "sha256", "hex": str, "bytes_hashed": int}

    sha256_verify(input, expected) -> bool
        Verify a string against expected hash.

    variance(actual, target) -> dict
        Calculate variance between values.
        Returns: {"absolute": float, "percentage": float}

    query_taxonomy(taxonomy_type, key) -> dict
        Query taxonomy by type (compliance, smst, category, node) and key.

    list_taxonomy(taxonomy_type) -> dict
        List all entries in a taxonomy.

    extract_smst(content) -> dict
        Extract SMST (Skill Machine Specification Template) from SKILL.md content.

    parse_frontmatter(content) -> dict
        Parse YAML frontmatter from SKILL.md content.

Example:
    >>> import rsk
    >>> result = rsk.levenshtein("kitten", "sitting")
    >>> print(f"Distance: {result['distance']}")
    Distance: 3

    >>> hash_result = rsk.sha256("hello world")
    >>> print(hash_result['hex'][:16])
    b94d27b9934d3e08

Performance:
    | Operation      | Python | Rust  | Speedup |
    |---------------|--------|-------|---------|
    | levenshtein   | 63ms   | 1ms   | 63x     |
    | sha256        | 10ms   | 0.5ms | 20x     |
    | extract_smst  | 50ms   | 5ms   | 10x     |
"""

from rsk.rsk import (
    # Levenshtein operations
    levenshtein,
    fuzzy_search,
    # Crypto operations
    sha256,
    sha256_verify,
    # Math operations
    variance,
    is_prime,
    # Taxonomy operations
    query_taxonomy,
    list_taxonomy,
    # SKILL.md operations
    extract_smst,
    parse_frontmatter,
    # Text processing operations
    tokenize,
    normalize,
    word_frequency,
    text_entropy,
    # Compression operations
    gzip_compress,
    gzip_decompress,
    estimate_compressibility,
    # Graph operations (C2 Sprint 1)
    topological_sort,
    level_parallelization,
    shortest_path,
    # YAML operations (C2 Sprint 1)
    parse_yaml_string,
    # Execution engine operations (C2 Sprint 2)
    build_execution_plan,
    # Code generator operations (C2 Sprint 4)
    generate_validation_rules,
    generate_test_scaffold,
    generate_rust_stub,
    # State manager operations (C2 Sprint 4)
    checkpoint_create_manager,
    checkpoint_create_context,
    checkpoint_save,
    checkpoint_load,
    checkpoint_find_resumable,
    checkpoint_list,
    checkpoint_stats,
    checkpoint_delete,
    checkpoint_cleanup,
    # JSON processing operations (C2 Sprint 5)
    parse_json_string,
    serialize_json,
    json_query,
    json_set,
    json_merge,
    json_diff,
    json_flatten,
    json_unflatten,
    # Decision Engine operations (C2 Sprint 6)
    execute_logic,
    generate_logic,
    # Module info
    __version__,
)

__all__ = [
    # Levenshtein operations
    "levenshtein",
    "fuzzy_search",
    # Crypto operations
    "sha256",
    "sha256_verify",
    # Math operations
    "variance",
    "is_prime",
    # Taxonomy operations
    "query_taxonomy",
    "list_taxonomy",
    # SKILL.md operations
    "extract_smst",
    "parse_frontmatter",
    # Text processing operations
    "tokenize",
    "normalize",
    "word_frequency",
    "text_entropy",
    # Compression operations
    "gzip_compress",
    "gzip_decompress",
    "estimate_compressibility",
    # Graph operations (C2 Sprint 1)
    "topological_sort",
    "level_parallelization",
    "shortest_path",
    # YAML operations (C2 Sprint 1)
    "parse_yaml_string",
    # Execution engine operations (C2 Sprint 2)
    "build_execution_plan",
    # Code generator operations (C2 Sprint 4)
    "generate_validation_rules",
    "generate_test_scaffold",
    "generate_rust_stub",
    # State manager operations (C2 Sprint 4)
    "checkpoint_create_manager",
    "checkpoint_create_context",
    "checkpoint_save",
    "checkpoint_load",
    "checkpoint_find_resumable",
    "checkpoint_list",
    "checkpoint_stats",
    "checkpoint_delete",
    "checkpoint_cleanup",
    # JSON processing operations (C2 Sprint 5)
    "parse_json_string",
    "serialize_json",
    "json_query",
    "json_set",
    "json_merge",
    "json_diff",
    "json_flatten",
    "json_unflatten",
    # Decision Engine operations (C2 Sprint 6)
    "execute_logic",
    "generate_logic",
    # Module info
    "__version__",
]
