use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use crate::context_file::generate_context_file;
use crate::manifest::{default_watch_services, Manifest};

use super::resolve_path;

/// Initialize a new ScriptSync project in the current directory.
#[derive(Debug, Parser)]
pub struct InitCommand {
    /// Path to the directory to initialize. Defaults to the current directory.
    #[clap(default_value = "")]
    pub path: PathBuf,

    /// File organization style: "mirrored" or "flat".
    #[clap(long, default_value = "mirrored")]
    pub organization: String,
}

impl InitCommand {
    pub fn run(self) -> Result<()> {
        let base_path = resolve_path(&self.path);
        fs_err::create_dir_all(&base_path)?;

        let canonical = fs_err::canonicalize(&base_path)?;
        let project_name = canonical
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("new-project")
            .to_string();

        if Manifest::exists(&base_path) {
            log::warn!("scriptsync.json already exists in this directory. Skipping.");
            return Ok(());
        }

        println!("Initializing ScriptSync project '{}'", project_name);

        let manifest = Manifest::new(
            project_name,
            self.organization,
            default_watch_services(),
        );

        manifest.save(&base_path)?;
        println!("Created scriptsync.json");

        generate_context_file(&manifest, &base_path)?;
        println!("Created CONTEXT.md");

        let gitignore_path = base_path.join(".gitignore");
        if !gitignore_path.exists() {
            fs_err::write(&gitignore_path, "*.deleted\n*.studio\n")?;
            println!("Created .gitignore");
        }

        for dir in manifest.settings.service_dirs.values() {
            let dir_path = base_path.join(dir);
            if !dir_path.exists() {
                fs_err::create_dir_all(&dir_path)?;
                fs_err::write(dir_path.join(".gitkeep"), "")?;
            }
        }
        println!("Created service directories");

        println!("\nProject initialized! Next steps:");
        println!("  1. Run `scriptsync serve` to start the sync server");
        println!("  2. Open Roblox Studio and connect the ScriptSync plugin");
        println!("  3. Scripts will be synced to this directory automatically");

        Ok(())
    }
}
