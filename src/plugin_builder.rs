use std::io::BufWriter;
use std::path::Path;

use anyhow::{Context, Result};
use rbx_dom_weak::types::Variant;
use rbx_dom_weak::{InstanceBuilder, WeakDom};

/// Build the ScriptSync plugin from Lua source files into a .rbxm file.
pub fn build_plugin(plugin_src_dir: &Path, output_path: &Path) -> Result<()> {
    // Create the root Folder named "ScriptSync"
    let mut dom = WeakDom::new(InstanceBuilder::new("Folder").with_name("ScriptSync"));
    let root_ref = dom.root_ref();

    // Read init.server.lua as the main plugin Script
    let init_path = plugin_src_dir.join("init.server.lua");
    let init_source = fs_err::read_to_string(&init_path)
        .with_context(|| format!("Failed to read {}", init_path.display()))?;

    let plugin_script = InstanceBuilder::new("Script")
        .with_name("ScriptSync")
        .with_property("Source", Variant::String(init_source));
    let plugin_ref = dom.insert(root_ref, plugin_script);

    // Add each module script as a child of the plugin script
    let modules = &[
        ("Config", "Config.lua"),
        ("ScriptWatcher", "ScriptWatcher.lua"),
        ("SyncClient", "SyncClient.lua"),
        ("PathResolver", "PathResolver.lua"),
        ("UI", "UI.lua"),
    ];

    for (name, filename) in modules {
        let file_path = plugin_src_dir.join(filename);
        let source = fs_err::read_to_string(&file_path)
            .with_context(|| format!("Failed to read {}", file_path.display()))?;

        let module = InstanceBuilder::new("ModuleScript")
            .with_name(*name)
            .with_property("Source", Variant::String(source));
        dom.insert(plugin_ref, module);
    }

    // Write the .rbxm file
    if let Some(parent) = output_path.parent() {
        fs_err::create_dir_all(parent)?;
    }

    let file = fs_err::File::create(output_path)
        .with_context(|| format!("Failed to create {}", output_path.display()))?;
    let writer = BufWriter::new(file);

    // Serialize only the children of root (the plugin script), not the root Folder itself
    let children: Vec<_> = dom.root().children().to_vec();
    rbx_binary::to_writer(writer, &dom, &children)
        .context("Failed to serialize plugin to .rbxm")?;

    log::info!("Plugin built: {}", output_path.display());
    Ok(())
}
