//! Telemetry configuration handler.

use crate::cli::actions::TelemetryAction;
use rsk::{TelemetryConfig, get_telemetry_status};
use serde_json::json;

/// Handle telemetry subcommands.
pub fn handle_telemetry(action: &TelemetryAction) {
    match action {
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
    }
}
