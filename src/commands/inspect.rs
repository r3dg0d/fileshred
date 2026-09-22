use crate::cli::Cli;
use crate::commands::opts_from;
use crate::fs::{self, StorageClass};
use crate::output::print_json;
use anyhow::Result;
use std::path::PathBuf;

pub fn run(cli: &Cli, path: PathBuf) -> Result<i32> {
    let opts = opts_from(cli);
    let insp = fs::inspect_path(&path)?;

    if opts.json {
        print_json(opts, &insp)?;
    } else if !opts.quiet {
        println!("path:        {}", insp.path.display());
        println!("exists:      {}", insp.exists);
        println!("file/dir:    file={} dir={}", insp.is_file, insp.is_dir);
        if let Some(sz) = insp.size_bytes {
            println!("size:        {sz} bytes");
        }
        println!("fstype:      {:?}", insp.filesystem_type);
        println!("mount:       {:?}", insp.mount_point);
        println!("device:      {:?}", insp.source_device);
        println!("rotational:  {:?}", insp.rotational);
        println!("class:       {:?}", insp.storage_class);
        println!("advice:      {:?}", insp.advice);
        println!();
        println!("{}", storage_explain(insp.storage_class));
        for n in &insp.notes {
            println!("note: {n}");
        }
        println!();
        match insp.advice {
            fs::DeleteAdvice::OverwriteMeaningful => {
                println!("overwrite: potentially meaningful (still not a guarantee)");
            }
            fs::DeleteAdvice::RequireAcknowledgement => {
                println!("overwrite: NOT meaningful as a guarantee — delete requires --i-understand/--force");
            }
            fs::DeleteAdvice::OverwriteNotMeaningful => {
                println!("overwrite: not meaningful");
            }
            fs::DeleteAdvice::NotApplicable => {
                println!("overwrite: not applicable (need a regular file)");
            }
        }
    }
    Ok(0)
}

fn storage_explain(c: StorageClass) -> String {
    c.explain().to_string()
}
