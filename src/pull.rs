use std::io::BufReader;
use std::path::Path;

use anyhow::{bail, Context, Result};
use rbx_dom_weak::WeakDom;

use crate::context_file::generate_context_file;
use crate::manifest::{default_watch_services, ManifestEntry, Manifest};
use crate::script_registry::ScriptClass;
use crate::sync::naming::file_path_from_dm;

/// Extract all scripts from a .rbxl or .rbxlx file and write them to disk.
pub fn extract_scripts_from_file(input_path: &Path, output_path: &Path) -> Result<()> {
    let ext = input_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let dom = match ext {
        "rbxl" => {
            let file = fs_err::File::open(input_path)
                .with_context(|| format!("Failed to open {}", input_path.display()))?;
            let reader = BufReader::new(file);
            rbx_binary::from_reader(reader)
                .with_context(|| format!("Failed to parse {}", input_path.display()))?
        }
        "rbxlx" => {
            let file = fs_err::File::open(input_path)
                .with_context(|| format!("Failed to open {}", input_path.display()))?;
            let reader = BufReader::new(file);
            rbx_xml::from_reader_default(reader)
                .with_context(|| format!("Failed to parse {}", input_path.display()))?
        }
        _ => bail!("Unsupported file type: .{}. Expected .rbxl or .rbxlx", ext),
    };

    // Load or create manifest
    let mut manifest = if Manifest::exists(output_path) {
        Manifest::load(output_path)?
    } else {
        let name = output_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("project")
            .to_string();
        Manifest::new(name, "mirrored".to_string(), default_watch_services())
    };

    let mut count = 0;

    // Walk the DOM tree and extract scripts
    walk_dom(&dom, dom.root_ref(), &mut |path, class_name, source| {
        let class = match ScriptClass::from_class_name(class_name) {
            Some(c) => c,
            None => return,
        };

        let dm_path = path.join(".");

        let file_path = match file_path_from_dm(&dm_path, &class, &manifest, output_path) {
            Some(p) => p,
            None => {
                log::warn!("Could not determine file path for {}", dm_path);
                return;
            }
        };

        // Create parent directories
        if let Some(parent) = file_path.parent() {
            let _ = fs_err::create_dir_all(parent);
        }

        // Write the script file
        if let Err(e) = fs_err::write(&file_path, source) {
            log::error!("Failed to write {}: {}", file_path.display(), e);
            return;
        }

        // Add to manifest
        let relative = file_path
            .strip_prefix(output_path)
            .unwrap_or(&file_path)
            .to_string_lossy()
            .replace('\\', "/");
        manifest.add_script(
            &dm_path,
            ManifestEntry::new(relative, class.as_str().to_string()),
        );

        count += 1;
    });

    // Save manifest
    manifest.save(output_path)?;
    generate_context_file(&manifest, output_path)?;

    println!("Extracted {} scripts to {}", count, output_path.display());
    println!("Manifest and CONTEXT.md updated.");

    Ok(())
}

fn walk_dom(
    dom: &WeakDom,
    referent: rbx_dom_weak::types::Ref,
    callback: &mut dyn FnMut(&[String], &str, &str),
) {
    walk_dom_recursive(dom, referent, &mut vec![], callback);
}

fn walk_dom_recursive(
    dom: &WeakDom,
    referent: rbx_dom_weak::types::Ref,
    path: &mut Vec<String>,
    callback: &mut dyn FnMut(&[String], &str, &str),
) {
    let instance = match dom.get_by_ref(referent) {
        Some(inst) => inst,
        None => return,
    };

    let class_name = &instance.class;
    let name = &instance.name;

    // Don't push the root DataModel name
    let is_root = referent == dom.root_ref();
    if !is_root {
        path.push(name.clone());
    }

    // Check if this is a script type
    if class_name == "Script" || class_name == "LocalScript" || class_name == "ModuleScript" {
        // Properties are keyed by Ustr in rbx_dom_weak v4, so iterate to find "Source"
        let source = instance.properties.iter().find_map(|(key, val)| {
            if key.as_str() == "Source" {
                if let rbx_dom_weak::types::Variant::String(s) = val {
                    Some(s.as_str())
                } else {
                    None
                }
            } else {
                None
            }
        });
        if let Some(source) = source {
            callback(path, class_name, source);
        }
    }

    // Recurse into children
    for &child_ref in instance.children() {
        walk_dom_recursive(dom, child_ref, path, callback);
    }

    if !is_root {
        path.pop();
    }
}
