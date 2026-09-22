use serde::Serialize;

#[derive(Clone, Copy)]
pub struct OutputOpts {
    pub json: bool,
    pub quiet: bool,
    pub verbose: bool,
}

pub fn print_json<T: Serialize>(opts: OutputOpts, value: &T) -> anyhow::Result<()> {
    if !opts.json {
        return Ok(());
    }
    let s = if opts.verbose {
        serde_json::to_string_pretty(value)?
    } else {
        serde_json::to_string(value)?
    };
    println!("{s}");
    Ok(())
}

pub fn human(opts: OutputOpts, msg: &str) {
    if !opts.quiet && !opts.json {
        println!("{msg}");
    }
}

pub fn warn(opts: OutputOpts, msg: &str) {
    if !opts.quiet {
        eprintln!("WARNING: {msg}");
    }
}
