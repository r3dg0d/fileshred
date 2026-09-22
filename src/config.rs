use anyhow::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_passes")]
    pub overwrite_passes: u32,
    #[serde(default)]
    pub prefer_shred: bool,
}

fn default_passes() -> u32 {
    3
}

impl Default for Config {
    fn default() -> Self {
        Self {
            overwrite_passes: 3,
            prefer_shred: true,
        }
    }
}

impl Config {
    pub fn load(explicit: Option<&Path>) -> Result<(Self, Option<PathBuf>)> {
        if let Some(p) = explicit {
            let text = fs::read_to_string(p)?;
            return Ok((serde_json::from_str(&text)?, Some(p.to_path_buf())));
        }
        if let Some(dirs) = ProjectDirs::from("dev", "r3dg0d", "fileshred") {
            let path = dirs.config_dir().join("config.json");
            if path.exists() {
                let text = fs::read_to_string(&path)?;
                return Ok((serde_json::from_str(&text)?, Some(path)));
            }
        }
        Ok((Config::default(), None))
    }
}
