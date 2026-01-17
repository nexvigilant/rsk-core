"""Type stubs for rsk - Rust Skill Kernel."""

from typing import TypedDict

class LevenshteinResult(TypedDict):
    distance: int
    similarity: float
    source_len: int
    target_len: int

class FuzzyMatch(TypedDict):
    candidate: str
    distance: int
    similarity: float

class HashResult(TypedDict):
    algorithm: str
    hex: str
    bytes_hashed: int

class VarianceResult(TypedDict):
    absolute: float
    percentage: float

def levenshtein(source: str, target: str) -> LevenshteinResult:
    """Calculate Levenshtein edit distance between two strings.

    Args:
        source: Source string
        target: Target string

    Returns:
        Dictionary with distance, similarity (0-1), and string lengths

    Example:
        >>> result = levenshtein("kitten", "sitting")
        >>> result['distance']
        3
    """
    ...

def fuzzy_search(query: str, candidates: list[str], limit: int) -> list[FuzzyMatch]:
    """Find best matches for query among candidates.

    Args:
        query: Query string to match
        candidates: List of candidate strings
        limit: Maximum number of results to return

    Returns:
        List of matches sorted by similarity (best first)

    Example:
        >>> matches = fuzzy_search("test", ["test-skill", "testing", "best"], 3)
        >>> matches[0]['candidate']
        'test-skill'
    """
    ...

def sha256(input: str) -> HashResult:
    """Calculate SHA-256 hash of a string.

    Args:
        input: String to hash

    Returns:
        Dictionary with algorithm name, hex digest, and bytes hashed

    Example:
        >>> result = sha256("hello")
        >>> len(result['hex'])
        64
    """
    ...

def sha256_verify(input: str, expected: str) -> bool:
    """Verify a string against an expected SHA-256 hash.

    Args:
        input: String to verify
        expected: Expected hex hash

    Returns:
        True if hash matches, False otherwise
    """
    ...

def variance(actual: float, target: float) -> VarianceResult:
    """Calculate variance between actual and target values.

    Args:
        actual: Actual value
        target: Target value

    Returns:
        Dictionary with absolute and percentage variance
    """
    ...

def query_taxonomy(taxonomy_type: str, key: str) -> dict:
    """Query taxonomy by type and key.

    Args:
        taxonomy_type: One of 'compliance', 'smst', 'category', 'node'
        key: Key to lookup in the taxonomy

    Returns:
        Taxonomy entry data or error if not found
    """
    ...

def list_taxonomy(taxonomy_type: str) -> dict:
    """List all entries in a taxonomy.

    Args:
        taxonomy_type: One of 'compliance', 'smst', 'category', 'node_types'

    Returns:
        Dictionary with all taxonomy entries
    """
    ...

def extract_smst(content: str) -> dict:
    """Extract SMST from SKILL.md content.

    Args:
        content: Full SKILL.md file content

    Returns:
        Parsed SMST with frontmatter, sections, and scoring
    """
    ...

def parse_frontmatter(content: str) -> dict:
    """Parse YAML frontmatter from SKILL.md content.

    Args:
        content: Full SKILL.md file content

    Returns:
        Parsed frontmatter as dictionary
    """
    ...

# Text processing operations

class TokenizeResult(TypedDict):
    tokens: list[str]
    count: int
    unique_count: int

class NormalizeResult(TypedDict):
    text: str
    original_length: int
    normalized_length: int

class WordFrequencyResult(TypedDict):
    frequencies: dict[str, int]
    total_words: int
    unique_words: int
    top_words: list[tuple[str, int]]

class TextEntropyResult(TypedDict):
    original_chars: int
    unique_chars: int
    entropy_estimate: float
    compressibility: str

def tokenize(text: str) -> TokenizeResult:
    """Tokenize text into words.

    Args:
        text: Input text to tokenize

    Returns:
        Dictionary with tokens list, count, and unique count
    """
    ...

def normalize(text: str, remove_punctuation: bool = True) -> NormalizeResult:
    """Normalize text for comparison.

    Args:
        text: Input text to normalize
        remove_punctuation: Whether to remove punctuation (default: True)

    Returns:
        Dictionary with normalized text and length information
    """
    ...

def word_frequency(text: str, top_n: int = 10) -> WordFrequencyResult:
    """Calculate word frequencies in text.

    Args:
        text: Input text to analyze
        top_n: Number of top words to return (default: 10)

    Returns:
        Dictionary with frequencies, totals, and top words
    """
    ...

def text_entropy(text: str) -> TextEntropyResult:
    """Calculate text entropy (compressibility analysis).

    Args:
        text: Input text to analyze

    Returns:
        Dictionary with entropy estimate and compressibility rating
    """
    ...

# Compression operations

class CompressionResult(TypedDict):
    original_size: int
    compressed_size: int
    ratio: float
    savings_percent: float
    data: bytes

def gzip_compress(text: str, level: str = "default") -> CompressionResult:
    """Compress text using gzip.

    Args:
        text: Input text to compress
        level: Compression level - "fast", "default", or "best"

    Returns:
        Dictionary with sizes, ratio, savings, and compressed data
    """
    ...

def gzip_decompress(data: bytes) -> str:
    """Decompress gzip data to text.

    Args:
        data: Compressed bytes

    Returns:
        Decompressed text as string

    Raises:
        ValueError: If decompression fails
    """
    ...

def estimate_compressibility(data: bytes) -> float:
    """Estimate compressibility without actually compressing.

    Args:
        data: Input bytes to analyze

    Returns:
        Estimated ratio (0.0 = highly compressible, 1.0 = not compressible)
    """
    ...

# Graph operations (C2 Sprint 1)

class TopologicalSortResult(TypedDict):
    sorted: list[str]
    order: list[int]
    status: str

class LevelParallelizationResult(TypedDict):
    levels: list[list[str]]
    total_levels: int
    status: str

class ShortestPathResult(TypedDict):
    path: list[str]
    cost: float
    status: str

def topological_sort(graph: dict[str, list[str]]) -> TopologicalSortResult:
    """Perform topological sort on a DAG.

    Args:
        graph: Dict mapping node names to lists of successor node names

    Returns:
        Dictionary with sorted nodes in dependency order

    Example:
        >>> result = topological_sort({"a": ["b"], "b": ["c"], "c": []})
        >>> result['sorted']
        ['a', 'b', 'c']
    """
    ...

def level_parallelization(graph: dict[str, list[str]]) -> LevelParallelizationResult:
    """Compute parallel execution levels for DAG vertices.

    Args:
        graph: Dict mapping node names to lists of successor node names

    Returns:
        Dictionary with levels (each inner list can execute in parallel)

    Example:
        >>> result = level_parallelization({"a": ["b", "c"], "b": ["d"], "c": ["d"], "d": []})
        >>> result['levels']
        [['a'], ['b', 'c'], ['d']]
    """
    ...

def shortest_path(
    graph: dict[str, list[tuple[str, float]]],
    start: str,
    end: str,
) -> ShortestPathResult:
    """Find shortest path between two nodes using Dijkstra's algorithm.

    Args:
        graph: Dict mapping node names to lists of (target, weight) tuples
        start: Starting node name
        end: Target node name

    Returns:
        Dictionary with path (list of nodes), cost, and status

    Example:
        >>> graph = {"a": [("b", 1.0), ("c", 3.0)], "b": [("c", 1.0)], "c": []}
        >>> result = shortest_path(graph, "a", "c")
        >>> result['path']
        ['a', 'b', 'c']
        >>> result['cost']
        2.0
    """
    ...

# YAML operations (C2 Sprint 1)

class YamlParseResult(TypedDict):
    status: str
    format: str
    data: dict
    keys: list[str]
    depth: int

def parse_yaml_string(content: str) -> YamlParseResult:
    """Parse YAML string to Python dict.

    Args:
        content: YAML content as string

    Returns:
        Dictionary with parsed data and metadata

    Example:
        >>> result = parse_yaml_string("name: test\\nversion: '1.0'")
        >>> result['data']['name']
        'test'
    """
    ...

# Execution engine operations (C2 Sprint 2)

class ExecutionPlanResult(TypedDict):
    execution_order: list[str]
    levels: list[list[str]]
    critical_path: list[str]
    estimated_duration_minutes: int
    module_count: int
    status: str

class ModuleSpec(TypedDict, total=False):
    id: str
    name: str
    dependencies: list[str]
    effort: str  # "S", "M", "L", "XL"
    risk: float  # 0.0 - 1.0
    critical: bool
    purpose: str
    resources: list[str]
    deliverables: list[str]

def build_execution_plan(modules: list[ModuleSpec]) -> ExecutionPlanResult:
    """Build an execution plan from a list of modules.

    Computes topological order, parallel execution levels, and critical path
    for a set of modules with dependencies.

    Args:
        modules: List of module specifications with id, name, dependencies,
                 and optional effort/risk/critical flags

    Returns:
        Dictionary with execution_order, levels, critical_path, and timing estimates

    Example:
        >>> modules = [
        ...     {"id": "M1", "name": "Root", "dependencies": []},
        ...     {"id": "M2", "name": "Child", "dependencies": ["M1"]},
        ... ]
        >>> plan = build_execution_plan(modules)
        >>> plan['levels']
        [['M1'], ['M2']]
    """
    ...

# Code generator operations (C2 Sprint 4)

class ValidationRule(TypedDict):
    id: str
    description: str
    severity: str
    condition: str
    error_message: str

class ValidationRuleset(TypedDict):
    skill_name: str
    invariant_rules: list[ValidationRule]
    failure_mode_rules: list[ValidationRule]
    input_rules: list[ValidationRule]
    output_rules: list[ValidationRule]
    total_rules: int

class GeneratedTestCase(TypedDict):
    name: str
    category: str
    description: str
    inputs: str
    expected: str

class TestScaffold(TypedDict):
    skill_name: str
    module_path: str
    test_cases: list[GeneratedTestCase]
    rust_code: str

class RustStub(TypedDict):
    skill_name: str
    module_name: str
    structs: str
    functions: str
    full_code: str

def generate_validation_rules(content: str) -> ValidationRuleset:
    """Generate validation rules from SMST content.

    Parses INVARIANTS and FAILURE_MODES sections to create validation rules.

    Args:
        content: SKILL.md content containing SMST specification

    Returns:
        ValidationRuleset with rules from invariants, failure modes, inputs, outputs

    Example:
        >>> rules = generate_validation_rules(skill_content)
        >>> rules['total_rules']
        12
    """
    ...

def generate_test_scaffold(content: str) -> TestScaffold:
    """Generate test scaffold from SMST content.

    Creates positive, negative, and edge case tests from SMST specification.

    Args:
        content: SKILL.md content containing SMST specification

    Returns:
        TestScaffold with test cases and Rust test module code

    Example:
        >>> scaffold = generate_test_scaffold(skill_content)
        >>> len(scaffold['test_cases'])
        5
    """
    ...

def generate_rust_stub(content: str) -> RustStub:
    """Generate Rust code stub from SMST content.

    Creates struct definitions, function signatures, and error types.

    Args:
        content: SKILL.md content containing SMST specification

    Returns:
        RustStub with structs, functions, and full module code

    Example:
        >>> stub = generate_rust_stub(skill_content)
        >>> print(stub['full_code'])
    """
    ...

# State manager operations (C2 Sprint 4)

class CheckpointManagerResult(TypedDict):
    status: str
    state_dir: str

class ExecutionContextResult(TypedDict):
    id: str
    name: str
    status: str
    total_steps: int
    completed_steps: list[int]
    failed_steps: list[int]
    skipped_steps: list[int]
    step_results: dict[str, dict]
    started_at: str
    updated_at: str
    artifacts: dict[str, object]
    parent_id: str | None
    tags: list[str]

class CheckpointListResult(TypedDict):
    contexts: list[ExecutionContextResult]
    count: int
    status: str

class CheckpointStatsResult(TypedDict):
    total: int
    created: int
    running: int
    paused: int
    completed: int
    failed: int
    cancelled: int
    status: str

def checkpoint_create_manager(state_dir: str) -> CheckpointManagerResult:
    """Create a new checkpoint manager for a state directory.

    Creates the directory if it doesn't exist.

    Args:
        state_dir: Path to directory for storing checkpoints

    Returns:
        Dictionary with status and state_dir on success, or error on failure
    """
    ...

def checkpoint_create_context(name: str, total_steps: int) -> ExecutionContextResult:
    """Create a new execution context.

    Args:
        name: Human-readable name for the execution
        total_steps: Total number of steps in the pipeline

    Returns:
        Execution context with id, name, status, and step tracking
    """
    ...

def checkpoint_save(state_dir: str, context_json: str) -> dict:
    """Save a context to disk.

    Args:
        state_dir: Path to checkpoint directory
        context_json: JSON string of the execution context

    Returns:
        Dictionary with status and path on success, or error on failure
    """
    ...

def checkpoint_load(state_dir: str, context_id: str) -> ExecutionContextResult | dict:
    """Load a context by ID.

    Args:
        state_dir: Path to checkpoint directory
        context_id: Unique identifier of the context

    Returns:
        Execution context or dict with status='not_found' if not found
    """
    ...

def checkpoint_find_resumable(state_dir: str, name: str) -> ExecutionContextResult | dict:
    """Find a resumable context by name.

    Searches for contexts that are Created, Running, or Paused.

    Args:
        state_dir: Path to checkpoint directory
        name: Name of the pipeline to find

    Returns:
        Most recent resumable context or dict with status='not_found'
    """
    ...

def checkpoint_list(state_dir: str) -> CheckpointListResult:
    """List all checkpoints in a directory.

    Args:
        state_dir: Path to checkpoint directory

    Returns:
        Dictionary with contexts list and count
    """
    ...

def checkpoint_stats(state_dir: str) -> CheckpointStatsResult:
    """Get checkpoint statistics.

    Args:
        state_dir: Path to checkpoint directory

    Returns:
        Dictionary with counts by status
    """
    ...

def checkpoint_delete(state_dir: str, context_id: str) -> dict:
    """Delete a checkpoint by ID.

    Args:
        state_dir: Path to checkpoint directory
        context_id: ID of the context to delete

    Returns:
        Dictionary with deleted (bool) and status
    """
    ...

def checkpoint_cleanup(state_dir: str, max_age_days: int) -> dict:
    """Cleanup old checkpoints.

    Removes completed/cancelled checkpoints older than max_age_days.

    Args:
        state_dir: Path to checkpoint directory
        max_age_days: Maximum age in days

    Returns:
        Dictionary with removed count and status
    """
    ...

__version__: str
"""Package version string."""
