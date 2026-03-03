//! CLI action enums for RSK subcommands.
//!
//! Each top-level command with subcommands has a corresponding Action enum here.

use clap::Subcommand;

#[derive(Subcommand)]
pub enum GuardianAction {
    /// Calculate context risk score
    Risk {
        /// Stakes level: low, moderate, high, critical
        #[arg(short, long)]
        stakes: String,
        /// Expertise level: low, moderate, high, unknown
        #[arg(short, long)]
        expertise: String,
        /// Checkability level: low, moderate, high, unfalsifiable
        #[arg(short, long)]
        checkability: String,
        /// Output treatment: draft, reviewed, direct_use, published
        #[arg(short, long, default_value = "direct_use")]
        output: String,
    },
    /// Create a minimal IAIR report
    Report {
        /// Incident category code (e.g., CL-CONFAB)
        #[arg(short, long)]
        category: String,
        /// Domain of the incident
        #[arg(short, long)]
        domain: String,
        /// Stakes level
        #[arg(short, long, default_value = "moderate")]
        stakes: String,
        /// Severity (0.0-1.0)
        #[arg(long, default_value = "0.0")]
        severity: f64,
    },
    /// Show all incident categories
    Categories,
    /// Recommend risk minimization level
    Minimize {
        /// Risk score (0.0-1.0)
        #[arg(short, long)]
        risk: f64,
        /// Number of similar incidents
        #[arg(short, long, default_value = "0")]
        incidents: usize,
    },
}

#[derive(Subcommand)]
pub enum Sha256Action {
    /// Hash a string
    Hash {
        /// Input string to hash
        input: String,
    },
    /// Hash contents of a file
    File {
        /// Path to file
        path: String,
    },
    /// Verify a string matches expected hash
    Verify {
        /// Input string
        input: String,
        /// Expected hex hash
        expected: String,
    },
}

#[derive(Subcommand)]
pub enum TextAction {
    /// Parse a SKILL.md file and extract its machine specification
    Parse {
        /// Path to the SKILL.md file
        path: String,
    },
    /// Validate a SKILL.md file for Diamond v2 compliance
    Validate {
        /// Path to the SKILL.md file
        path: String,
    },
    /// Extract complete SMST (Skill Machine Specification Template) with scoring
    Smst {
        /// Path to the SKILL.md file
        path: String,
    },
    /// Tokenize text into words
    Tokenize {
        /// Text to tokenize
        text: String,
    },
    /// Normalize text (lowercase, collapse whitespace)
    Normalize {
        /// Text to normalize
        text: String,
        /// Remove punctuation
        #[arg(long, default_value = "false")]
        strip_punctuation: bool,
    },
    /// Calculate word frequency in text
    Frequency {
        /// Text to analyze
        text: String,
        /// Number of top words to return
        #[arg(short, long, default_value = "10")]
        top: usize,
    },
    /// Analyze text compressibility (Shannon entropy)
    Entropy {
        /// Text to analyze
        text: String,
    },
    /// Extract n-grams from text
    Ngrams {
        /// Text to analyze
        text: String,
        /// N-gram size
        #[arg(short, long, default_value = "2")]
        n: usize,
        /// Use word n-grams instead of character n-grams
        #[arg(short, long)]
        words: bool,
    },
    /// Slugify text for URLs/filenames
    Slugify {
        /// Text to slugify
        text: String,
    },
}

#[derive(Subcommand)]
pub enum GraphAction {
    /// Perform topological sort on a skill graph (JSON input)
    TopSort {
        /// JSON string or path to JSON file containing the skill graph
        #[arg(short, long)]
        input: String,
    },
    /// Find shortest path between two skills
    ShortestPath {
        /// JSON string or path to JSON file containing the skill graph
        #[arg(short, long)]
        input: String,
        /// Start skill name
        start: String,
        /// End skill name
        end: String,
    },
    /// Compute parallel execution levels for DAG vertices
    Levels {
        /// JSON string or path to JSON file containing the skill graph
        #[arg(short, long)]
        input: String,
    },
}

#[derive(Subcommand)]
pub enum GenerateAction {
    /// Generate validation rules from SMST `INVARIANTS`/`FAILURE_MODES`
    Rules {
        /// Path to SKILL.md file
        path: String,
        /// Output format: json (default) or rust
        #[arg(short, long, default_value = "json")]
        format: String,
    },
    /// Generate test scaffolding from SMST
    Tests {
        /// Path to SKILL.md file
        path: String,
        /// Output format: json (default) or rust
        #[arg(short, long, default_value = "json")]
        format: String,
    },
    /// Generate Rust module stub from SMST
    Stub {
        /// Path to SKILL.md file
        path: String,
    },
    /// Generate decision tree logic (YAML) from SMST
    Logic {
        /// Path to SKILL.md file
        path: String,
    },
}

#[derive(Subcommand)]
pub enum YamlAction {
    /// Parse YAML file to JSON
    Parse {
        /// Path to YAML file
        path: String,
    },
    /// Parse YAML from stdin to JSON (enables Rust acceleration for string parsing)
    ParseStdin,
    /// Parse TOML file to JSON
    Toml {
        /// Path to TOML file
        path: String,
    },
    /// Validate YAML/TOML against schema patterns
    Validate {
        /// Path to YAML/TOML file
        path: String,
        /// Schema type (auto-detect if not specified): decision-tree, taxonomy, skill-frontmatter
        #[arg(short, long)]
        schema: Option<String>,
    },
    /// Analyze decision tree structure
    DecisionTree {
        /// Path to decision tree YAML file
        path: String,
    },
    /// Extract taxonomy schema from YAML
    Taxonomy {
        /// Path to taxonomy YAML file
        path: String,
    },
    /// Parse YAML frontmatter from SKILL.md
    Frontmatter {
        /// Path to SKILL.md file
        path: String,
    },
    /// Execute a decision tree from YAML string or file
    ExecuteLogic {
        /// YAML string or path to decision tree file
        #[arg(short, long)]
        tree: String,
        /// Input JSON string
        #[arg(short, long)]
        input: String,
    },
}

#[derive(Subcommand)]
pub enum TaxonomyAction {
    /// Query a taxonomy entry by type and key
    Query {
        /// Taxonomy type: compliance, smst, category, node
        #[arg(short, long)]
        taxonomy_type: String,
        /// Key to lookup
        key: String,
    },
    /// List all entries in a taxonomy
    List {
        /// Taxonomy type: compliance, smst, category, `node_types`
        taxonomy_type: String,
    },
    /// Show compliance level details
    Compliance {
        /// Level name: bronze, silver, gold, platinum, diamond
        level: String,
    },
    /// Show SMST component details
    Smst {
        /// Component name: INPUTS, OUTPUTS, STATE, etc.
        component: String,
    },
    /// Show skill category details
    Category {
        /// Category name: algorithms, validation, text-processing, etc.
        category: String,
    },
}

#[derive(Subcommand)]
pub enum TelemetryAction {
    /// Show current telemetry configuration
    Status,
    /// Show available configuration presets
    Presets,
    /// Show telemetry configuration example
    Config {
        /// Preset name: default, json, compact, debug
        #[arg(short, long, default_value = "default")]
        preset: String,
    },
}

#[derive(Subcommand)]
pub enum CompressAction {
    /// Compress a string using gzip
    Gzip {
        /// Text to compress
        text: String,
        /// Compression level: fast, default, best
        #[arg(short, long, default_value = "default")]
        level: String,
    },
    /// Decompress gzip data (base64 encoded input)
    Gunzip {
        /// Base64-encoded gzip data
        data: String,
    },
    /// Compress a file using gzip
    File {
        /// Path to file to compress
        path: String,
        /// Output path (default: input.gz)
        #[arg(short, long)]
        output: Option<String>,
        /// Compression level: fast, default, best
        #[arg(short, long, default_value = "default")]
        level: String,
    },
    /// Estimate compressibility of text without compressing
    Estimate {
        /// Text to analyze
        text: String,
    },
}

#[derive(Subcommand)]
pub enum ExecAction {
    /// Build an execution plan from modules JSON
    Plan {
        /// JSON array of modules or path to JSON file
        #[arg(short, long)]
        modules: String,
    },
    /// Get execution status of a plan
    Status {
        /// Plan ID
        #[arg(short, long)]
        id: String,
    },
    /// Resume execution from a checkpoint
    Resume {
        /// Checkpoint ID to resume from
        #[arg(short, long)]
        id: String,
    },
    /// Validate a module list without creating a plan
    Validate {
        /// JSON array of modules or path to JSON file
        #[arg(short, long)]
        modules: String,
    },
}

#[derive(Subcommand)]
pub enum RouteAction {
    /// Find best matching skills for a query
    Find {
        /// Natural language query
        query: String,
        /// Source skill (optional context)
        #[arg(short, long)]
        source: Option<String>,
        /// Routing strategy: adjacency, capability, semantic, hybrid
        #[arg(long, default_value = "hybrid")]
        strategy: String,
        /// Maximum results to return
        #[arg(short, long, default_value = "5")]
        limit: usize,
    },
    /// List available routing strategies
    Strategies,
    /// Fuzzy skill name lookup (typo correction)
    Fuzzy {
        /// Skill name to look up (may contain typos)
        query: String,
        /// Maximum results to return
        #[arg(short, long, default_value = "5")]
        limit: usize,
    },
}

#[derive(Subcommand)]
pub enum StateAction {
    /// List all checkpoints
    List {
        /// Filter by pipeline name
        #[arg(short, long)]
        name: Option<String>,
        /// Filter by status: created, running, paused, completed, failed
        #[arg(short, long)]
        status: Option<String>,
    },
    /// Show details of a specific checkpoint
    Show {
        /// Checkpoint ID
        id: String,
    },
    /// Delete a checkpoint
    Delete {
        /// Checkpoint ID
        id: String,
    },
    /// Cleanup old checkpoints
    Cleanup {
        /// Maximum age in days (default: 7)
        #[arg(long, default_value = "7")]
        max_age: u32,
    },
    /// Show checkpoint statistics
    Stats,
}

#[derive(Subcommand)]
pub enum ChainAction {
    /// Validate a skill chain recursively
    Validate {
        /// Start skill name
        name: String,
        /// Maximum recursion depth
        #[arg(short, long, default_value = "3")]
        depth: usize,
        /// Path to registry JSON
        #[arg(short, long)]
        registry: Option<String>,
    },
    /// Run an inline chain definition (e.g. "analyze -> transform -> output")
    Run {
        /// Inline chain definition using -> for sequential, | for parallel
        chain: String,
        /// Dry run mode — show what would execute without running
        #[arg(long)]
        dry_run: bool,
        /// Fail fast — stop on first error (default: true)
        #[arg(long, default_value = "true")]
        fail_fast: bool,
    },
    /// Run a chain from a YAML definition file
    RunYaml {
        /// Path to chain YAML file
        path: String,
        /// Dry run mode
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
pub enum SkillsAction {
    /// Scan a directory for skills and update the registry
    Scan {
        /// Path to skills directory
        path: String,
        /// Path to save the registry JSON (default: ~/.rsk/skills.json)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// List all registered skills
    List {
        /// Path to registry JSON (default: ~/.rsk/skills.json)
        #[arg(short, long)]
        registry: Option<String>,
        /// Filter by strategy: rust_intrinsic, deterministic_logic, hybrid, pure_llm
        #[arg(short, long)]
        strategy: Option<String>,
    },
    /// Show details for a specific skill
    Info {
        /// Skill name
        name: String,
        /// Path to registry JSON
        #[arg(short, long)]
        registry: Option<String>,
    },
    /// Execute a skill logic tree
    Execute {
        /// Skill name
        name: String,
        /// Input JSON string
        #[arg(short, long)]
        input: String,
        /// Path to registry JSON
        #[arg(short, long)]
        registry: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum MicrogramAction {
    /// Run a microgram with input JSON
    Run {
        /// Path to microgram YAML file
        path: String,
        /// Input JSON (e.g. '{"n": 5}')
        #[arg(short, long, default_value = "{}")]
        input: String,
    },
    /// Run self-tests for a microgram
    Test {
        /// Path to microgram YAML file
        path: String,
    },
    /// Run all self-tests in a directory
    TestAll {
        /// Path to micrograms directory
        #[arg(default_value = "micrograms")]
        dir: String,
    },
    /// List micrograms in a directory
    List {
        /// Path to micrograms directory
        #[arg(default_value = "micrograms")]
        dir: String,
    },
    /// Chain micrograms: output of N flows into input of N+1
    Chain {
        /// Microgram names separated by ->  (e.g. "threshold-gate -> score-label")
        chain: String,
        /// Directory containing microgram YAML files
        #[arg(short, long, default_value = "micrograms")]
        dir: String,
        /// Initial input JSON
        #[arg(short, long, default_value = "{}")]
        input: String,
    },
    /// Generate a microgram from a spec
    Generate {
        /// Microgram name
        name: String,
        /// Description
        #[arg(short, long)]
        desc: String,
        /// Variable to check
        #[arg(short, long)]
        var: String,
        /// Operator: gt, gte, lt, lte, eq, is_null, is_not_null, matches
        #[arg(short, long)]
        op: String,
        /// Threshold value (integer)
        #[arg(short, long)]
        threshold: i64,
        /// Output label for true branch
        #[arg(long, default_value = "result")]
        true_label: String,
        /// Output label for false branch
        #[arg(long, default_value = "result")]
        false_label: String,
        /// Directory to write to
        #[arg(long, default_value = "micrograms")]
        out_dir: String,
    },
    /// Evolve a microgram by adding boundary test cases
    Evolve {
        /// Path to microgram YAML file
        path: String,
    },
    /// Auto-compose a chain to produce required output fields
    Compose {
        /// Required output field names, comma-separated
        #[arg(short, long)]
        require: String,
        /// Directory containing micrograms
        #[arg(short, long, default_value = "micrograms")]
        dir: String,
        /// Initial input JSON
        #[arg(short, long, default_value = "{}")]
        input: String,
    },
    /// Benchmark all micrograms in a directory
    Bench {
        /// Directory containing micrograms
        #[arg(default_value = "micrograms")]
        dir: String,
        /// Number of iterations per microgram
        #[arg(short, long, default_value = "1000")]
        iterations: usize,
    },
    /// Auto: compose → chain → execute → verify in one shot
    Auto {
        /// Required output field names, comma-separated
        #[arg(short, long)]
        require: String,
        /// Directory containing micrograms
        #[arg(short, long, default_value = "micrograms")]
        dir: String,
        /// Initial input JSON
        #[arg(short, long, default_value = "{}")]
        input: String,
    },
    /// Catalog the microgram ecosystem with connection graph
    Catalog {
        /// Directory containing micrograms
        #[arg(default_value = "micrograms")]
        dir: String,
    },
    /// Diff two micrograms — structural and behavioral comparison
    Diff {
        /// Path to first microgram
        left: String,
        /// Path to second microgram
        right: String,
    },
    /// Merge two micrograms into one with dispatch routing
    Merge {
        /// Path to first microgram
        left: String,
        /// Path to second microgram
        right: String,
        /// Name for merged microgram
        #[arg(short, long)]
        name: String,
        /// Description
        #[arg(short, long, default_value = "Merged microgram")]
        desc: String,
        /// Output directory
        #[arg(long, default_value = "micrograms")]
        out_dir: String,
    },
    /// Pipe multiple JSON inputs through a microgram or chain
    Pipe {
        /// Microgram name or chain (names separated by ->)
        target: String,
        /// Directory containing micrograms
        #[arg(short, long, default_value = "micrograms")]
        dir: String,
        /// JSON array of input objects
        #[arg(short, long)]
        inputs: String,
    },
    /// Save ecosystem state to a snapshot file
    Snapshot {
        /// Directory containing micrograms
        #[arg(default_value = "micrograms")]
        dir: String,
        /// Output snapshot file
        #[arg(short, long, default_value = "micrograms.snapshot.json")]
        out: String,
    },
    /// Restore ecosystem from a snapshot file
    Restore {
        /// Snapshot file to restore from
        snap: String,
        /// Directory to restore into
        #[arg(short, long, default_value = "micrograms")]
        dir: String,
    },
    /// Stress test with random inputs
    Stress {
        /// Directory containing micrograms
        #[arg(default_value = "micrograms")]
        dir: String,
        /// Iterations per microgram
        #[arg(short, long, default_value = "10000")]
        iterations: usize,
        /// Random seed
        #[arg(short, long, default_value = "42")]
        seed: u64,
    },
    /// Cross-test matrix: run every microgram against every other's tests
    Matrix {
        /// Directory containing micrograms
        #[arg(default_value = "micrograms")]
        dir: String,
    },
    /// Decision path coverage analysis
    Coverage {
        /// Directory containing micrograms
        #[arg(default_value = "micrograms")]
        dir: String,
    },
    /// Clone a microgram with mutated thresholds
    Clone {
        /// Path to source microgram
        source: String,
        /// Name for the clone
        #[arg(short, long)]
        name: String,
        /// Threshold shift (positive or negative integer)
        #[arg(short, long, default_value = "0")]
        delta: i64,
        /// Output directory
        #[arg(long, default_value = "micrograms")]
        out_dir: String,
    },
    /// Shrink an input to its minimal form that produces the same output
    Shrink {
        /// Path to microgram YAML file
        path: String,
        /// Input JSON to shrink
        #[arg(short, long)]
        input: String,
    },
}

#[derive(Subcommand)]
pub enum AntiPatternAction {
    /// Detect anti-patterns in numeric features
    Detect {
        /// JSON object of numeric features (e.g. '{"method_count": 25, "line_count": 600}')
        #[arg(short, long)]
        features: String,
        /// Detection confidence threshold (0.0-1.0)
        #[arg(short, long, default_value = "0.3")]
        threshold: f64,
    },
    /// Register a new anti-pattern from an observed failure
    Add {
        /// Pattern name
        name: String,
        /// Category (code, process, infra)
        #[arg(short, long, default_value = "code")]
        category: String,
        /// Description of what this pattern catches
        #[arg(short, long)]
        description: String,
        /// Metric name to watch
        #[arg(short, long)]
        metric: String,
        /// Threshold value
        #[arg(short, long)]
        threshold: f64,
        /// Direction: "exceeds" or "below"
        #[arg(long, default_value = "exceeds")]
        direction: String,
        /// Remediation advice
        #[arg(short, long)]
        remediation: String,
    },
    /// List all registered anti-patterns
    List,
    /// Show registry stats
    Stats,
}

#[cfg(feature = "forge")]
#[derive(Subcommand)]
pub enum ForgeAction {
    /// Validate a pipeline specification file
    Validate {
        /// Path to pipeline TOML file
        path: String,
    },
    /// Parse and display pipeline specification
    Parse {
        /// Path to pipeline TOML file
        path: String,
    },
    /// Show pipeline graph (ingest → transform → sink)
    Graph {
        /// Path to pipeline TOML file
        path: String,
    },
    /// Run a pipeline (stdin/stdout supported)
    Run {
        /// Path to pipeline TOML file
        path: String,
        /// Input data (if not using stdin source)
        #[arg(short, long)]
        input: Option<String>,
        /// Dry run - show what would be executed without running
        #[arg(long)]
        dry_run: bool,
    },
    /// List available source types
    Sources,
    /// List available transform operations
    Transforms,
    /// List available sink types
    Sinks,
}

#[derive(Subcommand)]
pub enum HooksAction {
    /// Validate file placement against organization policies
    Validate {
        /// Path to file to validate
        path: String,
        /// Output format: text, json
        #[arg(short, long, default_value = "text")]
        format: String,
    },
    /// Check if a file is stale
    Staleness {
        /// Path to file to check
        path: String,
        /// Output format: text, json
        #[arg(short, long, default_value = "text")]
        format: String,
    },
    /// Get the category of a file
    Categorize {
        /// Path to file
        path: String,
    },
    /// Scan directory for policy violations and stale files
    Scan {
        /// Directory to scan (default: current directory)
        #[arg(default_value = ".")]
        path: String,
        /// Maximum depth to scan
        #[arg(short, long, default_value = "3")]
        depth: usize,
        /// Output format: text, json
        #[arg(short, long, default_value = "text")]
        format: String,
    },
    /// Show current policy configuration
    Policy,
    /// Generate a blindspot check reminder for a file
    Blindspot {
        /// Path to file
        path: String,
        /// Output format: text, json
        #[arg(short, long, default_value = "text")]
        format: String,
    },
    /// Generate blindspot check for subagent completion
    SubagentReview {
        /// Subagent type (e.g., Plan, Explore, Bash)
        #[arg(short = 't', long)]
        agent_type: String,
        /// Task description
        description: String,
    },
    /// Output schema version for compatibility checks
    SchemaVersion,
}

#[derive(Subcommand)]
pub enum TovAction {
    /// Classify a harm event into one of 8 types (A-H)
    Classify {
        /// Multiplicity: single or multiple
        #[arg(short, long)]
        mult: String,
        /// Temporal profile: acute or chronic
        #[arg(short, long)]
        temp: String,
        /// Determinism: deterministic or stochastic
        #[arg(short, long)]
        det: String,
    },
    /// Calculate attenuation analysis for propagation probabilities
    Attenuation {
        /// Comma-separated propagation probabilities (each in (0,1))
        #[arg(short, long)]
        probs: String,
    },
    /// Calculate protective depth for a target probability
    ProtectiveDepth {
        /// Target probability threshold
        #[arg(short, long)]
        target: f64,
        /// Attenuation rate alpha
        #[arg(short, long)]
        alpha: f64,
    },
    /// Determine ACA case (Four-Case Logic Engine)
    Aca {
        /// Algorithm correctness: correct or wrong
        #[arg(short, long)]
        correctness: String,
        /// Clinician response: followed or overrode
        #[arg(short, long)]
        response: String,
        /// Clinical outcome: good or harm
        #[arg(short, long)]
        outcome: String,
    },
    /// Calculate KHS_AI score
    Khs {
        /// Latency stability score (0-100)
        #[arg(short, long)]
        latency: u8,
        /// Accuracy stability score (0-100)
        #[arg(short, long)]
        accuracy: u8,
        /// Resource efficiency score (0-100)
        #[arg(short, long)]
        resource: u8,
        /// Drift score (0-100)
        #[arg(short, long)]
        drift: u8,
    },
    /// Show all harm types with their characteristics
    HarmTypes,
    /// Show all conservation laws
    ConservationLaws,
}
