use crate::cli::Cli;
use crate::commands::opts_from;
use crate::explain_text;
use crate::output::print_json;
use anyhow::Result;
use serde::Serialize;

#[derive(Serialize)]
struct ExplainOut {
    text: String,
}

pub fn run(cli: &Cli) -> Result<i32> {
    let opts = opts_from(cli);
    let text = explain_text::explain_markdown().to_string();
    if opts.json {
        print_json(opts, &ExplainOut { text })?;
    } else if !opts.quiet {
        print!("{text}");
    }
    Ok(0)
}
