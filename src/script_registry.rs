use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::manifest::Manifest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptClass {
    Script,
    LocalScript,
    ModuleScript,
}

impl ScriptClass {
    pub fn from_class_name(name: &str) -> Option<Self> {
        match name {
            "Script" => Some(ScriptClass::Script),
            "LocalScript" => Some(ScriptClass::LocalScript),
            "ModuleScript" => Some(ScriptClass::ModuleScript),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ScriptClass::Script => "Script",
            ScriptClass::LocalScript => "LocalScript",
            ScriptClass::ModuleScript => "ModuleScript",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScriptEntry {
    pub datamodel_path: String,
    pub file_path: PathBuf,
    pub class_name: ScriptClass,
    pub source: String,
    pub last_modified: SystemTime,
    pub tracked: bool,
}

pub struct ScriptRegistry {
    by_dm_path: HashMap<String, ScriptEntry>,
    by_file_path: HashMap<PathBuf, String>,
}

impl ScriptRegistry {
    pub fn new() -> Self {
        ScriptRegistry {
            by_dm_path: HashMap::new(),
            by_file_path: HashMap::new(),
        }
    }

    pub fn from_manifest(manifest: &Manifest, project_root: &Path) -> Self {
        let mut registry = Self::new();
        for (dm_path, entry) in &manifest.scripts {
            let file_path = project_root.join(entry.file.replace('/', std::path::MAIN_SEPARATOR_STR));
            let source = fs_err::read_to_string(&file_path).unwrap_or_default();
            let last_modified = file_path
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            let class_name = ScriptClass::from_class_name(&entry.class_name)
                .unwrap_or(ScriptClass::ModuleScript);

            registry.add(ScriptEntry {
                datamodel_path: dm_path.clone(),
                file_path,
                class_name,
                source,
                last_modified,
                tracked: true,
            });
        }
        registry
    }

    pub fn get_by_dm_path(&self, path: &str) -> Option<&ScriptEntry> {
        self.by_dm_path.get(path)
    }

    pub fn get_by_dm_path_mut(&mut self, path: &str) -> Option<&mut ScriptEntry> {
        self.by_dm_path.get_mut(path)
    }

    pub fn get_by_file(&self, path: &Path) -> Option<&ScriptEntry> {
        self.by_file_path
            .get(path)
            .and_then(|dm_path| self.by_dm_path.get(dm_path))
    }

    pub fn dm_path_for_file(&self, path: &Path) -> Option<&str> {
        self.by_file_path.get(path).map(|s| s.as_str())
    }

    pub fn update_source(&mut self, dm_path: &str, source: String, timestamp: SystemTime) {
        if let Some(entry) = self.by_dm_path.get_mut(dm_path) {
            entry.source = source;
            entry.last_modified = timestamp;
        }
    }

    pub fn add(&mut self, entry: ScriptEntry) {
        let dm_path = entry.datamodel_path.clone();
        let file_path = entry.file_path.clone();
        self.by_file_path.insert(file_path, dm_path.clone());
        self.by_dm_path.insert(dm_path, entry);
    }

    pub fn remove(&mut self, dm_path: &str) -> Option<ScriptEntry> {
        if let Some(entry) = self.by_dm_path.remove(dm_path) {
            self.by_file_path.remove(&entry.file_path);
            Some(entry)
        } else {
            None
        }
    }

    pub fn all_entries(&self) -> impl Iterator<Item = &ScriptEntry> {
        self.by_dm_path.values()
    }

    pub fn len(&self) -> usize {
        self.by_dm_path.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_dm_path.is_empty()
    }
}
