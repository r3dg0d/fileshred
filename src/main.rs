mod cli;
mod commands;
mod config;
mod exit_codes;
mod explain_text;
mod fs;
mod output;
mod wipe;

use clap::{CommandFactory, Parser};
use cli::{Cli, Commands};

fn main() {
    let code = match real_main() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e:#}");
            exit_codes::GENERAL_ERROR
        }
    };
    std::process::exit(code);
}

fn real_main() -> anyhow::Result<i32> {
    let cli = Cli::parse();

    let filter = match (cli.quiet, cli.verbose) {
        (true, _) => "error",
        (false, 0) => "warn",
        (false, 1) => "info",
        (false, _) => "debug",
    };
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(filter));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(std::io::stderr)
        .init();

    ctrlc::set_handler(|| {
        eprintln!("\ninterrupted");
        std::process::exit(exit_codes::INTERRUPTED);
    })?;

    match &cli.command {
        Commands::Inspect { path } => commands::inspect::run(&cli, path.clone()),
        Commands::Explain => commands::explain::run(&cli),
        Commands::Delete {
            path,
            i_understand,
            force,
            unlink_only,
            passes,
        } => commands::delete::run(
            &cli,
            path.clone(),
            *i_understand,
            *force,
            *unlink_only,
            *passes,
        ),
        Commands::WipeFreeSpace {
            path,
            i_understand,
            force,
        } => commands::wipe_free::run(&cli, path.clone(), *i_understand, *force),
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            let name = cmd.get_name().to_string();
            clap_complete::generate(
                clap_complete::Shell::from(*shell),
                &mut cmd,
                name,
                &mut std::io::stdout(),
            );
            Ok(exit_codes::SUCCESS)
        }
    }
}
