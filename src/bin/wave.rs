use clap::Parser;
use wave::{
    error::WaveError, handle_collection, handle_delete, handle_get, handle_patch, handle_post,
    handle_put, Cli,
};

fn spinner_msg(method: &str, url: &str, params: &[String]) -> String {
    format!(
        "{} {}{}",
        method,
        url,
        if params.is_empty() { "" } else { " " },
    )
}

fn run() -> Result<(), WaveError> {
    let cli = Cli::parse();
    use wave::Command;
    match cli.command {
        Command::Get {
            url,
            params,
            verbose,
        } => {
            let msg = spinner_msg("GET", &url, &params);
            handle_get(&url, &params, verbose, &msg)?;
        }
        Command::Post {
            url,
            params,
            form,
            verbose,
        } => {
            let msg = spinner_msg("POST", &url, &params);
            handle_post(&url, &params, form, verbose, &msg)?;
        }
        Command::Put {
            url,
            params,
            form,
            verbose,
        } => {
            let msg = spinner_msg("PUT", &url, &params);
            handle_put(&url, &params, form, verbose, &msg)?;
        }
        Command::Patch {
            url,
            params,
            form,
            verbose,
        } => {
            let msg = spinner_msg("PATCH", &url, &params);
            handle_patch(&url, &params, form, verbose, &msg)?;
        }
        Command::Delete {
            url,
            params,
            verbose,
        } => {
            let msg = spinner_msg("DELETE", &url, &params);
            handle_delete(&url, &params, verbose, &msg)?;
        }
        Command::Collection {
            collection,
            request,
            verbose,
        } => {
            handle_collection(&collection, &request, verbose)?;
        }
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e}");
        if let Some(suggestion) = e.suggestion() {
            eprintln!("Suggestion: {suggestion}");
        }
        std::process::exit(1);
    }
}
