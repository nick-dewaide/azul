use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::Parser;

use crate::pull::extract_scripts_from_file;

use super::resolve_path;

/// Pull scripts from a .rbxl or .rbxlx file to disk. This is the offline
/// entry point for existing projects — save your place as .rbxl, run
/// `scriptsync pull`, commit to Git.
#[derive(Debug, Parser)]
pub struct PullCommand {
    /// Path to the .rbxl or .rbxlx file.
    pub input: PathBuf,

    /// Path to the project directory. Defaults to the current directory.
    #[clap(long, default_value = "")]
    pub output: PathBuf,
}

impl PullCommand {
    pub fn run(self) -> Result<()> {
        let input_path = resolve_path(&self.input);
        let output_path = resolve_path(&self.output);

        if !input_path.exists() {
            bail!("Input file does not exist: {}", input_path.display());
        }

        extract_scripts_from_file(&input_path, &output_path)?;

        Ok(())
    }
}
