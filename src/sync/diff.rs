use crate::script_registry::ScriptRegistry;

#[derive(Debug)]
pub enum DiffResult {
    /// Script exists in Studio but not on disk
    StudioOnly {
        dm_path: String,
        class_name: String,
        source: String,
    },
    /// Script exists on disk but not in Studio
    DiskOnly {
        dm_path: String,
        file_path: String,
    },
    /// Script exists on both sides with different content
    ContentMismatch {
        dm_path: String,
        disk_source: String,
        studio_source: String,
    },
    /// Script exists on both sides with same content
    InSync {
        dm_path: String,
    },
}

/// Compare the current registry (disk state) against a list of scripts from Studio.
pub fn diff_studio_vs_disk(
    registry: &ScriptRegistry,
    studio_scripts: &[StudioScript],
) -> Vec<DiffResult> {
    let mut results = Vec::new();

    // Check each Studio script against disk
    for studio in studio_scripts {
        match registry.get_by_dm_path(&studio.datamodel_path) {
            Some(disk_entry) => {
                if disk_entry.source == studio.source {
                    results.push(DiffResult::InSync {
                        dm_path: studio.datamodel_path.clone(),
                    });
                } else {
                    results.push(DiffResult::ContentMismatch {
                        dm_path: studio.datamodel_path.clone(),
                        disk_source: disk_entry.source.clone(),
                        studio_source: studio.source.clone(),
                    });
                }
            }
            None => {
                results.push(DiffResult::StudioOnly {
                    dm_path: studio.datamodel_path.clone(),
                    class_name: studio.class_name.clone(),
                    source: studio.source.clone(),
                });
            }
        }
    }

    // Check for scripts on disk but not in Studio
    for entry in registry.all_entries() {
        let in_studio = studio_scripts
            .iter()
            .any(|s| s.datamodel_path == entry.datamodel_path);

        if !in_studio {
            results.push(DiffResult::DiskOnly {
                dm_path: entry.datamodel_path.clone(),
                file_path: entry.file_path.to_string_lossy().to_string(),
            });
        }
    }

    results
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct StudioScript {
    pub datamodel_path: String,
    pub class_name: String,
    pub source: String,
    #[serde(default)]
    pub checksum: String,
}
