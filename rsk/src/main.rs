use clap::{Parser, Subcommand};
use rsk::CheckpointManager;
use rsk::{
    DecisionContext, ExecutionResult, RoutingEngine, RoutingRequest, RoutingStrategy,
    SkillRegistry, Value,
};
use rsk::{EffortSize, ExecutionModule, build_execution_plan, detect_resource_conflicts};
use rsk::{
    SkillGraph, SkillNode, calculate_variance, fuzzy_search, levenshtein, sha256_hash,
    sha256_verify,
};
use rsk::{TelemetryConfig, get_telemetry_status};
use rsk::{
    analyze_decision_tree, extract_taxonomy_schema, parse_toml, parse_yaml, parse_yaml_frontmatter,
    validate_schema,
};
use rsk::{generate_test_scaffold, generate_validation_rules};
use rsk::{list_taxonomy, query_taxonomy};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "rsk")]
#[command(about = "Rust Skill Kernel (rsk) - High-performance core for Claude Code skills", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Calculate variance between actual and target
    Variance { actual: f64, target: f64 },
    /// Skill Graph operations
    Graph {
        #[command(subcommand)]
        action: GraphAction,
    },
    /// Text processing and SKILL.md validation
    Text {
        #[command(subcommand)]
        action: TextAction,
    },
    /// Validate a SKILL.md for Diamond v2 compliance (primary validation command)
    Verify {
        /// Path to SKILL.md or directory containing it
        path: String,
        /// Threshold for success (default 85.0)
        #[arg(short, long, default_value = "85.0")]
        threshold: f64,
        /// Output format: json (default), summary, minimal, or report
        #[arg(short, long, default_value = "json")]
        format: String,
        /// Export SMST JSON Schema and exit
        #[arg(long)]
        export_jsonschema: bool,
        /// Show detailed check results
        #[arg(short, long)]
        verbose: bool,
    },
    /// Alias for verify - validate skills for Diamond v2 compliance
    #[command(alias = "val")]
    Validate {
        /// Path to SKILL.md file or skill directory or parent directory containing skills
        path: String,
        /// Minimum score threshold (0-100, default: 85.0 for Diamond)
        #[arg(short, long, default_value = "85.0")]
        threshold: f64,
        /// Output format: json (default), summary, minimal, or report
        #[arg(short, long, default_value = "json")]
        format: String,
        /// Export SMST JSON Schema and exit
        #[arg(long)]
        export_jsonschema: bool,
        /// Show detailed check results
        #[arg(short, long)]
        verbose: bool,
    },
    /// Build skill artifacts (templates, references, compiled outputs)
    Build {
        /// Path to skill directory
        path: String,
        /// Dry run - report what would be built without executing
        #[arg(short, long)]
        dry_run: bool,
    },
    /// Generate code from SMST (validation rules, tests, Rust stubs)
    Generate {
        #[command(subcommand)]
        action: GenerateAction,
    },
    /// Levenshtein edit distance (63x faster than Python)
    Levenshtein {
        /// Source string
        source: String,
        /// Target string
        target: String,
    },
    /// Fuzzy search: find best matches for query against candidates
    Fuzzy {
        /// Query string to match
        query: String,
        /// Comma-separated list of candidates
        #[arg(short, long)]
        candidates: String,
        /// Maximum number of results (default: 5)
        #[arg(short, long, default_value = "5")]
        limit: usize,
    },
    /// SHA-256 hash operations
    Sha256 {
        #[command(subcommand)]
        action: Sha256Action,
    },
    /// Kernel version
    Version,
    /// YAML/TOML processing and validation
    Yaml {
        #[command(subcommand)]
        action: YamlAction,
    },
    /// Taxonomy lookup (O(1) compile-time tables via phf)
    Taxonomy {
        #[command(subcommand)]
        action: TaxonomyAction,
    },
    /// Telemetry configuration and status
    Telemetry {
        #[command(subcommand)]
        action: TelemetryAction,
    },
    /// Compression utilities (gzip)
    Compress {
        #[command(subcommand)]
        action: CompressAction,
    },
    /// Execution engine operations (plan building, DAG execution)
    Exec {
        #[command(subcommand)]
        action: ExecAction,
    },
    /// Skill routing operations (find next skills)
    /// Start the in-memory state server
    Server {
        /// Path to Unix domain socket
        #[arg(short, long)]
        socket: Option<String>,
    },
    Route {
        #[command(subcommand)]
        action: RouteAction,
    },
    /// State/checkpoint management operations
    /// Start the in-memory state server
    State {
        #[command(subcommand)]
        action: StateAction,
    },
    /// Skill registry operations (scan, list, execute)
    Skills {
        #[command(subcommand)]
        action: SkillsAction,
    },
    /// Skill chain operations (validation, visualization)
    Chain {
        #[command(subcommand)]
        action: ChainAction,
    },
    /// Autonomously evolve the kernel by compiling a skill into a Rust intrinsic
    Evolve {
        /// Skill name
        name: String,
        /// Path to registry JSON
        #[arg(short, long)]
        registry: Option<String>,
    },
    /// RustForge pipeline operations (requires --features forge)
    #[cfg(feature = "forge")]
    Forge {
        #[command(subcommand)]
        action: ForgeAction,
    },
    /// File organization hooks (validation, staleness, blindspot checks)
    Hooks {
        #[command(subcommand)]
        action: HooksAction,
    },
    /// Theory of Vigilance (ToV) operations
    Tov {
        #[command(subcommand)]
        action: TovAction,
    },
    /// Guardian-AV algorithmovigilance operations
    Guardian {
        #[command(subcommand)]
        action: GuardianAction,
    },
}

#[derive(Subcommand)]
enum GuardianAction {
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
enum Sha256Action {
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
enum TextAction {
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
enum GraphAction {
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
enum GenerateAction {
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
enum YamlAction {
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
enum TaxonomyAction {
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
enum TelemetryAction {
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
enum CompressAction {
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
enum ExecAction {
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
enum RouteAction {
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
enum StateAction {
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
enum ChainAction {
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
}

#[derive(Subcommand)]
enum SkillsAction {
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

#[cfg(feature = "forge")]
#[derive(Subcommand)]
enum ForgeAction {
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
enum HooksAction {
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
enum TovAction {
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

fn load_graph(input: &str) -> Result<SkillGraph, Box<dyn std::error::Error>> {
    let content = if input.ends_with(".json") {
        fs::read_to_string(input)?
    } else {
        input.to_string()
    };

    // Check if it's a direct SkillGraph or a list of nodes
    if let Ok(graph) = serde_json::from_str::<SkillGraph>(&content) {
        return Ok(graph);
    }

    if let Ok(nodes) = serde_json::from_str::<Vec<SkillNode>>(&content) {
        let mut graph = SkillGraph::new();
        for node in nodes {
            graph.add_node(node);
        }
        return Ok(graph);
    }

    Err("Invalid graph format".into())
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Variance { actual, target } => {
            let result = calculate_variance(*actual, *target);
            println!("{}", json!(result));
        }
        Commands::Verify {
            path,
            threshold,
            format,
            export_jsonschema,
            verbose,
        }
        | Commands::Validate {
            path,
            threshold,
            format,
            export_jsonschema,
            verbose,
        } => {
            // Handle JSON Schema export mode
            if *export_jsonschema {
                // Output SMST structure as JSON Schema Draft 2020-12
                let schema = json!({
                    "$schema": "https://json-schema.org/draft/2020-12/schema",
                    "$id": "https://claude.ai/schemas/smst-v2.json",
                    "title": "SMST - Skill Machine Specification Template v2",
                    "description": "Schema for Claude Code skill machine specifications (Diamond v2 compliance)",
                    "type": "object",
                    "properties": {
                        "frontmatter": {
                            "type": "object",
                            "description": "YAML frontmatter metadata",
                            "properties": {
                                "name": { "type": "string", "description": "Skill identifier (kebab-case)" },
                                "description": { "type": "string", "description": "One-line skill description" },
                                "version": { "type": "string", "pattern": "^\\d+\\.\\d+\\.\\d+$" },
                                "compliance-level": {
                                    "type": "string",
                                    "enum": ["Bronze", "Silver", "Gold", "Platinum", "Diamond"]
                                },
                                "categories": {
                                    "type": "array",
                                    "items": { "type": "string" }
                                },
                                "triggers": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "Patterns that activate this skill"
                                },
                                "author": { "type": "string" },
                                "user-invocable": { "type": "boolean", "default": true },
                                "context": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "Required context (codebase, conversation)"
                                },
                                "depends-on": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "Skill dependencies"
                                }
                            },
                            "required": ["name", "version", "compliance-level"]
                        },
                        "spec": {
                            "type": "object",
                            "description": "Machine Specification sections",
                            "properties": {
                                "inputs": { "type": "string", "description": "INPUTS section content" },
                                "outputs": { "type": "string", "description": "OUTPUTS section content" },
                                "state": { "type": "string", "description": "STATE section content" },
                                "operator_mode": { "type": "string", "description": "OPERATOR MODE section" },
                                "performance": { "type": "string", "description": "PERFORMANCE section" },
                                "invariants": { "type": "string", "description": "INVARIANTS section" },
                                "failure_modes": { "type": "string", "description": "FAILURE_MODES section" },
                                "telemetry": { "type": "string", "description": "TELEMETRY section" }
                            }
                        },
                        "score": {
                            "type": "object",
                            "description": "SMST compliance scoring",
                            "properties": {
                                "total_score": { "type": "number", "minimum": 0, "maximum": 100 },
                                "sections_present": { "type": "integer", "minimum": 0 },
                                "sections_required": { "type": "integer", "minimum": 8 },
                                "has_frontmatter": { "type": "boolean" },
                                "has_machine_spec": { "type": "boolean" },
                                "compliance_level": { "type": "string" },
                                "missing_sections": {
                                    "type": "array",
                                    "items": { "type": "string" }
                                }
                            }
                        },
                        "is_diamond_compliant": { "type": "boolean" }
                    },
                    "required": ["frontmatter", "spec", "score"]
                });
                println!("{}", serde_json::to_string_pretty(&schema).unwrap());
                return;
            }

            // Smart path detection:
            // 1. If path is a .md file, use it directly
            // 2. If path is a skill directory (has SKILL.md), validate that skill
            // 3. If path is a parent directory, scan for skill subdirectories
            let p = std::path::Path::new(path);
            let paths: Vec<String> = if path.ends_with(".md") || p.is_file() {
                // Direct file path
                vec![path.clone()]
            } else if p.is_dir() {
                let skill_md_direct = p.join("SKILL.md");
                if skill_md_direct.exists() {
                    // This directory IS a skill - validate it directly
                    vec![skill_md_direct.to_string_lossy().to_string()]
                } else {
                    // This is a parent directory - scan for skill subdirectories
                    std::fs::read_dir(path)
                        .into_iter()
                        .flatten()
                        .filter_map(|e| e.ok())
                        .filter(|e| e.path().is_dir())
                        .map(|e| e.path().join("SKILL.md"))
                        .filter(|p| p.exists())
                        .map(|p| p.to_string_lossy().to_string())
                        .collect()
                }
            } else {
                vec![path.clone()]
            };

            let mut results = Vec::new();
            let mut all_passed = true;

            let diamond_threshold = rsk::lookup_compliance_level("diamond")
                .map(|l| l.min_score as f64)
                .unwrap_or(*threshold as f64);

            for skill_path in &paths {
                // If the path is a file (SKILL.md), get its parent directory
                let p = std::path::Path::new(skill_path);
                let skill_dir = if p.is_file() {
                    p.parent().unwrap_or(std::path::Path::new("."))
                } else {
                    p
                };

                let result = rsk::verify_skill(skill_dir);
                // Status is "success" if it meets the Diamond threshold and has all artifacts
                if result.status == "failed" || result.score < diamond_threshold {
                    all_passed = false;
                }
                if *verbose {
                    results.push(json!(result));
                } else {
                    results.push(json!({"name": result.skill_name, "score": result.score, "compliance_level": result.compliance_level, "passed": result.status == "success"}));
                }
            }

            match format.as_str() {
                "json" => {
                    if results.len() == 1 {
                        println!("{}", serde_json::to_string_pretty(&results[0]).unwrap());
                    } else {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&json!({
                                "status": if all_passed { "success" } else { "failed" },
                                "total": results.len(),
                                "passed": results.iter().filter(|r| r["passed"] == true).count(),
                                "results": results,
                            }))
                            .unwrap()
                        );
                    }
                }
                "summary" => {
                    let passed_count = results.iter().filter(|r| r["passed"] == true).count();
                    println!(
                        "Validation Summary: {}/{} skills passed (threshold: {}%)",
                        passed_count,
                        results.len(),
                        threshold
                    );
                    for r in &results {
                        let status = if r["passed"] == true { "✓" } else { "✗" };
                        println!("  {} {} - {:.1}%", status, r["name"], r["score"]);
                    }
                }
                "minimal" => {
                    // Just exit code - 0 if all passed, 1 if any failed
                    if !all_passed {
                        std::process::exit(1);
                    }
                }
                "report" => {
                    for r in &results {
                        if let Some(err) = r.get("error") {
                            println!("❌ ERROR: {}", err);
                            continue;
                        }

                        let name = r["name"].as_str().unwrap_or("unknown");
                        let score = r["score"].as_f64().unwrap_or(0.0);
                        let passed = r["passed"].as_bool().unwrap_or(false);
                        let missing = r["missing_sections"]
                            .as_array()
                            .cloned()
                            .unwrap_or_default();

                        println!(
                            "═══════════════════════════════════════════════════════════════════"
                        );
                        println!("SMST VALIDATION REPORT: {}", name);
                        println!(
                            "═══════════════════════════════════════════════════════════════════"
                        );
                        println!("");
                        println!(
                            "OVERALL: {}",
                            if passed {
                                "✅ DIAMOND READY"
                            } else {
                                "❌ NOT READY"
                            }
                        );
                        println!("Score: {:.1}/100 (Threshold: {}%)", score, threshold);
                        println!("");

                        println!(
                            "───────────────────────────────────────────────────────────────────"
                        );
                        println!("COMPONENT STATUS");
                        println!(
                            "───────────────────────────────────────────────────────────────────"
                        );

                        let all_sections = [
                            "INPUTS",
                            "OUTPUTS",
                            "STATE",
                            "OPERATOR MODE",
                            "PERFORMANCE",
                            "INVARIANTS",
                            "FAILURE MODES",
                            "TELEMETRY",
                        ];

                        for section in all_sections {
                            let is_missing = missing.iter().any(|m| m.as_str() == Some(section));
                            let status = if is_missing { "□ ✗" } else { "■ ✓" };
                            println!("{:<15} [{}]", section, status);
                        }

                        if !missing.is_empty() {
                            println!("");
                            println!(
                                "───────────────────────────────────────────────────────────────────"
                            );
                            println!("GAPS TO DIAMOND");
                            println!(
                                "───────────────────────────────────────────────────────────────────"
                            );
                            for m in missing {
                                println!(
                                    "- {}: Missing required section",
                                    m.as_str().unwrap_or("UNKNOWN")
                                );
                            }
                        }
                        println!(
                            "═══════════════════════════════════════════════════════════════════\n"
                        );
                    }
                }
                _ => {
                    eprintln!("Unknown format: {}", format);
                    std::process::exit(1);
                }
            }
        }
        Commands::Build { path, dry_run } => {
            // Smart path detection like Verify
            let p = std::path::Path::new(path);
            let paths: Vec<String> = if path.ends_with(".md") || p.is_file() {
                // Not standard for build, but handle gracefully
                vec![
                    p.parent()
                        .unwrap_or(std::path::Path::new("."))
                        .to_string_lossy()
                        .to_string(),
                ]
            } else if p.is_dir() {
                let skill_md_direct = p.join("SKILL.md");
                if skill_md_direct.exists() {
                    vec![path.clone()]
                } else {
                    std::fs::read_dir(path)
                        .into_iter()
                        .flatten()
                        .filter_map(|e| e.ok())
                        .filter(|e| e.path().is_dir())
                        .map(|e| e.path().to_string_lossy().to_string())
                        .filter(|d| std::path::Path::new(d).join("SKILL.md").exists())
                        .collect()
                }
            } else {
                vec![path.clone()]
            };

            let mut all_results = Vec::new();
            let mut all_success = true;

            for skill_dir in &paths {
                let p = std::path::Path::new(skill_dir);
                let result = rsk::build_skill(p, *dry_run);
                if result.status == "failed" {
                    all_success = false;
                }
                all_results.push(result);
            }

            if all_results.len() == 1 {
                println!("{}", serde_json::to_string_pretty(&all_results[0]).unwrap());
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "status": if all_success { "success" } else { "failed" },
                        "count": all_results.len(),
                        "results": all_results
                    }))
                    .unwrap()
                );
            }

            if !all_success {
                std::process::exit(1);
            }
        }
        Commands::Generate { action } => {
            match action {
                GenerateAction::Rules { path, format } => {
                    match fs::read_to_string(path) {
                        Ok(content) => {
                            let smst = rsk::extract_smst(&content);
                            let rules = generate_validation_rules(&smst);
                            match format.as_str() {
                                "rust" => {
                                    // Generate Rust validation code
                                    println!(
                                        "// Auto-generated validation rules for {}",
                                        rules.skill_name
                                    );
                                    println!("// Total rules: {}\n", rules.total_rules);
                                    for rule in &rules.invariant_rules {
                                        println!("/// {}", rule.description);
                                        println!("fn {}() -> bool {{", rule.id);
                                        println!("    // Condition: {}", rule.condition);
                                        println!("    todo!(\"Implement validation\")",);
                                        println!("}}\n");
                                    }
                                    for rule in &rules.failure_mode_rules {
                                        println!("/// {} [{}]", rule.description, rule.severity);
                                        println!("fn {}() -> Result<(), &'static str> {{", rule.id);
                                        println!("    // Error: {}", rule.error_message);
                                        println!("    todo!(\"Implement error handling\")");
                                        println!("}}\n");
                                    }
                                }
                                _ => {
                                    println!("{}", serde_json::to_string_pretty(&rules).unwrap());
                                }
                            }
                        }
                        Err(e) => {
                            println!("{}", json!({"status": "error", "message": e.to_string()}))
                        }
                    }
                }
                GenerateAction::Tests { path, format } => match fs::read_to_string(path) {
                    Ok(content) => {
                        let smst = rsk::extract_smst(&content);
                        let scaffold = generate_test_scaffold(&smst);
                        match format.as_str() {
                            "rust" => {
                                println!("{}", scaffold.rust_code);
                            }
                            _ => {
                                println!("{}", serde_json::to_string_pretty(&scaffold).unwrap());
                            }
                        }
                    }
                    Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
                },
                GenerateAction::Stub { path } => match fs::read_to_string(path) {
                    Ok(content) => {
                        let smst = rsk::extract_smst(&content);
                        let stub = rsk::generate_rust_stub(&smst);
                        println!("{}", stub.full_code);
                    }
                    Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
                },
                GenerateAction::Logic { path } => match fs::read_to_string(path) {
                    Ok(content) => {
                        let smst = rsk::extract_smst(&content);
                        let tree = rsk::generate_decision_tree(&smst);
                        match serde_yaml::to_string(&tree) {
                            Ok(yaml) => println!("{}", yaml),
                            Err(e) => {
                                println!("{}", json!({"status": "error", "message": e.to_string()}))
                            }
                        }
                    }
                    Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
                },
            }
        }
        Commands::Graph { action } => match action {
            GraphAction::TopSort { input } => match load_graph(input) {
                Ok(graph) => match graph.topological_sort() {
                    Ok(sorted) => println!("{}", json!({"status": "success", "result": sorted})),
                    Err(cycle) => println!(
                        "{}",
                        json!({"status": "error", "message": "Cycle detected", "cycle": cycle})
                    ),
                },
                Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
            },
            GraphAction::ShortestPath { input, start, end } => match load_graph(input) {
                Ok(graph) => match graph.shortest_path(start, end) {
                    Some((path, cost)) => println!(
                        "{}",
                        json!({"status": "success", "path": path, "cost": cost})
                    ),
                    None => println!("{}", json!({"status": "error", "message": "No path found"})),
                },
                Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
            },
            GraphAction::Levels { input } => match load_graph(input) {
                Ok(graph) => match graph.level_parallelization() {
                    Ok(levels) => {
                        let level_info: Vec<_> = levels
                            .iter()
                            .enumerate()
                            .map(|(i, nodes)| {
                                json!({
                                    "level": i,
                                    "parallel_count": nodes.len(),
                                    "nodes": nodes
                                })
                            })
                            .collect();
                        println!(
                            "{}",
                            json!({
                                "status": "success",
                                "total_levels": levels.len(),
                                "max_parallelism": levels.iter().map(|l| l.len()).max().unwrap_or(0),
                                "levels": level_info
                            })
                        );
                    }
                    Err(cycle) => println!(
                        "{}",
                        json!({"status": "error", "message": "Cycle detected", "cycle": cycle})
                    ),
                },
                Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
            },
        },
        Commands::Text { action } => match action {
            TextAction::Parse { path } => match fs::read_to_string(path) {
                Ok(content) => {
                    let result = rsk::parse_skill_md(&content);
                    println!("{}", json!(result));
                }
                Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
            },
            TextAction::Validate { path } => match fs::read_to_string(path) {
                Ok(content) => {
                    let parse_result = rsk::parse_skill_md(&content);
                    let errors = rsk::validate_diamond_spec(&parse_result);
                    if errors.is_empty() {
                        println!(
                            "{}",
                            json!({"status": "success", "message": "Diamond v2 compliant"})
                        );
                    } else {
                        println!("{}", json!({"status": "error", "errors": errors}));
                    }
                }
                Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
            },
            TextAction::Smst { path } => match fs::read_to_string(path) {
                Ok(content) => {
                    let result = rsk::extract_smst(&content);
                    println!("{}", serde_json::to_string_pretty(&result).unwrap());
                }
                Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
            },
            TextAction::Tokenize { text } => {
                let result = rsk::tokenize(text);
                println!("{}", serde_json::to_string_pretty(&result).unwrap());
            }
            TextAction::Normalize {
                text,
                strip_punctuation,
            } => {
                let result = rsk::normalize(text, *strip_punctuation);
                println!("{}", serde_json::to_string_pretty(&result).unwrap());
            }
            TextAction::Frequency { text, top } => {
                let result = rsk::word_frequency(text, *top);
                println!("{}", serde_json::to_string_pretty(&result).unwrap());
            }
            TextAction::Entropy { text } => {
                let result = rsk::analyze_compressibility(text);
                println!("{}", serde_json::to_string_pretty(&result).unwrap());
            }
            TextAction::Ngrams { text, n, words } => {
                let ngrams = rsk::extract_ngrams(text, *n, *words);
                println!(
                    "{}",
                    json!({
                        "n": n,
                        "mode": if *words { "word" } else { "character" },
                        "count": ngrams.len(),
                        "ngrams": ngrams,
                    })
                );
            }
            TextAction::Slugify { text } => {
                let result = rsk::slugify(text);
                println!(
                    "{}",
                    json!({
                        "original": text,
                        "slug": result,
                    })
                );
            }
        },
        Commands::Levenshtein { source, target } => {
            let result = levenshtein(source, target);
            println!("{}", json!(result));
        }
        Commands::Fuzzy {
            query,
            candidates,
            limit,
        } => {
            let candidate_list: Vec<String> = candidates
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
            let results = fuzzy_search(query, &candidate_list, *limit);
            println!("{}", json!({"status": "success", "matches": results}));
        }
        Commands::Sha256 { action } => match action {
            Sha256Action::Hash { input } => {
                let result = sha256_hash(input);
                println!("{}", json!(result));
            }
            Sha256Action::File { path } => match fs::read(path) {
                Ok(bytes) => {
                    let result = rsk::sha256_bytes(&bytes);
                    println!("{}", json!(result));
                }
                Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
            },
            Sha256Action::Verify { input, expected } => {
                let matches = sha256_verify(input, expected);
                println!("{}", json!({"matches": matches}));
            }
        },
        Commands::Version => {
            println!("rsk version {}", rsk::version());
        }
        Commands::Yaml { action } => match action {
            YamlAction::Parse { path } => match fs::read_to_string(path) {
                Ok(content) => match parse_yaml(&content) {
                    Ok(result) => println!("{}", serde_json::to_string_pretty(&result).unwrap()),
                    Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
                },
                Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
            },
            YamlAction::ParseStdin => {
                use std::io::{self, Read};
                let mut content = String::new();
                match io::stdin().read_to_string(&mut content) {
                    Ok(_) => match parse_yaml(&content) {
                        Ok(result) => {
                            println!("{}", serde_json::to_string_pretty(&result).unwrap())
                        }
                        Err(e) => {
                            println!("{}", json!({"status": "error", "message": e.to_string()}))
                        }
                    },
                    Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
                }
            }
            YamlAction::Toml { path } => match fs::read_to_string(path) {
                Ok(content) => match parse_toml(&content) {
                    Ok(result) => println!("{}", serde_json::to_string_pretty(&result).unwrap()),
                    Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
                },
                Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
            },
            YamlAction::Validate { path, schema } => match fs::read_to_string(path) {
                Ok(content) => {
                    let result = validate_schema(&content, schema.as_deref());
                    println!("{}", serde_json::to_string_pretty(&result).unwrap());
                }
                Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
            },
            YamlAction::DecisionTree { path } => match fs::read_to_string(path) {
                Ok(content) => match analyze_decision_tree(&content) {
                    Ok(analysis) => {
                        println!("{}", serde_json::to_string_pretty(&analysis).unwrap())
                    }
                    Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
                },
                Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
            },
            YamlAction::Taxonomy { path } => match fs::read_to_string(path) {
                Ok(content) => match extract_taxonomy_schema(&content) {
                    Ok(schema) => println!("{}", serde_json::to_string_pretty(&schema).unwrap()),
                    Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
                },
                Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
            },
            YamlAction::Frontmatter { path } => match fs::read_to_string(path) {
                Ok(content) => match parse_yaml_frontmatter(&content) {
                    Ok(frontmatter) => {
                        println!("{}", serde_json::to_string_pretty(&frontmatter).unwrap())
                    }
                    Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
                },
                Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
            },
            YamlAction::ExecuteLogic { tree, input } => {
                let tree_content = if tree.ends_with(".yaml") || tree.ends_with(".yml") {
                    fs::read_to_string(tree).unwrap_or_else(|_| tree.clone())
                } else {
                    tree.clone()
                };

                let tree: rsk::DecisionTree = match serde_yaml::from_str(&tree_content) {
                    Ok(t) => t,
                    Err(e) => {
                        eprintln!(
                            "{}",
                            json!({"status": "error", "message": format!("Invalid logic tree: {}", e)})
                        );
                        std::process::exit(1);
                    }
                };

                let engine = rsk::DecisionEngine::new(tree);
                let variables: HashMap<String, Value> =
                    serde_json::from_str(input).unwrap_or_default();
                let mut ctx = DecisionContext {
                    variables,
                    execution_path: Vec::new(),
                };

                let result = engine.execute(&mut ctx);
                println!("{}", serde_json::to_string_pretty(&json!({
                    "status": "success",
                    "execution_path": ctx.execution_path,
                    "result": match result {
                        ExecutionResult::Value(v) => json!(v),
                        ExecutionResult::LlmRequest { prompt, .. } => json!({"llm_fallback": prompt}),
                        ExecutionResult::Error(e) => json!({"error": e}),
                    }
                })).unwrap());
            }
        },
        Commands::Taxonomy { action } => match action {
            TaxonomyAction::Query { taxonomy_type, key } => {
                let result = query_taxonomy(taxonomy_type, key);
                println!("{}", serde_json::to_string_pretty(&result).unwrap());
            }
            TaxonomyAction::List { taxonomy_type } => {
                let result = list_taxonomy(taxonomy_type);
                println!("{}", serde_json::to_string_pretty(&result).unwrap());
            }
            TaxonomyAction::Compliance { level } => {
                let result = query_taxonomy("compliance", level);
                println!("{}", serde_json::to_string_pretty(&result).unwrap());
            }
            TaxonomyAction::Smst { component } => {
                let result = query_taxonomy("smst", component);
                println!("{}", serde_json::to_string_pretty(&result).unwrap());
            }
            TaxonomyAction::Category { category } => {
                let result = query_taxonomy("category", category);
                println!("{}", serde_json::to_string_pretty(&result).unwrap());
            }
        },
        Commands::Telemetry { action } => match action {
            TelemetryAction::Status => {
                let status = get_telemetry_status();
                println!("{}", serde_json::to_string_pretty(&status).unwrap());
            }
            TelemetryAction::Presets => {
                let presets = json!({
                    "presets": [
                        {
                            "name": "default",
                            "description": "Standard text output with timestamps",
                            "use_case": "Development and debugging"
                        },
                        {
                            "name": "json",
                            "description": "Structured JSON logging",
                            "use_case": "Log aggregation systems (ELK, Datadog)"
                        },
                        {
                            "name": "compact",
                            "description": "Minimal output without timestamps",
                            "use_case": "CI/CD pipelines and automated testing"
                        },
                        {
                            "name": "debug",
                            "description": "Verbose output with file/line info",
                            "use_case": "Troubleshooting and development"
                        }
                    ]
                });
                println!("{}", serde_json::to_string_pretty(&presets).unwrap());
            }
            TelemetryAction::Config { preset } => {
                let config = match preset.as_str() {
                    "json" => TelemetryConfig::json(),
                    "compact" => TelemetryConfig::compact(),
                    "debug" => TelemetryConfig::debug(),
                    _ => TelemetryConfig::default(),
                };
                println!("{}", serde_json::to_string_pretty(&config).unwrap());
            }
        },
        Commands::Compress { action } => {
            use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

            match action {
                CompressAction::Gzip { text, level } => {
                    let compression_level = match level.as_str() {
                        "fast" => rsk::CompressionLevel::Fast,
                        "best" => rsk::CompressionLevel::Best,
                        _ => rsk::CompressionLevel::Default,
                    };
                    let result = rsk::gzip_compress_string(text, compression_level);
                    println!(
                        "{}",
                        json!({
                            "original_size": result.original_size,
                            "compressed_size": result.compressed_size,
                            "ratio": result.ratio,
                            "savings_percent": result.savings_percent,
                            "data_base64": BASE64.encode(&result.data),
                        })
                    );
                }
                CompressAction::Gunzip { data } => match BASE64.decode(data) {
                    Ok(bytes) => match rsk::gzip_decompress_string(&bytes) {
                        Ok(text) => println!(
                            "{}",
                            json!({
                                "status": "success",
                                "text": text,
                            })
                        ),
                        Err(e) => println!(
                            "{}",
                            json!({
                                "status": "error",
                                "message": e,
                            })
                        ),
                    },
                    Err(e) => println!(
                        "{}",
                        json!({
                            "status": "error",
                            "message": format!("Invalid base64: {}", e),
                        })
                    ),
                },
                CompressAction::File {
                    path,
                    output,
                    level,
                } => {
                    let compression_level = match level.as_str() {
                        "fast" => rsk::CompressionLevel::Fast,
                        "best" => rsk::CompressionLevel::Best,
                        _ => rsk::CompressionLevel::Default,
                    };
                    match fs::read(path) {
                        Ok(data) => {
                            let result = rsk::gzip_compress(&data, compression_level);
                            let out_path = output.clone().unwrap_or_else(|| format!("{}.gz", path));
                            match fs::write(&out_path, &result.data) {
                                Ok(_) => println!(
                                    "{}",
                                    json!({
                                        "status": "success",
                                        "input_path": path,
                                        "output_path": out_path,
                                        "original_size": result.original_size,
                                        "compressed_size": result.compressed_size,
                                        "ratio": result.ratio,
                                        "savings_percent": result.savings_percent,
                                    })
                                ),
                                Err(e) => println!(
                                    "{}",
                                    json!({
                                        "status": "error",
                                        "message": format!("Failed to write output: {}", e),
                                    })
                                ),
                            }
                        }
                        Err(e) => println!(
                            "{}",
                            json!({
                                "status": "error",
                                "message": format!("Failed to read input: {}", e),
                            })
                        ),
                    }
                }
                CompressAction::Estimate { text } => {
                    let ratio = rsk::estimate_compressibility(text.as_bytes());
                    let compressibility = if ratio < 0.3 {
                        "highly_compressible"
                    } else if ratio < 0.6 {
                        "moderately_compressible"
                    } else if ratio < 0.8 {
                        "low_compressibility"
                    } else {
                        "incompressible"
                    };
                    println!(
                        "{}",
                        json!({
                            "estimated_ratio": ratio,
                            "compressibility": compressibility,
                            "input_size": text.len(),
                            "estimated_compressed_size": (text.len() as f64 * ratio).round() as usize,
                        })
                    );
                }
            }
        }
        #[cfg(feature = "forge")]
        Commands::Forge { action } => {
            use forge_spec::{load_spec, parse_spec};

            match action {
                ForgeAction::Validate { path } => match fs::read_to_string(path) {
                    Ok(content) => match load_spec(&content) {
                        Ok(spec) => {
                            let ingest_count = spec.ingest.len();
                            let transform_count = spec.transform.len();
                            let sink_count = spec.sink.len();
                            println!(
                                "{}",
                                json!({
                                    "status": "valid",
                                    "pipeline": spec.pipeline.name,
                                    "version": spec.pipeline.version,
                                    "ingest_sources": ingest_count,
                                    "transforms": transform_count,
                                    "sinks": sink_count,
                                })
                            );
                        }
                        Err(e) => {
                            println!(
                                "{}",
                                json!({
                                    "status": "invalid",
                                    "error": e.to_string(),
                                })
                            );
                            std::process::exit(1);
                        }
                    },
                    Err(e) => {
                        println!(
                            "{}",
                            json!({
                                "status": "error",
                                "message": format!("Failed to read file: {}", e),
                            })
                        );
                        std::process::exit(1);
                    }
                },
                ForgeAction::Parse { path } => match fs::read_to_string(path) {
                    Ok(content) => match parse_spec(&content) {
                        Ok(spec) => {
                            println!("{}", serde_json::to_string_pretty(&spec).unwrap());
                        }
                        Err(e) => {
                            println!("{}", json!({"status": "error", "message": e.to_string()}));
                            std::process::exit(1);
                        }
                    },
                    Err(e) => {
                        println!("{}", json!({"status": "error", "message": e.to_string()}));
                        std::process::exit(1);
                    }
                },
                ForgeAction::Graph { path } => {
                    match fs::read_to_string(path) {
                        Ok(content) => match parse_spec(&content) {
                            Ok(spec) => {
                                // Build a simple graph representation
                                let mut nodes = Vec::new();
                                let mut edges = Vec::new();

                                // Add ingest nodes
                                for ingest in &spec.ingest {
                                    nodes.push(json!({
                                        "id": &ingest.id,
                                        "type": "ingest",
                                        "source_type": format!("{:?}", ingest.source_type),
                                    }));
                                }

                                // Add transform nodes and edges
                                let mut prev_id: Option<&str> =
                                    spec.ingest.first().map(|i| i.id.as_str());
                                for transform in &spec.transform {
                                    nodes.push(json!({
                                        "id": &transform.id,
                                        "type": "transform",
                                        "operation": format!("{:?}", transform.operation),
                                    }));
                                    if let Some(prev) = prev_id {
                                        edges.push(json!({
                                            "from": prev,
                                            "to": &transform.id,
                                        }));
                                    }
                                    prev_id = Some(&transform.id);
                                }

                                // Connect last node to sinks
                                if let Some(last_id) = prev_id {
                                    for sink in &spec.sink {
                                        edges.push(json!({
                                            "from": last_id,
                                            "to": &sink.id,
                                        }));
                                    }
                                }

                                // Add sink nodes
                                for sink in &spec.sink {
                                    nodes.push(json!({
                                        "id": &sink.id,
                                        "type": "sink",
                                        "sink_type": format!("{:?}", sink.sink_type),
                                    }));
                                }

                                println!(
                                    "{}",
                                    serde_json::to_string_pretty(&json!({
                                        "pipeline": spec.pipeline.name,
                                        "nodes": nodes,
                                        "edges": edges,
                                    }))
                                    .unwrap()
                                );
                            }
                            Err(e) => {
                                println!(
                                    "{}",
                                    json!({"status": "error", "message": e.to_string()})
                                );
                                std::process::exit(1);
                            }
                        },
                        Err(e) => {
                            println!("{}", json!({"status": "error", "message": e.to_string()}));
                            std::process::exit(1);
                        }
                    }
                }
                ForgeAction::Sources => {
                    println!(
                        "{}",
                        json!({
                            "sources": [
                                {"type": "stdin", "description": "Standard input"},
                                {"type": "http_json", "description": "HTTP endpoint returning JSON"},
                                {"type": "http_csv", "description": "HTTP endpoint returning CSV"},
                                {"type": "s3_parquet", "description": "S3 bucket with Parquet files"},
                                {"type": "s3_json", "description": "S3 bucket with JSON files"},
                                {"type": "postgres", "description": "PostgreSQL database"},
                                {"type": "mysql", "description": "MySQL database"},
                                {"type": "sqlite", "description": "SQLite database"},
                            ]
                        })
                    );
                }
                ForgeAction::Transforms => {
                    println!(
                        "{}",
                        json!({
                            "transforms": [
                                {"operation": "filter", "description": "Filter rows by expression"},
                                {"operation": "select", "description": "Select/rename columns"},
                                {"operation": "aggregate", "description": "Group and aggregate data"},
                                {"operation": "join", "description": "Join with another source"},
                                {"operation": "deduplicate", "description": "Remove duplicate rows"},
                                {"operation": "chunk", "description": "Split into chunks for batching"},
                                {"operation": "embed", "description": "Generate embeddings for text"},
                                {"operation": "signal_detect_prr", "description": "PRR signal detection (pharmacovigilance)"},
                            ]
                        })
                    );
                }
                ForgeAction::Run {
                    path,
                    input,
                    dry_run,
                } => {
                    use std::io::{self, Read as IoRead, Write as IoWrite};

                    match fs::read_to_string(path) {
                        Ok(content) => match load_spec(&content) {
                            Ok(spec) => {
                                // Validate we can run this pipeline
                                let source = spec.ingest.first();
                                let sink = spec.sink.first();

                                let source_type = source.map(|s| format!("{:?}", s.source_type));
                                let sink_type = sink.map(|s| format!("{:?}", s.sink_type));

                                if *dry_run {
                                    println!(
                                        "{}",
                                        json!({
                                            "status": "dry_run",
                                            "pipeline": spec.pipeline.name,
                                            "source": source_type,
                                            "transforms": spec.transform.len(),
                                            "sink": sink_type,
                                            "would_execute": true,
                                        })
                                    );
                                    return;
                                }

                                // Get input data
                                let data: String = if let Some(input_str) = input {
                                    input_str.clone()
                                } else if source.is_some_and(|s| {
                                    matches!(s.source_type, forge_spec::SourceType::Stdin)
                                }) {
                                    let mut buffer = String::new();
                                    io::stdin().read_to_string(&mut buffer).unwrap_or_default();
                                    buffer
                                } else {
                                    eprintln!(
                                        "{}",
                                        json!({
                                            "status": "error",
                                            "message": "Source type not supported for direct execution. Use --input or stdin source.",
                                        })
                                    );
                                    std::process::exit(1);
                                };

                                // Apply transforms (basic JSON-aware processing)
                                let mut result = data;
                                for transform in &spec.transform {
                                    match &transform.operation {
                                        forge_spec::Operation::Deduplicate => {
                                            // For JSON arrays, deduplicate
                                            if let Ok(json_val) =
                                                serde_json::from_str::<serde_json::Value>(&result)
                                                && let Some(arr) = json_val.as_array()
                                            {
                                                let mut seen = std::collections::HashSet::new();
                                                let deduped: Vec<_> = arr
                                                    .iter()
                                                    .filter(|item| {
                                                        let key = item.to_string();
                                                        seen.insert(key)
                                                    })
                                                    .cloned()
                                                    .collect();
                                                result = serde_json::to_string_pretty(&deduped)
                                                    .unwrap_or(result);
                                            }
                                        }
                                        _ => {
                                            // Other transforms: pass through (log for debug)
                                            eprintln!(
                                                "Transform {:?} not yet implemented, passing through",
                                                transform.operation
                                            );
                                        }
                                    }
                                }

                                // Output to sink
                                if sink.is_some_and(|s| {
                                    matches!(s.sink_type, forge_spec::SinkType::Stdout)
                                }) {
                                    io::stdout().write_all(result.as_bytes()).unwrap();
                                    if !result.ends_with('\n') {
                                        io::stdout().write_all(b"\n").unwrap();
                                    }
                                } else if let Some(s) = sink {
                                    match &s.sink_type {
                                        forge_spec::SinkType::JsonFile => {
                                            if let Some(ref file_config) = s.file {
                                                fs::write(&file_config.path, &result)
                                                    .unwrap_or_else(|e| {
                                                        eprintln!("Failed to write JSON: {}", e);
                                                    });
                                                println!(
                                                    "{}",
                                                    json!({
                                                        "status": "success",
                                                        "output_path": file_config.path,
                                                        "bytes_written": result.len(),
                                                    })
                                                );
                                            }
                                        }
                                        _ => {
                                            eprintln!(
                                                "{}",
                                                json!({
                                                    "status": "error",
                                                    "message": format!("Sink type {:?} not yet supported for execution", s.sink_type),
                                                })
                                            );
                                            std::process::exit(1);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!(
                                    "{}",
                                    json!({
                                        "status": "error",
                                        "message": e.to_string(),
                                    })
                                );
                                std::process::exit(1);
                            }
                        },
                        Err(e) => {
                            eprintln!(
                                "{}",
                                json!({
                                    "status": "error",
                                    "message": format!("Failed to read file: {}", e),
                                })
                            );
                            std::process::exit(1);
                        }
                    }
                }
                ForgeAction::Sinks => {
                    println!(
                        "{}",
                        json!({
                            "sinks": [
                                {"type": "stdout", "description": "Standard output"},
                                {"type": "parquet", "description": "Parquet file"},
                                {"type": "json", "description": "JSON file"},
                                {"type": "csv", "description": "CSV file"},
                                {"type": "postgres", "description": "PostgreSQL database"},
                                {"type": "mysql", "description": "MySQL database"},
                                {"type": "sqlite", "description": "SQLite database"},
                                {"type": "qdrant", "description": "Qdrant vector database"},
                            ]
                        })
                    );
                }
            }
        }
        Commands::Exec { action } => {
            match action {
                ExecAction::Plan { modules } => {
                    // Load modules from JSON string or file
                    let content = if modules.ends_with(".json") {
                        fs::read_to_string(modules).unwrap_or_else(|e| {
                            eprintln!("{}", json!({"status": "error", "message": e.to_string()}));
                            std::process::exit(1);
                        })
                    } else {
                        modules.clone()
                    };

                    // Parse modules
                    let module_list: Vec<serde_json::Value> = match serde_json::from_str(&content) {
                        Ok(list) => list,
                        Err(e) => {
                            eprintln!(
                                "{}",
                                json!({"status": "error", "message": format!("Invalid JSON: {}", e)})
                            );
                            std::process::exit(1);
                        }
                    };

                    // Convert to ExecutionModule
                    let exec_modules: Vec<ExecutionModule> = module_list
                        .iter()
                        .map(|m| {
                            let id = m["id"].as_str().unwrap_or("unknown").to_string();
                            let name = m["name"].as_str().unwrap_or(&id).to_string();
                            let deps: Vec<String> = m["dependencies"]
                                .as_array()
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|v| v.as_str().map(String::from))
                                        .collect()
                                })
                                .unwrap_or_default();
                            let effort = m["effort"]
                                .as_str()
                                .and_then(EffortSize::from_str)
                                .unwrap_or(EffortSize::M);
                            let risk = m["risk"].as_f64().unwrap_or(0.3) as f32;

                            ExecutionModule::new(&id, &name, deps)
                                .with_effort(effort)
                                .with_risk(risk)
                        })
                        .collect();

                    // Build execution plan
                    match build_execution_plan(exec_modules) {
                        Ok(plan) => {
                            let conflicts = detect_resource_conflicts(&plan);
                            println!("{}", serde_json::to_string_pretty(&json!({
                                "status": "success",
                                "plan": {
                                    "total_modules": plan.modules.len(),
                                    "execution_order": plan.execution_order,
                                    "levels": plan.levels,
                                    "critical_path": plan.critical_path,
                                    "estimated_duration_minutes": plan.estimated_duration_minutes,
                                    "resource_conflicts": conflicts.len(),
                                }
                            })).unwrap());
                        }
                        Err(e) => {
                            eprintln!("{}", json!({"status": "error", "message": e.to_string()}));
                            std::process::exit(1);
                        }
                    }
                }
                ExecAction::Status { id } => {
                    let state_dir = dirs::home_dir()
                        .map(|h| h.join(".claude/chain-state"))
                        .unwrap_or_else(|| std::path::PathBuf::from("/tmp/chain-state"));

                    match CheckpointManager::new(state_dir.to_str().unwrap_or("/tmp/chain-state")) {
                        Ok(manager) => match manager.load(id) {
                            Ok(Some(ctx)) => {
                                println!(
                                    "{}",
                                    serde_json::to_string_pretty(&json!({
                                        "status": "found",
                                        "context": {
                                            "id": ctx.id,
                                            "name": ctx.name,
                                            "status": format!("{:?}", ctx.status),
                                            "progress_percent": ctx.progress_percent(),
                                            "completed_steps": ctx.completed_steps.len(),
                                            "failed_steps": ctx.failed_steps.len(),
                                            "total_steps": ctx.total_steps,
                                        }
                                    }))
                                    .unwrap()
                                );
                            }
                            Ok(None) => {
                                println!("{}", json!({"status": "not_found", "id": id}));
                            }
                            Err(e) => {
                                eprintln!(
                                    "{}",
                                    json!({"status": "error", "message": e.to_string()})
                                );
                            }
                        },
                        Err(e) => {
                            eprintln!("{}", json!({"status": "error", "message": e.to_string()}));
                        }
                    }
                }
                ExecAction::Resume { id } => {
                    let state_dir = dirs::home_dir()
                        .map(|h| h.join(".claude/chain-state"))
                        .unwrap_or_else(|| std::path::PathBuf::from("/tmp/chain-state"));

                    match CheckpointManager::new(state_dir.to_str().unwrap_or("/tmp/chain-state")) {
                        Ok(manager) => match manager.load(id) {
                            Ok(Some(ctx)) => {
                                let next = ctx.next_step();
                                println!(
                                    "{}",
                                    serde_json::to_string_pretty(&json!({
                                        "status": "resumable",
                                        "context": {
                                            "id": ctx.id,
                                            "name": ctx.name,
                                            "next_step": next,
                                            "completed_steps": ctx.completed_steps,
                                            "total_steps": ctx.total_steps,
                                        }
                                    }))
                                    .unwrap()
                                );
                            }
                            Ok(None) => {
                                println!("{}", json!({"status": "not_found", "id": id}));
                            }
                            Err(e) => {
                                eprintln!(
                                    "{}",
                                    json!({"status": "error", "message": e.to_string()})
                                );
                            }
                        },
                        Err(e) => {
                            eprintln!("{}", json!({"status": "error", "message": e.to_string()}));
                        }
                    }
                }
                ExecAction::Validate { modules } => {
                    // Same as Plan but just validates without saving
                    let content = if modules.ends_with(".json") {
                        fs::read_to_string(modules).unwrap_or_else(|e| {
                            eprintln!("{}", json!({"status": "error", "message": e.to_string()}));
                            std::process::exit(1);
                        })
                    } else {
                        modules.clone()
                    };

                    let module_list: Vec<serde_json::Value> = match serde_json::from_str(&content) {
                        Ok(list) => list,
                        Err(e) => {
                            eprintln!(
                                "{}",
                                json!({"status": "error", "message": format!("Invalid JSON: {}", e)})
                            );
                            std::process::exit(1);
                        }
                    };

                    let exec_modules: Vec<ExecutionModule> = module_list
                        .iter()
                        .map(|m| {
                            let id = m["id"].as_str().unwrap_or("unknown").to_string();
                            let name = m["name"].as_str().unwrap_or(&id).to_string();
                            let deps: Vec<String> = m["dependencies"]
                                .as_array()
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|v| v.as_str().map(String::from))
                                        .collect()
                                })
                                .unwrap_or_default();
                            ExecutionModule::new(&id, &name, deps)
                        })
                        .collect();

                    match build_execution_plan(exec_modules) {
                        Ok(plan) => {
                            let conflicts = detect_resource_conflicts(&plan);
                            println!(
                                "{}",
                                json!({
                                    "status": "valid",
                                    "modules": plan.modules.len(),
                                    "levels": plan.levels.len(),
                                    "conflicts": conflicts.len(),
                                })
                            );
                        }
                        Err(e) => {
                            println!("{}", json!({"status": "invalid", "error": e.to_string()}));
                            std::process::exit(1);
                        }
                    }
                }
            }
        }
        Commands::Server { socket } => {
            let path = socket
                .clone()
                .unwrap_or_else(|| "/tmp/rsk-state.sock".to_string());
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let server = rsk::StateServer::new(&path);
                server.run().await.unwrap();
            });
        }
        Commands::Route { action } => {
            match action {
                RouteAction::Find {
                    query,
                    source,
                    strategy,
                    limit,
                } => {
                    let engine = RoutingEngine::new();
                    // Note: In production, engine would be loaded with skill capabilities
                    // For now, show what the interface would return

                    let strat =
                        RoutingStrategy::from_str(strategy).unwrap_or(RoutingStrategy::Hybrid);
                    let request = RoutingRequest {
                        source: source.clone().unwrap_or_default(),
                        context: query.clone(),
                        strategy: strat,
                        limit: *limit,
                    };

                    match engine.route(&request) {
                        Ok(result) => {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&json!({
                                    "status": "success",
                                    "query": query,
                                    "strategy": format!("{:?}", result.strategy),
                                    "recommendations": result.recommendations.iter().map(|r| json!({
                                        "target": r.target,
                                        "score": r.score,
                                        "confidence": r.confidence,
                                        "reasoning": r.reasoning,
                                    })).collect::<Vec<_>>(),
                                    "total_considered": result.total_considered,
                                    "duration_ms": result.duration_ms,
                                }))
                                .unwrap()
                            );
                        }
                        Err(e) => {
                            eprintln!("{}", json!({"status": "error", "message": e.to_string()}));
                        }
                    }
                }
                RouteAction::Strategies => {
                    println!(
                        "{}",
                        json!({
                            "strategies": [
                                {
                                    "name": "adjacency",
                                    "weight": 0.5,
                                    "description": "Graph-based routing using skill adjacency edges"
                                },
                                {
                                    "name": "capability",
                                    "weight": 0.3,
                                    "description": "Pattern matching on skill triggers and handles"
                                },
                                {
                                    "name": "semantic",
                                    "weight": 0.2,
                                    "description": "Keyword similarity using Levenshtein distance"
                                },
                                {
                                    "name": "hybrid",
                                    "weight": 1.0,
                                    "description": "Weighted combination of all strategies (default)"
                                }
                            ]
                        })
                    );
                }
                RouteAction::Fuzzy { query, limit } => {
                    // This uses the existing fuzzy_search functionality
                    // In production, would load actual skill names from index
                    let example_skills = vec![
                        "proceed".to_string(),
                        "process".to_string(),
                        "skill-validator".to_string(),
                        "topological-sort".to_string(),
                        "level-parallelization".to_string(),
                        "execution-engine".to_string(),
                    ];

                    let results = fuzzy_search(query, &example_skills, *limit);
                    println!(
                        "{}",
                        json!({
                            "status": "success",
                            "query": query,
                            "matches": results,
                        })
                    );
                }
            }
        }
        Commands::State { action } => {
            let state_dir = dirs::home_dir()
                .map(|h| h.join(".claude/chain-state"))
                .unwrap_or_else(|| std::path::PathBuf::from("/tmp/chain-state"));

            match CheckpointManager::new(state_dir.to_str().unwrap_or("/tmp/chain-state")) {
                Ok(mut manager) => {
                    match action {
                        StateAction::List { name, status } => {
                            let contexts = if let Some(n) = name {
                                manager.list_by_name(n).unwrap_or_default()
                            } else {
                                manager.list().unwrap_or_default()
                            };

                            // Filter by status if specified
                            let filtered: Vec<_> = if let Some(s) = status {
                                contexts
                                    .into_iter()
                                    .filter(|ctx| {
                                        format!("{:?}", ctx.status)
                                            .to_lowercase()
                                            .contains(&s.to_lowercase())
                                    })
                                    .collect()
                            } else {
                                contexts
                            };

                            println!(
                                "{}",
                                serde_json::to_string_pretty(&json!({
                                    "status": "success",
                                    "count": filtered.len(),
                                    "checkpoints": filtered.iter().map(|ctx| json!({
                                        "id": ctx.id,
                                        "name": ctx.name,
                                        "status": format!("{:?}", ctx.status),
                                        "progress": ctx.progress_percent(),
                                        "updated_at": ctx.updated_at.to_rfc3339(),
                                    })).collect::<Vec<_>>(),
                                }))
                                .unwrap()
                            );
                        }
                        StateAction::Show { id } => match manager.load(id) {
                            Ok(Some(ctx)) => {
                                println!("{}", serde_json::to_string_pretty(&ctx).unwrap());
                            }
                            Ok(None) => {
                                println!("{}", json!({"status": "not_found", "id": id}));
                            }
                            Err(e) => {
                                eprintln!(
                                    "{}",
                                    json!({"status": "error", "message": e.to_string()})
                                );
                            }
                        },
                        StateAction::Delete { id } => match manager.delete(id) {
                            Ok(true) => {
                                println!("{}", json!({"status": "deleted", "id": id}));
                            }
                            Ok(false) => {
                                println!("{}", json!({"status": "not_found", "id": id}));
                            }
                            Err(e) => {
                                eprintln!(
                                    "{}",
                                    json!({"status": "error", "message": e.to_string()})
                                );
                            }
                        },
                        StateAction::Cleanup { max_age } => match manager.cleanup(*max_age) {
                            Ok(count) => {
                                println!(
                                    "{}",
                                    json!({
                                        "status": "success",
                                        "removed": count,
                                        "max_age_days": max_age,
                                    })
                                );
                            }
                            Err(e) => {
                                eprintln!(
                                    "{}",
                                    json!({"status": "error", "message": e.to_string()})
                                );
                            }
                        },
                        StateAction::Stats => match manager.stats() {
                            Ok(stats) => {
                                println!("{}", serde_json::to_string_pretty(&stats).unwrap());
                            }
                            Err(e) => {
                                eprintln!(
                                    "{}",
                                    json!({"status": "error", "message": e.to_string()})
                                );
                            }
                        },
                    }
                }
                Err(e) => {
                    eprintln!("{}", json!({"status": "error", "message": e.to_string()}));
                    std::process::exit(1);
                }
            }
        }
        Commands::Skills { action } => {
            let default_registry_path = dirs::home_dir()
                .map(|h| h.join(".rsk/skills.json"))
                .unwrap_or_else(|| PathBuf::from("skills.json"));

            match action {
                SkillsAction::Scan { path, output } => {
                    let mut registry = SkillRegistry::new();
                    if let Err(e) = registry.load_from_directory(&path) {
                        eprintln!("{}", json!({"status": "error", "message": e}));
                        std::process::exit(1);
                    }

                    let out_path = output
                        .as_ref()
                        .map(PathBuf::from)
                        .unwrap_or(default_registry_path);
                    if let Some(parent) = out_path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }

                    if let Err(e) = registry.save(&out_path) {
                        eprintln!("{}", json!({"status": "error", "message": e}));
                        std::process::exit(1);
                    }

                    println!("{}", serde_json::to_string_pretty(&json!({
                        "status": "success",
                        "message": format!("Scanned {} skills and saved to {:?}", registry.skills.len(), out_path),
                        "count": registry.skills.len(),
                    })).unwrap());
                }
                SkillsAction::List { registry, strategy } => {
                    let reg_path = registry
                        .as_ref()
                        .map(PathBuf::from)
                        .unwrap_or(default_registry_path);
                    let reg = match SkillRegistry::load(&reg_path) {
                        Ok(r) => r,
                        Err(e) => {
                            eprintln!(
                                "{}",
                                json!({"status": "error", "message": format!("Failed to load registry: {}", e)})
                            );
                            std::process::exit(1);
                        }
                    };

                    let mut skills = reg.list();
                    if let Some(s) = strategy {
                        skills.retain(|entry| {
                            format!("{:?}", entry.strategy)
                                .to_lowercase()
                                .contains(&s.to_lowercase())
                        });
                    }

                    println!("{}", serde_json::to_string_pretty(&skills).unwrap());
                }
                SkillsAction::Info { name, registry } => {
                    let reg_path = registry
                        .as_ref()
                        .map(PathBuf::from)
                        .unwrap_or(default_registry_path);
                    let reg = match SkillRegistry::load(&reg_path) {
                        Ok(r) => r,
                        Err(e) => {
                            eprintln!("{}", json!({"status": "error", "message": e}));
                            std::process::exit(1);
                        }
                    };

                    match reg.get(&name) {
                        Some(skill) => println!("{}", serde_json::to_string_pretty(skill).unwrap()),
                        None => {
                            eprintln!("{}", json!({"status": "not_found", "name": name}));
                            std::process::exit(1);
                        }
                    }
                }
                SkillsAction::Execute {
                    name,
                    input,
                    registry,
                } => {
                    let reg_path = registry
                        .as_ref()
                        .map(PathBuf::from)
                        .unwrap_or(default_registry_path);
                    let reg = match SkillRegistry::load(&reg_path) {
                        Ok(r) => r,
                        Err(e) => {
                            eprintln!("{}", json!({"status": "error", "message": e}));
                            std::process::exit(1);
                        }
                    };

                    let skill = match reg.get(&name) {
                        Some(s) => s,
                        None => {
                            eprintln!("{}", json!({"status": "not_found", "name": name}));
                            std::process::exit(1);
                        }
                    };

                    if let Some(logic_path) = &skill.logic_path {
                        let logic_content = fs::read_to_string(logic_path).unwrap();
                        let tree: rsk::DecisionTree = serde_yaml::from_str(&logic_content).unwrap();
                        let engine = rsk::DecisionEngine::new(tree);

                        let variables: HashMap<String, Value> =
                            serde_json::from_str(&input).unwrap();
                        let mut ctx = DecisionContext {
                            variables,
                            execution_path: Vec::new(),
                        };

                        let result = engine.execute(&mut ctx);
                        println!("{}", serde_json::to_string_pretty(&json!({
                            "status": "success",
                            "skill": name,
                            "execution_path": ctx.execution_path,
                            "result": match result {
                                ExecutionResult::Value(v) => json!(v),
                                ExecutionResult::LlmRequest { prompt, .. } => json!({"llm_fallback": prompt}),
                                ExecutionResult::Error(e) => json!({"error": e}),
                            }
                        })).unwrap());
                    } else {
                        println!(
                            "{}",
                            json!({
                                "status": "unsupported",
                                "message": "Skill has no logic.yaml and cannot be executed deterministically yet",
                                "strategy": format!("{:?}", skill.strategy),
                            })
                        );
                    }
                }
            }
        }
        Commands::Chain { action } => {
            let default_registry_path = dirs::home_dir()
                .map(|h| h.join(".rsk/skills.json"))
                .unwrap_or_else(|| PathBuf::from("skills.json"));

            match action {
                ChainAction::Validate {
                    name,
                    depth,
                    registry,
                } => {
                    let reg_path = registry
                        .as_ref()
                        .map(PathBuf::from)
                        .unwrap_or(default_registry_path);
                    let reg = match SkillRegistry::load(&reg_path) {
                        Ok(r) => r,
                        Err(e) => {
                            eprintln!("{}", json!({"status": "error", "message": e}));
                            std::process::exit(1);
                        }
                    };

                    println!("═══════════════════════════════════════════════════════════════════");
                    println!("CHAIN VALIDATION: {} (Depth: {})", name, depth);
                    println!("═══════════════════════════════════════════════════════════════════");
                    println!("");

                    let chain_results = reg.validate_chain(&name, *depth);
                    let mut all_diamond = true;

                    for (skill, passed, score) in chain_results {
                        let status = if passed {
                            "✅ DIAMOND"
                        } else {
                            "❌ NOT READY"
                        };
                        if !passed {
                            all_diamond = false;
                        }
                        println!("{:<30} | {:<15} | {:.1}%", skill, status, score);
                    }

                    println!("");
                    if all_diamond {
                        println!("STATUS: 💎 FULL CHAIN VALIDATED");
                    } else {
                        println!("STATUS: ⚠️ GAPS DETECTED IN CHAIN");
                    }
                    println!("═══════════════════════════════════════════════════════════════════");
                }
            }
        }
        Commands::Evolve { name, registry } => {
            let default_registry_path = dirs::home_dir()
                .map(|h| h.join(".rsk/skills.json"))
                .unwrap_or_else(|| PathBuf::from("skills.json"));

            let reg_path = registry
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or(default_registry_path);
            let reg = match SkillRegistry::load(&reg_path) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("{}", json!({"status": "error", "message": e}));
                    std::process::exit(1);
                }
            };

            let skill = match reg.get(&name) {
                Some(s) => s,
                None => {
                    eprintln!("{}", json!({"status": "not_found", "name": name}));
                    std::process::exit(1);
                }
            };

            if let Some(logic_path) = &skill.logic_path {
                let logic_content = fs::read_to_string(logic_path).unwrap();
                let tree: rsk::DecisionTree = serde_yaml::from_str(&logic_content).unwrap();

                // 1. Synthesize Code
                let code = rsk::synthesize_intrinsic(&name, &tree);
                let out_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("src/modules/dynamic_intrinsics.rs");

                fs::write(&out_path, code).unwrap();

                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "status": "evolved",
                        "skill": name,
                        "generated_file": out_path.to_string_lossy(),
                        "message": "Logic synthesized. Run 'cargo build' to integrate.",
                    }))
                    .unwrap()
                );
            } else {
                eprintln!(
                    "{}",
                    json!({"status": "error", "message": "Skill has no logic to evolve"})
                );
            }
        }
        Commands::Hooks { action } => {
            use rsk::hooks::{
                blindspot::BlindspotCheck,
                policy::PolicyFile,
                scanner::{format_scan_result, scan_directory},
                staleness::{check_staleness, format_staleness_result},
                validation::{categorize_file, format_validation_result, validate_file},
            };

            let policy = PolicyFile::load_or_default(None);

            match action {
                HooksAction::Validate { path, format } => {
                    let path = PathBuf::from(&path);
                    let result = validate_file(&path, &policy);

                    if format == "json" {
                        println!("{}", serde_json::to_string_pretty(&result).unwrap());
                    } else {
                        let formatted = format_validation_result(&result);
                        if !formatted.is_empty() {
                            println!("{}", formatted);
                        } else {
                            println!("[OK] {} - no policy violations", path.display());
                        }
                    }
                }
                HooksAction::Staleness { path, format } => {
                    let path = PathBuf::from(&path);
                    let result = check_staleness(&path, &policy);

                    if format == "json" {
                        println!("{}", serde_json::to_string_pretty(&result).unwrap());
                    } else {
                        println!("{}", format_staleness_result(&result));
                    }
                }
                HooksAction::Categorize { path } => {
                    let path = PathBuf::from(&path);
                    let category = categorize_file(&path, &policy);
                    println!("{}", category);
                }
                HooksAction::Scan {
                    path,
                    depth,
                    format,
                } => {
                    let path = PathBuf::from(&path);
                    let result = scan_directory(&path, *depth, &policy);

                    if format == "json" {
                        println!("{}", serde_json::to_string_pretty(&result).unwrap());
                    } else {
                        println!("{}", format_scan_result(&result));
                    }
                }
                HooksAction::Policy => {
                    println!("=== File Organization Policy ===\n");

                    if let Some(settings) = &policy.settings {
                        println!("Settings:");
                        println!("  Mode: {}", settings.mode.as_deref().unwrap_or("advisory"));
                        println!(
                            "  Stale action: {}",
                            settings.stale_action.as_deref().unwrap_or("report")
                        );
                        println!();
                    }

                    if let Some(rules) = &policy.placement_rules {
                        println!("Placement Rules ({} categories):", rules.len());
                        for (name, rule) in rules {
                            println!("  {}:", name);
                            println!("    Patterns: {:?}", rule.patterns);
                            if !rule.forbidden_paths.is_empty() {
                                println!("    Forbidden: {:?}", rule.forbidden_paths);
                            }
                            if !rule.recommended_paths.is_empty() {
                                println!("    Recommended: {:?}", rule.recommended_paths);
                            }
                        }
                        println!();
                    }

                    if let Some(staleness) = &policy.staleness {
                        println!("Staleness Rules:");
                        println!("  Default: {} days", staleness.default_days.unwrap_or(30));
                        if let Some(rules) = &staleness.path_rules {
                            for (pattern, rule) in rules {
                                println!(
                                    "  {}: {} days ({})",
                                    pattern,
                                    rule.days.unwrap_or(30),
                                    rule.action.as_deref().unwrap_or("report")
                                );
                            }
                        }
                    }
                }
                HooksAction::Blindspot { path, format } => {
                    let path = PathBuf::from(&path);
                    let check = BlindspotCheck::for_file(&path, &policy);

                    if format == "json" {
                        println!("{}", serde_json::to_string_pretty(&check).unwrap());
                    } else {
                        println!("{}", check.message);
                        println!("\nChecklist:");
                        for item in &check.items {
                            println!("  - {}", item);
                        }
                    }
                }
                HooksAction::SubagentReview {
                    agent_type,
                    description,
                } => {
                    let check = BlindspotCheck::for_subagent(&agent_type, &description);
                    println!("{}", check.message);
                }
                HooksAction::SchemaVersion => {
                    println!("{}", rsk::hooks::SCHEMA_VERSION);
                }
            }
        }
        Commands::Tov { action } => {
            use rsk::tov::{
                AlgorithmCorrectness, CharacterizedHarmEvent, ClinicalOutcome, ClinicianResponse,
                ConservationLaw, Determinism, HarmCharacteristics, HarmType, KHSAI, Multiplicity,
                PropagationProbability, Temporal, analyze_attenuation, classify_harm,
                determine_aca_case, harm_type_characteristics, interpret_khs_ai, protective_depth,
            };

            match action {
                TovAction::Classify { mult, temp, det } => {
                    let multiplicity = match mult.to_lowercase().as_str() {
                        "single" => Multiplicity::Single,
                        "multiple" => Multiplicity::Multiple,
                        _ => {
                            eprintln!("Error: mult must be 'single' or 'multiple'");
                            std::process::exit(1);
                        }
                    };
                    let temporal = match temp.to_lowercase().as_str() {
                        "acute" => Temporal::Acute,
                        "chronic" => Temporal::Chronic,
                        _ => {
                            eprintln!("Error: temp must be 'acute' or 'chronic'");
                            std::process::exit(1);
                        }
                    };
                    let determinism = match det.to_lowercase().as_str() {
                        "deterministic" => Determinism::Deterministic,
                        "stochastic" => Determinism::Stochastic,
                        _ => {
                            eprintln!("Error: det must be 'deterministic' or 'stochastic'");
                            std::process::exit(1);
                        }
                    };

                    let event = CharacterizedHarmEvent {
                        characteristics: HarmCharacteristics {
                            multiplicity,
                            temporal,
                            determinism,
                        },
                    };
                    let harm_type = classify_harm(event);
                    println!(
                        "{}",
                        json!({
                            "harm_type": format!("{:?}", harm_type),
                            "multiplicity": format!("{:?}", multiplicity),
                            "temporal": format!("{:?}", temporal),
                            "determinism": format!("{:?}", determinism),
                        })
                    );
                }
                TovAction::Attenuation { probs } => {
                    let probabilities: Result<Vec<PropagationProbability>, _> = probs
                        .split(',')
                        .map(|s| {
                            s.trim()
                                .parse::<f64>()
                                .map_err(|e| e.to_string())
                                .and_then(|v| {
                                    if v > 0.0 && v < 1.0 {
                                        Ok(PropagationProbability::new(v))
                                    } else {
                                        Err("Probability must be in (0, 1)".to_string())
                                    }
                                })
                        })
                        .collect();

                    match probabilities {
                        Ok(probs) => {
                            let result = analyze_attenuation(&probs);
                            println!("{}", serde_json::to_string_pretty(&result).unwrap());
                        }
                        Err(e) => {
                            eprintln!("Error parsing probabilities: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                TovAction::ProtectiveDepth { target, alpha } => {
                    if *target <= 0.0 || *target >= 1.0 {
                        eprintln!("Error: target must be in (0, 1)");
                        std::process::exit(1);
                    }
                    if *alpha <= 0.0 {
                        eprintln!("Error: alpha must be positive");
                        std::process::exit(1);
                    }
                    let depth = protective_depth(*target, *alpha);
                    println!(
                        "{}",
                        json!({
                            "target_probability": target,
                            "attenuation_rate": alpha,
                            "protective_depth": depth,
                        })
                    );
                }
                TovAction::Aca {
                    correctness,
                    response,
                    outcome,
                } => {
                    let alg_correctness = match correctness.to_lowercase().as_str() {
                        "correct" => AlgorithmCorrectness::Correct,
                        "wrong" => AlgorithmCorrectness::Wrong,
                        _ => {
                            eprintln!("Error: correctness must be 'correct' or 'wrong'");
                            std::process::exit(1);
                        }
                    };
                    let clin_response = match response.to_lowercase().as_str() {
                        "followed" => ClinicianResponse::Followed,
                        "overrode" => ClinicianResponse::Overrode,
                        _ => {
                            eprintln!("Error: response must be 'followed' or 'overrode'");
                            std::process::exit(1);
                        }
                    };
                    let clin_outcome = match outcome.to_lowercase().as_str() {
                        "good" => ClinicalOutcome::Good,
                        "harm" => ClinicalOutcome::Harm,
                        _ => {
                            eprintln!("Error: outcome must be 'good' or 'harm'");
                            std::process::exit(1);
                        }
                    };

                    let case = determine_aca_case(alg_correctness, clin_response, clin_outcome);
                    let propagation = rsk::tov::case_propagation_factor(case);
                    println!(
                        "{}",
                        json!({
                            "case": format!("{:?}", case),
                            "propagation_factor": propagation,
                            "description": match case {
                                rsk::tov::ACACase::CaseI => "Incident - algorithm wrong, followed, harm occurred",
                                rsk::tov::ACACase::CaseII => "Exculpated - algorithm correct, overridden, harm occurred",
                                rsk::tov::ACACase::CaseIII => "Signal - algorithm wrong, overridden (near-miss)",
                                rsk::tov::ACACase::CaseIV => "Baseline - algorithm correct, followed, good outcome",
                            },
                        })
                    );
                }
                TovAction::Khs {
                    latency,
                    accuracy,
                    resource,
                    drift,
                } => {
                    let khs = KHSAI::calculate(*latency, *accuracy, *resource, *drift);
                    let status = interpret_khs_ai(khs.overall);
                    println!(
                        "{}",
                        json!({
                            "overall": khs.overall,
                            "status": format!("{:?}", status),
                            "latency_stability": khs.latency_stability,
                            "accuracy_stability": khs.accuracy_stability,
                            "resource_efficiency": khs.resource_efficiency,
                            "drift_score": khs.drift_score,
                        })
                    );
                }
                TovAction::HarmTypes => {
                    let types = [
                        HarmType::Acute,
                        HarmType::Cumulative,
                        HarmType::OffTarget,
                        HarmType::Cascade,
                        HarmType::Idiosyncratic,
                        HarmType::Saturation,
                        HarmType::Interaction,
                        HarmType::Population,
                    ];
                    let output: Vec<_> = types
                        .iter()
                        .map(|t| {
                            let chars = harm_type_characteristics(*t);
                            json!({
                                "type": format!("{:?}", t),
                                "multiplicity": format!("{:?}", chars.multiplicity),
                                "temporal": format!("{:?}", chars.temporal),
                                "determinism": format!("{:?}", chars.determinism),
                            })
                        })
                        .collect();
                    println!("{}", serde_json::to_string_pretty(&output).unwrap());
                }
                TovAction::ConservationLaws => {
                    let laws = [
                        ConservationLaw::Mass,
                        ConservationLaw::Energy,
                        ConservationLaw::State,
                        ConservationLaw::Flux,
                        ConservationLaw::Catalyst,
                        ConservationLaw::Rate,
                        ConservationLaw::Equilibrium,
                        ConservationLaw::Saturation,
                        ConservationLaw::Entropy,
                        ConservationLaw::Discretization,
                        ConservationLaw::Structure,
                    ];
                    let output: Vec<_> = laws
                        .iter()
                        .map(|l| {
                            json!({
                                "index": l.index(),
                                "name": format!("{:?}", l),
                                "type": format!("{:?}", l.law_type()),
                            })
                        })
                        .collect();
                    println!("{}", serde_json::to_string_pretty(&output).unwrap());
                }
            }
        }
        Commands::Guardian { action } => {
            use rsk::guardian::iair::{
                CheckabilityLevel, ExpertiseLevel, OutputTreatment, StakesLevel,
            };
            use rsk::guardian::{
                ContextRiskParams, IAIRBuilder, IncidentCategory, OutcomeType, calculate_risk,
                recommend_minimization,
            };

            match action {
                GuardianAction::Risk {
                    stakes,
                    expertise,
                    checkability,
                    output,
                } => {
                    let stakes_level = match stakes.to_lowercase().as_str() {
                        "low" => StakesLevel::Low,
                        "moderate" => StakesLevel::Moderate,
                        "high" => StakesLevel::High,
                        "critical" => StakesLevel::Critical,
                        _ => {
                            eprintln!("Error: stakes must be low, moderate, high, or critical");
                            std::process::exit(1);
                        }
                    };
                    let expertise_level = match expertise.to_lowercase().as_str() {
                        "low" => ExpertiseLevel::Low,
                        "moderate" => ExpertiseLevel::Moderate,
                        "high" => ExpertiseLevel::High,
                        "unknown" => ExpertiseLevel::Unknown,
                        _ => {
                            eprintln!("Error: expertise must be low, moderate, high, or unknown");
                            std::process::exit(1);
                        }
                    };
                    let checkability_level = match checkability.to_lowercase().as_str() {
                        "low" => CheckabilityLevel::Low,
                        "moderate" => CheckabilityLevel::Moderate,
                        "high" => CheckabilityLevel::High,
                        "unfalsifiable" => CheckabilityLevel::Unfalsifiable,
                        _ => {
                            eprintln!(
                                "Error: checkability must be low, moderate, high, or unfalsifiable"
                            );
                            std::process::exit(1);
                        }
                    };
                    let output_treatment = match output.to_lowercase().as_str() {
                        "draft" => OutputTreatment::Draft,
                        "reviewed" => OutputTreatment::Reviewed,
                        "direct_use" => OutputTreatment::DirectUse,
                        "published" => OutputTreatment::Published,
                        _ => {
                            eprintln!(
                                "Error: output must be draft, reviewed, direct_use, or published"
                            );
                            std::process::exit(1);
                        }
                    };

                    let params = ContextRiskParams {
                        stakes: stakes_level,
                        expertise: expertise_level,
                        checkability: checkability_level,
                        output_treatment,
                    };
                    let result = calculate_risk(&params);
                    println!("{}", serde_json::to_string_pretty(&result).unwrap());
                }
                GuardianAction::Report {
                    category,
                    domain,
                    stakes,
                    severity,
                } => {
                    let cat = match IncidentCategory::from_code(&category) {
                        Some(c) => c,
                        None => {
                            eprintln!(
                                "Error: unknown category code '{}'. Use 'rsk guardian categories' to see valid codes.",
                                category
                            );
                            std::process::exit(1);
                        }
                    };
                    let stakes_level = match stakes.to_lowercase().as_str() {
                        "low" => StakesLevel::Low,
                        "moderate" => StakesLevel::Moderate,
                        "high" => StakesLevel::High,
                        "critical" => StakesLevel::Critical,
                        _ => StakesLevel::Moderate,
                    };

                    let iair = IAIRBuilder::new()
                        .session_id("cli-generated")
                        .model("Claude", "unknown")
                        .context(
                            ExpertiseLevel::Unknown,
                            stakes_level,
                            CheckabilityLevel::Moderate,
                        )
                        .domain(domain.clone())
                        .incident(cat)
                        .outcome(OutcomeType::NearMiss, *severity)
                        .build_minimal()
                        .unwrap();

                    println!("{}", serde_json::to_string_pretty(&iair).unwrap());
                }
                GuardianAction::Categories => {
                    let categories = [
                        (
                            "CL-CONFAB",
                            "Confabulation",
                            "Confident, detailed, incorrect output",
                        ),
                        (
                            "CL-MOTREASON",
                            "Motivated Reasoning",
                            "Apparent rigor, wrong conclusion",
                        ),
                        (
                            "CL-VULNCODE",
                            "Vulnerable Code",
                            "Security flaw in generated code",
                        ),
                        (
                            "CL-MANIP",
                            "Manipulation",
                            "Persuasion without user awareness",
                        ),
                        (
                            "CL-FALSESYNTH",
                            "False Synthesis",
                            "Imposed coherence on contradictions",
                        ),
                        ("CL-APOPH", "Apophenia", "False pattern detection"),
                        (
                            "CL-BADFOLLOW",
                            "Bad Follow",
                            "Harmful instruction following",
                        ),
                        (
                            "CL-ERRORPROP",
                            "Error Propagation",
                            "Early error compounded",
                        ),
                        (
                            "CL-OVERCONF",
                            "Overconfidence",
                            "Certainty exceeded accuracy",
                        ),
                        (
                            "CL-HALLUCITE",
                            "Hallucinated Citation",
                            "Non-existent source cited",
                        ),
                    ];
                    let output: Vec<_> = categories
                        .iter()
                        .map(|(code, name, desc)| {
                            json!({
                                "code": code,
                                "name": name,
                                "description": desc,
                            })
                        })
                        .collect();
                    println!("{}", serde_json::to_string_pretty(&output).unwrap());
                }
                GuardianAction::Minimize { risk, incidents } => {
                    let level = recommend_minimization(*risk, *incidents);
                    println!(
                        "{}",
                        json!({
                            "risk_score": risk,
                            "incident_count": incidents,
                            "recommended_level": format!("{:?}", level),
                            "description": level.description(),
                            "effect": format!("{:?}", level.signal_effect()),
                        })
                    );
                }
            }
        }
    }
}
