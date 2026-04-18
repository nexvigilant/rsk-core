//! CLI handler for session tracking operations.

use crate::cli::actions::SessionAction;
use rsk::modules::session_tracker;
use serde_json::json;
use std::path::Path;

/// Handle the session subcommands.
pub fn handle_session(action: &SessionAction) {
    match action {
        SessionAction::Load { path } => {
            let state_path = path
                .as_deref()
                .map(Path::new)
                .map(std::borrow::Cow::Borrowed)
                .unwrap_or_else(|| {
                    std::borrow::Cow::Owned(session_tracker::default_state_path("default"))
                });
            match session_tracker::load_state(&state_path) {
                Ok(state) => println!("{}", json!(state)),
                Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
            }
        }
        SessionAction::Track {
            skill,
            context,
            path,
        } => {
            let state_path = path
                .as_deref()
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| session_tracker::default_state_path(skill));
            match session_tracker::track_execution(&state_path, skill, context.as_deref()) {
                Ok(state) => println!("{}", json!(state)),
                Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
            }
        }
        SessionAction::Complete { duration_ms, path } => {
            let state_path = path
                .as_deref()
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| session_tracker::default_state_path("default"));
            match session_tracker::track_completion(&state_path, *duration_ms) {
                Ok(state) => println!("{}", json!(state)),
                Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
            }
        }
        SessionAction::Fail { error, path } => {
            let state_path = path
                .as_deref()
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| session_tracker::default_state_path("default"));
            match session_tracker::track_failure(&state_path, error.as_deref()) {
                Ok(state) => println!("{}", json!(state)),
                Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
            }
        }
        SessionAction::Log {
            skill,
            message,
            path,
        } => {
            let log_path = path
                .as_deref()
                .map(std::path::PathBuf::from)
                .unwrap_or_else(session_tracker::default_log_path);
            match session_tracker::append_log(&log_path, skill, message) {
                Ok(()) => println!("{}", json!({"status": "success"})),
                Err(e) => println!("{}", json!({"status": "error", "message": e.to_string()})),
            }
        }
    }
}
