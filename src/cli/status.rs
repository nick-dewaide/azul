use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use crate::manifest::Manifest;

use super::resolve_path;

/// Show the current state of tracked scripts.
#[derive(Debug, Parser)]
pub struct StatusCommand {
    /// Path to the project directory. Defaults to the current directory.
    #[clap(default_value = "")]
    pub project: PathBuf,
}

impl StatusCommand {
    pub fn run(self) -> Result<()> {
        let project_path = resolve_path(&self.project);
        let manifest = Manifest::load(&project_path)?;

        println!("Project: {}", manifest.name);
        println!("Sync direction: {}", manifest.settings.sync_direction);
        println!(
            "File organization: {}",
            manifest.settings.file_organization
        );
        println!("Tracked scripts: {}", manifest.scripts.len());
        println!();

        if manifest.scripts.is_empty() {
            println!("No scripts tracked yet. Connect the Studio plugin to sync.");
            return Ok(());
        }

        let mut scripts: Vec<_> = manifest.scripts.iter().collect();
        scripts.sort_by_key(|(_, entry)| &entry.file);

        for (dm_path, entry) in scripts {
            let file_path = project_path.join(entry.file.replace('/', std::path::MAIN_SEPARATOR_STR));
            let status = if file_path.exists() { "ok" } else { "MISSING" };
            println!(
                "  [{}] {} → {} ({})",
                status, entry.file, dm_path, entry.class_name
            );
        }

        Ok(())
    }
}
