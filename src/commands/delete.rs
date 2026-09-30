use crate::cli::Cli;
use crate::commands::opts_from;
use crate::config::Config;
use crate::exit_codes;
use crate::fs::{self, DeleteAdvice, StorageClass};
use crate::output::{self, print_json};
use crate::wipe;
use anyhow::Result;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize)]
struct DeleteOut {
    path: PathBuf,
    inspection: fs::PathInspection,
    result: Option<WipeResultSer>,
    refused: bool,
    message: String,
    guaranteed_physical_erasure: bool,
}

#[derive(Serialize)]
struct WipeResultSer {
    method: String,
    passes: u32,
    unlinked: bool,
    notes: Vec<String>,
}

pub fn run(
    cli: &Cli,
    path: PathBuf,
    i_understand: bool,
    force: bool,
    unlink_only: bool,
    passes: Option<u32>,
) -> Result<i32> {
    let opts = opts_from(cli);
    let (cfg, _) = Config::load(cli.config.as_deref())?;
    let passes = passes.unwrap_or(cfg.overwrite_passes);
    let ack = i_understand || force;

    let insp = fs::inspect_path(&path)?;

    if !insp.exists {
        let msg = format!("path does not exist: {}", path.display());
        if opts.json {
            print_json(
                opts,
                &DeleteOut {
                    path,
                    inspection: insp,
                    result: None,
                    refused: true,
                    message: msg,
                    guaranteed_physical_erasure: false,
                },
            )?;
        } else {
            eprintln!("{msg}");
        }
        return Ok(exit_codes::NOT_FOUND);
    }

    if !insp.is_file {
        let msg = "refusing: only regular files are supported".into();
        emit(
            opts,
            &DeleteOut {
                path,
                inspection: insp,
                result: None,
                refused: true,
                message: msg,
                guaranteed_physical_erasure: false,
            },
        )?;
        return Ok(exit_codes::REFUSED);
    }

    // Always assert we never claim physical guarantee
    let guaranteed = false;

    match insp.advice {
        DeleteAdvice::OverwriteMeaningful if !unlink_only => {
            output::human(
                opts,
                "Storage class suggests overwrite may be meaningful (HDD/non-CoW). Still NOT a physical guarantee.",
            );
            let result = wipe::overwrite_and_unlink(&path, passes, cfg.prefer_shred, cli.dry_run)?;
            emit(
                opts,
                &DeleteOut {
                    path,
                    inspection: insp,
                    result: Some(WipeResultSer {
                        method: result.method,
                        passes: result.passes,
                        unlinked: result.unlinked,
                        notes: result.notes,
                    }),
                    refused: false,
                    message: if cli.dry_run {
                        "dry-run complete".into()
                    } else {
                        "best-effort overwrite+unlink completed; physical erasure NOT guaranteed"
                            .into()
                    },
                    guaranteed_physical_erasure: guaranteed,
                },
            )?;
            Ok(0)
        }
        DeleteAdvice::RequireAcknowledgement
        | DeleteAdvice::OverwriteNotMeaningful
        | DeleteAdvice::OverwriteMeaningful => {
            // unlink_only path or non-meaningful storage
            if !ack && !unlink_only {
                let msg = format!(
                    "refusing guaranteed wipe on {:?} storage. Re-run with --i-understand or --force \
                     for best-effort unlink/overwrite, or --unlink-only. See: fileshred explain",
                    insp.storage_class
                );
                output::warn(opts, &msg);
                emit(
                    opts,
                    &DeleteOut {
                        path,
                        inspection: insp,
                        result: None,
                        refused: true,
                        message: msg,
                        guaranteed_physical_erasure: false,
                    },
                )?;
                return Ok(exit_codes::REFUSED);
            }

            output::warn(
                opts,
                &format!(
                    "Proceeding WITHOUT physical erasure guarantee ({})",
                    insp.storage_class.explain()
                ),
            );

            let result = if unlink_only {
                wipe::unlink_only(&path, cli.dry_run)?
            } else if matches!(insp.storage_class, StorageClass::HddTraditional) {
                wipe::overwrite_and_unlink(&path, passes, cfg.prefer_shred, cli.dry_run)?
            } else if force {
                // User forced: try overwrite anyway then unlink, with loud notes
                match wipe::overwrite_and_unlink(&path, passes, cfg.prefer_shred, cli.dry_run) {
                    Ok(mut r) => {
                        r.notes.push(
                            "Forced overwrite on storage where it is likely ineffective.".into(),
                        );
                        r
                    }
                    Err(e) => {
                        output::warn(
                            opts,
                            &format!("overwrite failed ({e}); falling back to unlink"),
                        );
                        wipe::unlink_only(&path, cli.dry_run)?
                    }
                }
            } else {
                // --i-understand without --force: document + unlink
                wipe::unlink_only(&path, cli.dry_run)?
            };

            emit(
                opts,
                &DeleteOut {
                    path,
                    inspection: insp,
                    result: Some(WipeResultSer {
                        method: result.method,
                        passes: result.passes,
                        unlinked: result.unlinked,
                        notes: result.notes,
                    }),
                    refused: false,
                    message: "completed best-effort operation; physical erasure NOT guaranteed"
                        .into(),
                    guaranteed_physical_erasure: false,
                },
            )?;
            Ok(0)
        }
        DeleteAdvice::NotApplicable => {
            let msg = "not applicable".into();
            emit(
                opts,
                &DeleteOut {
                    path,
                    inspection: insp,
                    result: None,
                    refused: true,
                    message: msg,
                    guaranteed_physical_erasure: false,
                },
            )?;
            Ok(exit_codes::REFUSED)
        }
    }
}

fn emit(opts: output::OutputOpts, out: &DeleteOut) -> Result<()> {
    if opts.json {
        print_json(opts, out)?;
    } else if !opts.quiet {
        println!("{}", out.message);
        println!(
            "guaranteed_physical_erasure: {}",
            out.guaranteed_physical_erasure
        );
        if let Some(ref r) = out.result {
            println!("method: {}", r.method);
            println!("unlinked: {}", r.unlinked);
            for n in &r.notes {
                println!("note: {n}");
            }
        }
    }
    Ok(())
}
