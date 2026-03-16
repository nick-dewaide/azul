use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Result;
use crossbeam_channel::Sender;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

#[derive(Debug, Clone)]
pub enum FileEvent {
    Modified(PathBuf),
    Created(PathBuf),
    Deleted(PathBuf),
    Renamed { from: PathBuf, to: PathBuf },
}

pub struct FileWatcher {
    _watcher: RecommendedWatcher,
}

impl FileWatcher {
    pub fn new(root: PathBuf, tx: Sender<FileEvent>) -> Result<Self> {
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            let event = match res {
                Ok(event) => event,
                Err(e) => {
                    log::warn!("File watcher error: {}", e);
                    return;
                }
            };

            for path in &event.paths {
                if !should_watch(path) {
                    return;
                }
            }

            match event.kind {
                EventKind::Create(_) => {
                    for path in event.paths {
                        let _ = tx.send(FileEvent::Created(path));
                    }
                }
                EventKind::Modify(_) => {
                    for path in event.paths {
                        let _ = tx.send(FileEvent::Modified(path));
                    }
                }
                EventKind::Remove(_) => {
                    for path in event.paths {
                        let _ = tx.send(FileEvent::Deleted(path));
                    }
                }
                _ => {}
            }
        })?;

        watcher.configure(Config::default().with_poll_interval(Duration::from_millis(200)))?;
        watcher.watch(&root, RecursiveMode::Recursive)?;

        Ok(FileWatcher {
            _watcher: watcher,
        })
    }
}

fn should_watch(path: &Path) -> bool {
    let path_str = path.to_string_lossy();

    // Ignore .git, node_modules, .deleted files, manifest, context file
    if path_str.contains(".git")
        || path_str.contains("node_modules")
        || path_str.ends_with(".deleted")
        || path_str.ends_with(".studio")
        || path_str.ends_with("scriptsync.json")
        || path_str.ends_with("CONTEXT.md")
    {
        return false;
    }

    // Only watch .lua and .luau files
    if let Some(ext) = path.extension() {
        let ext = ext.to_string_lossy();
        ext == "lua" || ext == "luau"
    } else {
        false
    }
}
