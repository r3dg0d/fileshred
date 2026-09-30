use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "fileshred",
    author = "r3dg0d",
    version,
    about = "Honest secure-delete — best-effort overwrite with clear modern-storage limitations"
)]
pub struct Cli {
    #[arg(long, global = true)]
    pub json: bool,

    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    #[arg(short, long, global = true)]
    pub quiet: bool,

    #[arg(long, global = true, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Show what would happen without modifying data
    #[arg(long, global = true)]
    pub dry_run: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Inspect path filesystem/storage and whether overwrite is meaningful
    Inspect { path: PathBuf },
    /// Print educational explanation of secure-delete limitations
    Explain,
    /// Best-effort delete (overwrite when meaningful; otherwise refuse or require ack)
    Delete {
        path: PathBuf,
        /// Acknowledge that wipe is not guaranteed on this storage
        #[arg(long = "i-understand")]
        i_understand: bool,
        /// Force best-effort overwrite+unlink even when not meaningful (implies understanding)
        #[arg(long)]
        force: bool,
        /// Only unlink; never overwrite
        #[arg(long)]
        unlink_only: bool,
        /// Overwrite passes (default from config or 3)
        #[arg(long, short = 'n')]
        passes: Option<u32>,
    },
    /// Optional free-space wipe (careful; often ineffective on SSD)
    WipeFreeSpace {
        /// Mountpoint or directory on the target filesystem
        path: PathBuf,
        #[arg(long = "i-understand")]
        i_understand: bool,
        #[arg(long)]
        force: bool,
    },
    Completions {
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Shell {
    Bash,
    Elvish,
    Fish,
    Powershell,
    Zsh,
}

impl From<Shell> for clap_complete::Shell {
    fn from(s: Shell) -> Self {
        match s {
            Shell::Bash => Self::Bash,
            Shell::Elvish => Self::Elvish,
            Shell::Fish => Self::Fish,
            Shell::Powershell => Self::PowerShell,
            Shell::Zsh => Self::Zsh,
        }
    }
}
