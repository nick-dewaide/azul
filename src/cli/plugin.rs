use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use clap::Parser;

use crate::plugin_builder::build_plugin;

static PLUGIN_FILE_NAME: &str = "ScriptSync.rbxm";

/// Manage the ScriptSync Roblox Studio plugin.
#[derive(Debug, Parser)]
pub struct PluginCommand {
    #[clap(subcommand)]
    subcommand: PluginSubcommand,
}

/// Manages the ScriptSync Roblox Studio plugin.
#[derive(Debug, Parser)]
pub enum PluginSubcommand {
    /// Install the plugin in Roblox Studio's plugins folder.
    Install,
    /// Remove the plugin if it is installed.
    Uninstall,
    /// Build the plugin .rbxm to a specific path (for CI/distribution).
    Build {
        /// Output path for the .rbxm file.
        #[clap(long, default_value = "ScriptSync.rbxm")]
        output: PathBuf,
    },
}

impl PluginCommand {
    pub fn run(self) -> Result<()> {
        self.subcommand.run()
    }
}

impl PluginSubcommand {
    pub fn run(self) -> Result<()> {
        match self {
            PluginSubcommand::Install => install_plugin(),
            PluginSubcommand::Uninstall => uninstall_plugin(),
            PluginSubcommand::Build { output } => build_plugin_to(&output),
        }
    }
}

fn get_plugins_dir() -> Result<std::path::PathBuf> {
    // Try roblox_install first
    if let Ok(studio) = roblox_install::RobloxStudio::locate() {
        return Ok(studio.plugins_path().to_path_buf());
    }

    // Fallback to manual detection
    #[cfg(target_os = "windows")]
    {
        if let Some(local_app_data) = dirs::data_local_dir() {
            return Ok(local_app_data.join("Roblox").join("Plugins"));
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            return Ok(home.join("Library/Application Support/Roblox/Plugins"));
        }
    }

    Err(anyhow!("Could not find Roblox Studio plugins directory"))
}

fn find_plugin_src() -> Result<PathBuf> {
    // Look for plugin/src relative to the executable, then relative to cwd
    let exe_dir = env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()));

    let candidates = [
        exe_dir.as_ref().map(|d| d.join("../../plugin/src")),
        exe_dir.as_ref().map(|d| d.join("../plugin/src")),
        Some(PathBuf::from("plugin/src")),
    ];

    for candidate in candidates.into_iter().flatten() {
        let canonical = candidate.canonicalize();
        if let Ok(path) = canonical {
            if path.join("init.server.lua").exists() {
                return Ok(path);
            }
        }
    }

    Err(anyhow!(
        "Could not find plugin source directory. Run this command from the ScriptSync repo root."
    ))
}

fn install_plugin() -> Result<()> {
    let plugins_dir = get_plugins_dir()?;

    if !plugins_dir.exists() {
        log::debug!("Creating Roblox Studio plugins folder");
        fs::create_dir_all(&plugins_dir)?;
    }

    let plugin_path = plugins_dir.join(PLUGIN_FILE_NAME);

    let plugin_src = find_plugin_src()?;
    println!("Building plugin from {}...", plugin_src.display());
    build_plugin(&plugin_src, &plugin_path)?;

    println!("Plugin installed to {}", plugin_path.display());
    println!("Restart Roblox Studio to load the plugin.");

    Ok(())
}

fn build_plugin_to(output: &Path) -> Result<()> {
    let plugin_src = find_plugin_src()?;
    println!("Building plugin from {}...", plugin_src.display());
    build_plugin(&plugin_src, output)?;
    println!("Plugin written to {}", output.display());
    Ok(())
}

fn uninstall_plugin() -> Result<()> {
    let plugins_dir = get_plugins_dir()?;
    let plugin_path = plugins_dir.join(PLUGIN_FILE_NAME);

    if plugin_path.exists() {
        log::debug!("Removing plugin from {}", plugin_path.display());
        fs::remove_file(&plugin_path)?;
        println!("Plugin removed.");
    } else {
        println!("Plugin not installed at {}", plugin_path.display());
    }

    Ok(())
}
