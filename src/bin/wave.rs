use clap::Parser;
use wave::{handle_delete, handle_get, handle_patch, handle_post, handle_put, Cli, Commands};

fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Get { url, params } => {
            let spinner_msg = format!(
                "get {}{}{}",
                url,
                if params.is_empty() { "" } else { " " },
                params.join(" ")
            );
            handle_get(url, params, cli.verbose, &spinner_msg);
        }
        Commands::Post { url, params, form } => {
            let spinner_msg = format!(
                "post {}{}{}{}",
                url,
                if params.is_empty() { "" } else { " " },
                params.join(" "),
                if *form { " --form" } else { "" }
            );
            handle_post(url, params, *form, cli.verbose, &spinner_msg);
        }
        Commands::Put { url, params, form } => {
            let spinner_msg = format!(
                "put {}{}{}{}",
                url,
                if params.is_empty() { "" } else { " " },
                params.join(" "),
                if *form { " --form" } else { "" }
            );
            handle_put(url, params, *form, cli.verbose, &spinner_msg);
        }
        Commands::Patch { url, params, form } => {
            let spinner_msg = format!(
                "patch {}{}{}{}",
                url,
                if params.is_empty() { "" } else { " " },
                params.join(" "),
                if *form { " --form" } else { "" }
            );
            handle_patch(url, params, *form, cli.verbose, &spinner_msg);
        }
        Commands::Delete { url, params } => {
            let spinner_msg = format!(
                "delete {}{}{}",
                url,
                if params.is_empty() { "" } else { " " },
                params.join(" ")
            );
            handle_delete(url, params, cli.verbose, &spinner_msg);
        }
    }
}
