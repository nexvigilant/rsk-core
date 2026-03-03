//! RSK (Rust Skill Kernel) - CLI entry point
//!
//! This binary provides the command-line interface for RSK functionality.
//! Handler implementations are in the `cli` module.

use clap::{Parser, Subcommand};

mod cli;

use cli::{
    AntiPatternAction, ChainAction, CompressAction, ExecAction, GenerateAction, GraphAction,
    GuardianAction, HooksAction, MicrogramAction, RouteAction, Sha256Action, SkillsAction,
    StateAction, TaxonomyAction, TelemetryAction, TextAction, TovAction, YamlAction,
};

#[cfg(feature = "forge")]
use cli::ForgeAction;

// ═══════════════════════════════════════════════════════════════════════════════
// CLI DEFINITION
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Parser)]
#[command(name = "rsk")]
#[command(
    about = "RSK (Rust Skill Kernel) - Skill validation, graph operations, and text processing"
)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate a skill or directory of skills against Diamond v2 compliance
    Verify {
        /// Path to SKILL.md file, skill directory, or parent directory
        #[arg(default_value = ".")]
        path: String,
        /// Minimum score threshold for passing (0-100)
        #[arg(short, long, default_value = "85.0")]
        threshold: f64,
        /// Output format: json, summary, report, minimal
        #[arg(short, long, default_value = "json")]
        format: String,
        /// Export JSON Schema for SMST v2
        #[arg(long)]
        export_jsonschema: bool,
        /// Show verbose output with full SMST details
        #[arg(short, long)]
        verbose: bool,
    },
    /// Alias for verify (backward compatibility)
    #[command(hide = true)]
    Validate {
        #[arg(default_value = ".")]
        path: String,
        #[arg(short, long, default_value = "85.0")]
        threshold: f64,
        #[arg(short, long, default_value = "json")]
        format: String,
        #[arg(long)]
        export_jsonschema: bool,
        #[arg(short, long)]
        verbose: bool,
    },
    /// Build skill artifacts (manifest.json, etc.)
    Build {
        /// Path to skill directory or parent directory
        #[arg(default_value = ".")]
        path: String,
        /// Dry run - show what would be generated without writing
        #[arg(long)]
        dry_run: bool,
    },
    /// Code generation from SMST
    Generate {
        #[command(subcommand)]
        action: GenerateAction,
    },
    /// Graph/DAG operations (topological sort, shortest path, levels)
    Graph {
        #[command(subcommand)]
        action: GraphAction,
    },
    /// Text processing and SKILL.md validation
    Text {
        #[command(subcommand)]
        action: TextAction,
    },
    /// Calculate Levenshtein edit distance
    Levenshtein {
        /// Source string
        source: String,
        /// Target string
        target: String,
    },
    /// Fuzzy search with Levenshtein
    Fuzzy {
        /// Query to search for
        query: String,
        /// Comma-separated list of candidates
        candidates: String,
        /// Max results
        #[arg(short, long, default_value = "5")]
        limit: usize,
    },
    /// SHA-256 hashing operations
    Sha256 {
        #[command(subcommand)]
        action: Sha256Action,
    },
    /// Print version
    Version,
    /// Variance calculation
    Variance {
        /// Actual value
        actual: f64,
        /// Target value
        target: f64,
    },
    /// YAML/TOML processing
    Yaml {
        #[command(subcommand)]
        action: YamlAction,
    },
    /// Taxonomy queries
    Taxonomy {
        #[command(subcommand)]
        action: TaxonomyAction,
    },
    /// Telemetry configuration
    Telemetry {
        #[command(subcommand)]
        action: TelemetryAction,
    },
    /// Compression utilities
    Compress {
        #[command(subcommand)]
        action: CompressAction,
    },
    /// Data pipeline specification (requires forge feature)
    #[cfg(feature = "forge")]
    Forge {
        #[command(subcommand)]
        action: ForgeAction,
    },
    /// Execution engine operations
    Exec {
        #[command(subcommand)]
        action: ExecAction,
    },
    /// Start the state server (Unix socket IPC)
    Server {
        /// Socket path
        #[arg(short, long)]
        socket: Option<String>,
    },
    /// Skill routing
    Route {
        #[command(subcommand)]
        action: RouteAction,
    },
    /// State/checkpoint management
    State {
        #[command(subcommand)]
        action: StateAction,
    },
    /// Skills registry operations
    Skills {
        #[command(subcommand)]
        action: SkillsAction,
    },
    /// Skill chain validation and execution
    Chain {
        #[command(subcommand)]
        action: ChainAction,
    },
    /// Anti-pattern detection and registration
    #[command(name = "anti-pattern")]
    AntiPattern {
        #[command(subcommand)]
        action: AntiPatternAction,
    },
    /// Microgram: atomic self-testing programs
    #[command(name = "mcg")]
    Microgram {
        #[command(subcommand)]
        action: MicrogramAction,
    },
    /// Evolve a skill from logic.yaml to Rust intrinsic
    Evolve {
        /// Skill name to evolve
        name: String,
        /// Path to registry JSON
        #[arg(short, long)]
        registry: Option<String>,
    },
    /// File organization hooks
    Hooks {
        #[command(subcommand)]
        action: HooksAction,
    },
    /// Theory of Vigilance (ToV) framework
    Tov {
        #[command(subcommand)]
        action: TovAction,
    },
    /// Guardian-AV algorithmovigilance
    Guardian {
        #[command(subcommand)]
        action: GuardianAction,
    },
}

// ═══════════════════════════════════════════════════════════════════════════════
// MAIN DISPATCH
// ═══════════════════════════════════════════════════════════════════════════════

fn main() {
    let cli = Cli::parse();

    match cli.command {
        // Simple commands
        Commands::Version => cli::handlers::simple::handle_version(),
        Commands::Variance { actual, target } => {
            cli::handlers::simple::handle_variance(actual, target)
        }
        Commands::Levenshtein { source, target } => {
            cli::handlers::simple::handle_levenshtein(&source, &target)
        }
        Commands::Fuzzy {
            query,
            candidates,
            limit,
        } => cli::handlers::simple::handle_fuzzy(&query, &candidates, limit),
        Commands::Sha256 { action } => cli::handlers::simple::handle_sha256(&action),

        // Verify/Validate commands
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
        } => cli::handlers::verify::handle_verify(
            &path,
            threshold,
            &format,
            export_jsonschema,
            verbose,
        ),
        Commands::Build { path, dry_run } => cli::handlers::verify::handle_build(&path, dry_run),

        // Domain-specific handlers
        Commands::Graph { action } => cli::handlers::graph::handle_graph(&action),
        Commands::Text { action } => cli::handlers::text::handle_text(&action),
        Commands::Yaml { action } => cli::handlers::yaml::handle_yaml(&action),
        Commands::Taxonomy { action } => cli::handlers::taxonomy::handle_taxonomy(&action),
        Commands::Telemetry { action } => cli::handlers::telemetry::handle_telemetry(&action),
        Commands::Compress { action } => cli::handlers::compress::handle_compress(&action),
        Commands::Generate { action } => cli::handlers::generate::handle_generate(&action),
        Commands::Exec { action } => cli::handlers::exec::handle_exec(&action),
        Commands::Route { action } => cli::handlers::route::handle_route(&action),
        Commands::State { action } => cli::handlers::state::handle_state(&action),
        Commands::Skills { action } => cli::handlers::skills::handle_skills(&action),
        Commands::Chain { action } => cli::handlers::skills::handle_chain(&action),
        Commands::AntiPattern { action } => {
            cli::handlers::anti_pattern::handle_anti_pattern(&action)
        }
        Commands::Microgram { action } => cli::handlers::microgram::handle_microgram(&action),
        Commands::Evolve { name, registry } => {
            cli::handlers::skills::handle_evolve(&name, &registry)
        }
        Commands::Hooks { action } => cli::handlers::hooks::handle_hooks(&action),
        Commands::Tov { action } => cli::handlers::tov::handle_tov(&action),
        Commands::Guardian { action } => cli::handlers::guardian::handle_guardian(&action),

        // Server command
        Commands::Server { socket } => {
            let path = socket.unwrap_or_else(|| "/tmp/rsk-state.sock".to_string());
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let server = rsk::StateServer::new(&path);
                server.run().await.unwrap();
            });
        }

        // Forge (feature-gated)
        #[cfg(feature = "forge")]
        Commands::Forge { action } => cli::handlers::forge::handle_forge(&action),
    }
}
