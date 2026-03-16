use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const MANIFEST_FILENAME: &str = "scriptsync.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version: u32,
    pub scripts: HashMap<String, ManifestEntry>,
    pub settings: ManifestSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub file: String,
    #[serde(rename = "className")]
    pub class_name: String,
    #[serde(rename = "lastSync")]
    pub last_sync: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestSettings {
    #[serde(rename = "syncDirection")]
    pub sync_direction: String,
    #[serde(rename = "fileOrganization")]
    pub file_organization: String,
    #[serde(rename = "watchServices")]
    pub watch_services: Vec<String>,
    #[serde(rename = "serviceDirs", default)]
    pub service_dirs: HashMap<String, String>,
}

impl Manifest {
    pub fn new(name: String, file_organization: String, watch_services: Vec<String>) -> Self {
        let service_dirs = default_service_dirs();
        Manifest {
            name,
            version: 1,
            scripts: HashMap::new(),
            settings: ManifestSettings {
                sync_direction: "bidirectional".to_string(),
                file_organization,
                watch_services,
                service_dirs,
            },
        }
    }

    pub fn load(project_root: &Path) -> Result<Self> {
        let path = project_root.join(MANIFEST_FILENAME);
        let content = fs_err::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let manifest: Manifest = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse {}", path.display()))?;
        Ok(manifest)
    }

    pub fn save(&self, project_root: &Path) -> Result<()> {
        let path = project_root.join(MANIFEST_FILENAME);
        let content = serde_json::to_string_pretty(self)?;
        fs_err::write(&path, content)?;
        Ok(())
    }

    pub fn manifest_path(project_root: &Path) -> PathBuf {
        project_root.join(MANIFEST_FILENAME)
    }

    pub fn exists(project_root: &Path) -> bool {
        project_root.join(MANIFEST_FILENAME).exists()
    }

    pub fn add_script(&mut self, dm_path: &str, entry: ManifestEntry) {
        self.scripts.insert(dm_path.to_string(), entry);
    }

    pub fn remove_script(&mut self, dm_path: &str) -> Option<ManifestEntry> {
        self.scripts.remove(dm_path)
    }

    pub fn get_by_file(&self, file_path: &str) -> Option<(&str, &ManifestEntry)> {
        self.scripts
            .iter()
            .find(|(_, entry)| entry.file == file_path)
            .map(|(k, v)| (k.as_str(), v))
    }

    pub fn get_by_dm_path(&self, dm_path: &str) -> Option<&ManifestEntry> {
        self.scripts.get(dm_path)
    }

    pub fn update_last_sync(&mut self, dm_path: &str) {
        if let Some(entry) = self.scripts.get_mut(dm_path) {
            entry.last_sync = now_unix();
        }
    }
}

impl ManifestEntry {
    pub fn new(file: String, class_name: String) -> Self {
        ManifestEntry {
            file,
            class_name,
            last_sync: now_unix(),
        }
    }
}

pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn default_service_dirs() -> HashMap<String, String> {
    let mut dirs = HashMap::new();
    dirs.insert("ServerScriptService".to_string(), "server".to_string());
    dirs.insert("ServerStorage".to_string(), "server-storage".to_string());
    dirs.insert("ReplicatedStorage".to_string(), "shared".to_string());
    dirs.insert(
        "StarterPlayer.StarterPlayerScripts".to_string(),
        "client".to_string(),
    );
    dirs.insert(
        "StarterPlayer.StarterCharacterScripts".to_string(),
        "character".to_string(),
    );
    dirs.insert("StarterGui".to_string(), "gui".to_string());
    dirs.insert("StarterPack".to_string(), "starterpack".to_string());
    dirs
}

pub fn default_watch_services() -> Vec<String> {
    vec![
        "ServerScriptService".to_string(),
        "ReplicatedStorage".to_string(),
        "StarterPlayer".to_string(),
        "StarterGui".to_string(),
        "ServerStorage".to_string(),
        "StarterPack".to_string(),
    ]
}
