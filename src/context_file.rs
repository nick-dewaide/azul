use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;

use crate::manifest::Manifest;

pub fn generate_context_file(manifest: &Manifest, project_root: &Path) -> Result<()> {
    let mut out = String::new();

    out.push_str(&format!("# Project: {}\n\n", manifest.name));
    out.push_str(
        "This is a Roblox project. Scripts are synced bidirectionally between this\n\
         workspace and Roblox Studio via ScriptSync. Only script source code lives\n\
         here — all other game assets (models, UI, maps, etc.) live exclusively in\n\
         Roblox Studio.\n\n",
    );

    // Directory mapping table
    out.push_str("## Directory → Roblox Service Mapping\n\n");
    out.push_str("| Directory | Roblox Service |\n");
    out.push_str("|-----------|---------------|\n");

    let mut sorted_dirs: Vec<_> = manifest.settings.service_dirs.iter().collect();
    sorted_dirs.sort_by_key(|(_, dir)| dir.to_string());
    for (service, dir) in &sorted_dirs {
        out.push_str(&format!("| {dir}/ | {service} |\n"));
    }

    // File naming
    out.push_str("\n## File Naming\n\n");
    out.push_str("| Suffix | Script Type | Runs On |\n");
    out.push_str("|--------|------------|---------|\n");
    out.push_str("| .server.lua | Script | Server |\n");
    out.push_str("| .client.lua | LocalScript | Client |\n");
    out.push_str("| .lua | ModuleScript | Wherever required |\n");

    // All tracked scripts, grouped by top-level directory
    out.push_str("\n## All Tracked Scripts\n\n");
    let mut by_dir: BTreeMap<String, Vec<(&str, &str, &str)>> = BTreeMap::new();
    for (dm_path, entry) in &manifest.scripts {
        let top_dir = entry.file.split('/').next().unwrap_or("root");
        by_dir
            .entry(top_dir.to_string())
            .or_default()
            .push((&entry.file, dm_path, &entry.class_name));
    }
    for (dir, mut scripts) in by_dir {
        scripts.sort_by_key(|(file, _, _)| file.to_string());
        out.push_str(&format!("### {dir}/\n"));
        for (file, dm_path, class_name) in scripts {
            out.push_str(&format!("- `{file}` → {dm_path} ({class_name})\n"));
        }
        out.push_str("\n");
    }

    fs_err::write(project_root.join("CONTEXT.md"), out)?;
    Ok(())
}
