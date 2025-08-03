//! # wave - Terminal-based HTTP client
//!
//! A command-line HTTP client designed for developers who prefer working in the terminal.
//! Provides functionality similar to GUI tools like Postman, but with a terminal interface
//! for increased productivity and workflow integration.
//!
//! ## Features
//!
//! - Support for all major HTTP methods (GET, POST, PUT, DELETE, PATCH)
//! - Interactive request/response display with colored output
//! - Save/load collections of requests in YAML format
//! - Header and body parameter support via CLI arguments
//! - Collection parameter override capability
//!
//! ## Usage
//!
//! ```bash
//! # Basic HTTP requests
//! wave get example.com
//! wave post example.com name=john age=30
//! wave put example.com Authorization:Bearer123 status=active
//!
//! # Using saved collections
//! wave myCollection myRequest
//! wave myCollection myRequest Authorization:Bearer456  # Override collection headers
//! ```

use clap::Parser;
use wave::{
    error::WaveError, handle_collection, handle_delete, handle_get, handle_patch, handle_post,
    handle_put, Cli,
};

/// Creates a spinner message for HTTP requests
///
/// Formats the HTTP method, URL, and indicates if parameters are present
/// for display during request execution.
///
/// # Arguments
/// * `method` - The HTTP method name (e.g., "GET", "POST")
/// * `url` - The target URL for the request  
/// * `params` - Additional parameters being sent with the request
///
/// # Returns
/// A formatted string suitable for spinner display
fn spinner_msg(method: &str, url: &str, params: &[String]) -> String {
    format!(
        "{} {}{}",
        method,
        url,
        if params.is_empty() { "" } else { " " },
    )
}

/// Executes the wave application logic
///
/// Parses command-line arguments and dispatches to the appropriate HTTP handler
/// based on the command type. Handles all supported HTTP methods and collection
/// execution.
///
/// # Returns
/// `Ok(())` on successful execution, or a `WaveError` if the request fails
///
/// # Errors
/// Returns errors for:
/// - Invalid command-line arguments (handled by clap)
/// - Network failures during HTTP requests
/// - Invalid URLs or malformed parameters
/// - Missing collection files or requests
async fn run() -> Result<(), WaveError> {
    let cli = Cli::parse();
    use wave::Command;
    match cli.command {
        Command::Get {
            url,
            params,
            verbose,
        } => {
            let msg = spinner_msg("GET", &url, &params);
            handle_get(&url, &params, verbose, &msg).await?;
        }
        Command::Post {
            url,
            params,
            form,
            verbose,
        } => {
            let msg = spinner_msg("POST", &url, &params);
            handle_post(&url, &params, form, verbose, &msg).await?;
        }
        Command::Put {
            url,
            params,
            form,
            verbose,
        } => {
            let msg = spinner_msg("PUT", &url, &params);
            handle_put(&url, &params, form, verbose, &msg).await?;
        }
        Command::Patch {
            url,
            params,
            form,
            verbose,
        } => {
            let msg = spinner_msg("PATCH", &url, &params);
            handle_patch(&url, &params, form, verbose, &msg).await?;
        }
        Command::Delete {
            url,
            params,
            verbose,
        } => {
            let msg = spinner_msg("DELETE", &url, &params);
            handle_delete(&url, &params, verbose, &msg).await?;
        }
        Command::Collection {
            collection,
            request,
            verbose,
            params,
        } => {
            handle_collection(&collection, &request, verbose, &params).await?;
        }
    }
    Ok(())
}

/// Application entry point
///
/// Initializes the tokio async runtime and executes the wave CLI application.
/// Handles error reporting and sets appropriate exit codes.
#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {e}");
        if let Some(suggestion) = e.suggestion() {
            eprintln!("Suggestion: {suggestion}");
        }
        std::process::exit(1);
    }
}
