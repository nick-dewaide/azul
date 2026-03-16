use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::manifest::Manifest;
use crate::script_registry::ScriptClass;

/// Determine ScriptClass from a file name based on its suffix.
pub fn class_from_file(path: &Path) -> ScriptClass {
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();

    if name.ends_with(".server.lua") || name.ends_with(".server.luau") {
        ScriptClass::Script
    } else if name.ends_with(".client.lua") || name.ends_with(".client.luau") {
        ScriptClass::LocalScript
    } else {
        ScriptClass::ModuleScript
    }
}

/// Get the file extension suffix for a given script class.
pub fn suffix_for_class(class: &ScriptClass) -> &'static str {
    match class {
        ScriptClass::Script => ".server.lua",
        ScriptClass::LocalScript => ".client.lua",
        ScriptClass::ModuleScript => ".lua",
    }
}

/// Extract the script name from a file name by stripping the suffix.
pub fn script_name_from_file(path: &Path) -> String {
    let name = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // Strip .server or .client from the stem if present
    if let Some(stripped) = name.strip_suffix(".server") {
        stripped.to_string()
    } else if let Some(stripped) = name.strip_suffix(".client") {
        stripped.to_string()
    } else {
        name
    }
}

/// Derive the DataModel path from a file path and the manifest's service directory mappings.
///
/// For example, with service_dirs { "ServerScriptService": "server" }:
///   `server/GameManager.server.lua` → `ServerScriptService.GameManager`
///   `shared/Modules/Combat.lua`     → `ReplicatedStorage.Modules.Combat`
pub fn dm_path_from_file(file_path: &Path, manifest: &Manifest, project_root: &Path) -> Option<String> {
    let relative = file_path
        .strip_prefix(project_root)
        .ok()?;

    let relative_str = relative.to_string_lossy().replace('\\', "/");

    // Find which service directory this file belongs to
    for (service, dir) in &manifest.settings.service_dirs {
        let prefix = format!("{}/", dir);
        if relative_str.starts_with(&prefix) {
            let remainder = &relative_str[prefix.len()..];
            // Convert path components to dot-separated DataModel path
            let script_name = script_name_from_file(Path::new(remainder));
            let parent_path = Path::new(remainder).parent();

            let dm_path = if let Some(parent) = parent_path {
                let parent_str = parent.to_string_lossy().replace('/', ".").replace('\\', ".");
                if parent_str.is_empty() {
                    format!("{}.{}", service, script_name)
                } else {
                    format!("{}.{}.{}", service, parent_str, script_name)
                }
            } else {
                format!("{}.{}", service, script_name)
            };

            return Some(dm_path);
        }
    }

    None
}

/// Derive the filesystem path from a DataModel path and manifest settings.
///
/// For example: `ServerScriptService.GameManager` with Script class
///   → `server/GameManager.server.lua`
pub fn file_path_from_dm(
    dm_path: &str,
    class: &ScriptClass,
    manifest: &Manifest,
    project_root: &Path,
) -> Option<PathBuf> {
    let (dir, rest) = find_service_dir(dm_path, &manifest.settings.service_dirs)?;

    let suffix = suffix_for_class(class);

    if manifest.settings.file_organization == "flat" {
        // Flat: all in one scripts/ directory
        let file_name = format!("{}{}", dm_path.replace('.', "_"), suffix);
        return Some(project_root.join("scripts").join(file_name));
    }

    // Mirrored: filesystem mirrors DataModel hierarchy
    let rest_parts: Vec<&str> = rest.split('.').filter(|s| !s.is_empty()).collect();

    if rest_parts.is_empty() {
        return None;
    }

    let script_name = rest_parts.last().unwrap();
    let parent_parts = &rest_parts[..rest_parts.len() - 1];

    let mut path = project_root.join(&dir);
    for part in parent_parts {
        path = path.join(part);
    }
    path = path.join(format!("{}{}", script_name, suffix));

    Some(path)
}

/// Find the service directory and remaining DataModel path for a given full path.
fn find_service_dir(
    dm_path: &str,
    service_dirs: &HashMap<String, String>,
) -> Option<(String, String)> {
    // Try longest service name first (e.g., "StarterPlayer.StarterPlayerScripts" before "StarterPlayer")
    let mut services: Vec<(&String, &String)> = service_dirs.iter().collect();
    services.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

    for (service, dir) in services {
        let prefix = format!("{}.", service);
        if dm_path.starts_with(&prefix) {
            let rest = dm_path[prefix.len()..].to_string();
            return Some((dir.clone(), rest));
        }
        if dm_path == service.as_str() {
            return Some((dir.clone(), String::new()));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_class_from_file() {
        assert_eq!(
            class_from_file(Path::new("GameManager.server.lua")),
            ScriptClass::Script
        );
        assert_eq!(
            class_from_file(Path::new("HUD.client.lua")),
            ScriptClass::LocalScript
        );
        assert_eq!(
            class_from_file(Path::new("Combat.lua")),
            ScriptClass::ModuleScript
        );
    }

    #[test]
    fn test_script_name_from_file() {
        assert_eq!(
            script_name_from_file(Path::new("GameManager.server.lua")),
            "GameManager"
        );
        assert_eq!(
            script_name_from_file(Path::new("HUD.client.lua")),
            "HUD"
        );
        assert_eq!(
            script_name_from_file(Path::new("Combat.lua")),
            "Combat"
        );
    }
}
