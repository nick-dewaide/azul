use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

use crate::context_file::generate_context_file;
use crate::echo_guard::EchoGuard;
use crate::manifest::{Manifest, ManifestEntry};
use crate::script_registry::{ScriptClass, ScriptEntry, ScriptRegistry};
use crate::sync::conflict::{checksum, has_conflict, mark_deleted, write_studio_sidecar};
use crate::sync::naming::file_path_from_dm;
use crate::sync::protocol::{IncomingMessage, OutgoingMessage, StudioScriptInfo};

pub fn handle_message(
    msg: IncomingMessage,
    manifest: &Arc<RwLock<Manifest>>,
    registry: &Arc<RwLock<ScriptRegistry>>,
    project_root: &Path,
    echo_guard: &mut EchoGuard,
) -> Vec<OutgoingMessage> {
    match msg {
        IncomingMessage::InitialSync { scripts } => {
            handle_initial_sync(scripts, manifest, registry, project_root)
        }
        IncomingMessage::SourceChanged {
            datamodel_path,
            class_name,
            source,
            checksum: cs,
        } => handle_source_changed(
            &datamodel_path,
            &class_name,
            &source,
            &cs,
            manifest,
            registry,
            project_root,
            echo_guard,
        ),
        IncomingMessage::ScriptCreated {
            datamodel_path,
            class_name,
            source,
        } => handle_script_created(
            &datamodel_path,
            &class_name,
            &source,
            manifest,
            registry,
            project_root,
        ),
        IncomingMessage::ScriptDeleted { datamodel_path } => {
            handle_script_deleted(&datamodel_path, manifest, registry, project_root)
        }
        IncomingMessage::ScriptMoved {
            old_datamodel_path,
            new_datamodel_path,
        } => handle_script_moved(
            &old_datamodel_path,
            &new_datamodel_path,
            manifest,
            registry,
            project_root,
        ),
    }
}

fn handle_initial_sync(
    scripts: Vec<StudioScriptInfo>,
    manifest: &Arc<RwLock<Manifest>>,
    registry: &Arc<RwLock<ScriptRegistry>>,
    project_root: &Path,
) -> Vec<OutgoingMessage> {
    let mut count = 0;

    for script_info in &scripts {
        let class = ScriptClass::from_class_name(&script_info.class_name)
            .unwrap_or(ScriptClass::ModuleScript);

        let manifest_guard = manifest.read().unwrap();
        let file_path = match file_path_from_dm(
            &script_info.datamodel_path,
            &class,
            &manifest_guard,
            project_root,
        ) {
            Some(p) => p,
            None => {
                log::warn!(
                    "Could not determine file path for {}",
                    script_info.datamodel_path
                );
                continue;
            }
        };
        drop(manifest_guard);

        if let Some(parent) = file_path.parent() {
            let _ = fs_err::create_dir_all(parent);
        }

        if let Err(e) = fs_err::write(&file_path, &script_info.source) {
            log::error!("Failed to write {}: {}", file_path.display(), e);
            continue;
        }

        let mut reg = registry.write().unwrap();
        reg.add(ScriptEntry {
            datamodel_path: script_info.datamodel_path.clone(),
            file_path: file_path.clone(),
            class_name: class.clone(),
            source: script_info.source.clone(),
            last_modified: SystemTime::now(),
            tracked: true,
        });

        let relative_path = file_path
            .strip_prefix(project_root)
            .unwrap_or(&file_path)
            .to_string_lossy()
            .replace('\\', "/");

        let mut man = manifest.write().unwrap();
        man.add_script(
            &script_info.datamodel_path,
            ManifestEntry::new(relative_path, class.as_str().to_string()),
        );

        count += 1;
    }

    {
        let man = manifest.read().unwrap();
        if let Err(e) = man.save(project_root) {
            log::error!("Failed to save manifest: {}", e);
        }
        if let Err(e) = generate_context_file(&man, project_root) {
            log::error!("Failed to generate CONTEXT.md: {}", e);
        }
    }

    log::info!("Initial sync complete: {} scripts synced", count);
    vec![OutgoingMessage::SyncConfirmed { count }]
}

fn handle_source_changed(
    datamodel_path: &str,
    _class_name: &str,
    source: &str,
    _studio_checksum: &str,
    manifest: &Arc<RwLock<Manifest>>,
    registry: &Arc<RwLock<ScriptRegistry>>,
    project_root: &Path,
    echo_guard: &mut EchoGuard,
) -> Vec<OutgoingMessage> {
    let reg = registry.read().unwrap();
    let entry = match reg.get_by_dm_path(datamodel_path) {
        Some(e) => e,
        None => {
            log::warn!("Source changed for untracked script: {}", datamodel_path);
            return vec![];
        }
    };

    let file_path = entry.file_path.clone();
    let last_known_source = entry.source.clone();
    drop(reg);

    let current_file_content = fs_err::read_to_string(&file_path).unwrap_or_default();
    let last_known_cs = checksum(&last_known_source);

    if has_conflict(&last_known_cs, &current_file_content) {
        log::warn!("Conflict on {}: both sides changed", datamodel_path);
        let _ = write_studio_sidecar(&file_path, source);
    } else {
        echo_guard.mark_written(file_path.clone());
        if let Err(e) = fs_err::write(&file_path, source) {
            log::error!("Failed to write {}: {}", file_path.display(), e);
            return vec![];
        }
    }

    let mut reg = registry.write().unwrap();
    reg.update_source(datamodel_path, source.to_string(), SystemTime::now());
    drop(reg);

    let mut man = manifest.write().unwrap();
    man.update_last_sync(datamodel_path);
    let _ = man.save(project_root);

    vec![]
}

fn handle_script_created(
    datamodel_path: &str,
    class_name: &str,
    source: &str,
    manifest: &Arc<RwLock<Manifest>>,
    registry: &Arc<RwLock<ScriptRegistry>>,
    project_root: &Path,
) -> Vec<OutgoingMessage> {
    let class = ScriptClass::from_class_name(class_name).unwrap_or(ScriptClass::ModuleScript);

    let man = manifest.read().unwrap();
    let file_path = match file_path_from_dm(datamodel_path, &class, &man, project_root) {
        Some(p) => p,
        None => {
            log::warn!(
                "Could not determine file path for new script: {}",
                datamodel_path
            );
            return vec![];
        }
    };
    drop(man);

    if let Some(parent) = file_path.parent() {
        let _ = fs_err::create_dir_all(parent);
    }

    if let Err(e) = fs_err::write(&file_path, source) {
        log::error!("Failed to write new script {}: {}", file_path.display(), e);
        return vec![];
    }

    let relative_path = file_path
        .strip_prefix(project_root)
        .unwrap_or(&file_path)
        .to_string_lossy()
        .replace('\\', "/");

    let mut reg = registry.write().unwrap();
    reg.add(ScriptEntry {
        datamodel_path: datamodel_path.to_string(),
        file_path,
        class_name: class.clone(),
        source: source.to_string(),
        last_modified: SystemTime::now(),
        tracked: true,
    });

    let mut man = manifest.write().unwrap();
    man.add_script(
        datamodel_path,
        ManifestEntry::new(relative_path, class.as_str().to_string()),
    );
    let _ = man.save(project_root);
    let _ = generate_context_file(&man, project_root);

    log::info!("New script from Studio: {}", datamodel_path);
    vec![]
}

fn handle_script_deleted(
    datamodel_path: &str,
    manifest: &Arc<RwLock<Manifest>>,
    registry: &Arc<RwLock<ScriptRegistry>>,
    project_root: &Path,
) -> Vec<OutgoingMessage> {
    let mut reg = registry.write().unwrap();
    if let Some(entry) = reg.remove(datamodel_path) {
        let _ = mark_deleted(&entry.file_path);
    }
    drop(reg);

    let mut man = manifest.write().unwrap();
    man.remove_script(datamodel_path);
    let _ = man.save(project_root);
    let _ = generate_context_file(&man, project_root);

    log::info!("Script deleted in Studio: {}", datamodel_path);
    vec![]
}

fn handle_script_moved(
    old_path: &str,
    new_path: &str,
    manifest: &Arc<RwLock<Manifest>>,
    registry: &Arc<RwLock<ScriptRegistry>>,
    project_root: &Path,
) -> Vec<OutgoingMessage> {
    let mut reg = registry.write().unwrap();
    if let Some(mut entry) = reg.remove(old_path) {
        let man = manifest.read().unwrap();
        if let Some(new_file_path) =
            file_path_from_dm(new_path, &entry.class_name, &man, project_root)
        {
            drop(man);

            if let Some(parent) = new_file_path.parent() {
                let _ = fs_err::create_dir_all(parent);
            }
            let _ = fs_err::rename(&entry.file_path, &new_file_path);

            entry.datamodel_path = new_path.to_string();
            entry.file_path = new_file_path;
            reg.add(entry.clone());

            drop(reg);
            let mut man = manifest.write().unwrap();
            man.remove_script(old_path);
            let relative = entry
                .file_path
                .strip_prefix(project_root)
                .unwrap_or(&entry.file_path)
                .to_string_lossy()
                .replace('\\', "/");
            man.add_script(
                new_path,
                ManifestEntry::new(relative, entry.class_name.as_str().to_string()),
            );
            let _ = man.save(project_root);
            let _ = generate_context_file(&man, project_root);
        }
    }

    log::info!("Script moved: {} -> {}", old_path, new_path);
    vec![]
}
