use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub struct EchoGuard {
    recently_written: HashMap<PathBuf, Instant>,
    ttl: Duration,
}

impl EchoGuard {
    pub fn new() -> Self {
        EchoGuard {
            recently_written: HashMap::new(),
            ttl: Duration::from_millis(500),
        }
    }

    pub fn mark_written(&mut self, path: PathBuf) {
        self.recently_written.insert(path, Instant::now());
    }

    pub fn should_ignore(&mut self, path: &Path) -> bool {
        if let Some(when) = self.recently_written.get(path) {
            if when.elapsed() < self.ttl {
                return true;
            }
            self.recently_written.remove(&path.to_path_buf());
        }
        false
    }

    pub fn cleanup_expired(&mut self) {
        self.recently_written
            .retain(|_, when| when.elapsed() < self.ttl);
    }
}
