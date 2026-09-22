pub mod delete;
pub mod explain;
pub mod inspect;
pub mod wipe_free;

use crate::cli::Cli;
use crate::output::OutputOpts;

pub fn opts_from(cli: &Cli) -> OutputOpts {
    OutputOpts {
        json: cli.json,
        quiet: cli.quiet,
        verbose: cli.verbose > 0,
    }
}
