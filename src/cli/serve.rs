use std::{
    io::{self, Write},
    net::{IpAddr, Ipv4Addr},
    path::PathBuf,
    sync::{Arc, Mutex, RwLock},
};

use clap::Parser;
use crossbeam_channel::Receiver;
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

use crate::echo_guard::EchoGuard;
use crate::manifest::Manifest;
use crate::script_registry::ScriptRegistry;
use crate::sync::conflict::checksum;
use crate::sync::naming::{class_from_file, dm_path_from_file};
use crate::sync::protocol::OutgoingMessage;
use crate::watcher::{FileEvent, FileWatcher};
use crate::web::api::MessageQueue;
use crate::web::LiveServer;

use super::{resolve_path, GlobalOptions};

const DEFAULT_BIND_ADDRESS: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);
const DEFAULT_PORT: u16 = 34872;

/// Start the ScriptSync server. Watches the filesystem and serves HTTP
/// endpoints for the Roblox Studio plugin.
#[derive(Debug, Parser)]
pub struct ServeCommand {
    /// Path to the project directory. Defaults to the current directory.
    #[clap(default_value = "")]
    pub project: PathBuf,

    /// The IP address to listen on. Defaults to `127.0.0.1`.
    #[clap(long)]
    pub address: Option<IpAddr>,

    /// The port to listen on. Defaults to `34872`.
    #[clap(long)]
    pub port: Option<u16>,
}

impl ServeCommand {
    pub fn run(self, global: GlobalOptions) -> anyhow::Result<()> {
        let project_path = resolve_path(&self.project);

        let manifest = Manifest::load(&project_path)?;
        let registry = ScriptRegistry::from_manifest(&manifest, &project_path);

        let ip = self.address.unwrap_or(DEFAULT_BIND_ADDRESS.into());
        let port = self.port.unwrap_or(DEFAULT_PORT);

        let manifest = Arc::new(RwLock::new(manifest));
        let registry = Arc::new(RwLock::new(registry));
        let project_root = Arc::new(project_path.into_owned());

        let server = LiveServer::new(
            Arc::clone(&manifest),
            Arc::clone(&registry),
            Arc::clone(&project_root),
        );

        let message_queue = server.message_queue();
        let echo_guard = server.echo_guard();

        // Start filesystem watcher
        let (tx, rx) = crossbeam_channel::unbounded();
        let _watcher = FileWatcher::new((*project_root).clone(), tx)?;

        // Spawn a thread to process file events and push messages to the queue
        let watcher_manifest = Arc::clone(&manifest);
        let watcher_registry = Arc::clone(&registry);
        let watcher_root = Arc::clone(&project_root);
        std::thread::spawn(move || {
            process_file_events(
                rx,
                watcher_manifest,
                watcher_registry,
                watcher_root,
                message_queue,
                echo_guard,
            );
        });

        let _ = show_start_message(ip, port, global.color.into());
        server.start((ip, port).into());

        Ok(())
    }
}

fn process_file_events(
    rx: Receiver<FileEvent>,
    manifest: Arc<RwLock<Manifest>>,
    registry: Arc<RwLock<ScriptRegistry>>,
    project_root: Arc<PathBuf>,
    message_queue: MessageQueue,
    echo_guard: Arc<Mutex<EchoGuard>>,
) {
    while let Ok(event) = rx.recv() {
        match event {
            FileEvent::Modified(path) => {
                // Check echo guard — skip if we just wrote this file from a Studio sync
                {
                    let mut guard = echo_guard.lock().unwrap();
                    if guard.should_ignore(&path) {
                        continue;
                    }
                }

                let source = match fs_err::read_to_string(&path) {
                    Ok(s) => s,
                    Err(e) => {
                        log::warn!("Failed to read modified file {}: {}", path.display(), e);
                        continue;
                    }
                };

                // Look up DM path: try naming convention first, then registry
                let dm_path = {
                    let man = manifest.read().unwrap();
                    dm_path_from_file(&path, &man, &project_root)
                }
                .or_else(|| {
                    let reg = registry.read().unwrap();
                    reg.get_by_file(&path)
                        .map(|entry| entry.datamodel_path.clone())
                });

                let dm_path = match dm_path {
                    Some(p) => p,
                    None => {
                        log::debug!("Modified file not tracked: {}", path.display());
                        continue;
                    }
                };

                let cs = checksum(&source);

                // Update registry
                {
                    let mut reg = registry.write().unwrap();
                    reg.update_source(&dm_path, source.clone(), std::time::SystemTime::now());
                }

                log::info!("File changed on disk: {} -> {}", path.display(), dm_path);

                let msg = OutgoingMessage::UpdateSource {
                    datamodel_path: dm_path,
                    source,
                    checksum: cs,
                };

                let mut q = message_queue.lock().unwrap();
                q.push_back(msg);
            }
            FileEvent::Created(path) => {
                let source = match fs_err::read_to_string(&path) {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                let class = class_from_file(&path);

                let man = manifest.read().unwrap();
                let dm_path = match dm_path_from_file(&path, &man, &project_root) {
                    Some(p) => p,
                    None => continue,
                };
                drop(man);

                log::info!("New file on disk: {} -> {}", path.display(), dm_path);

                let msg = OutgoingMessage::CreateScript {
                    datamodel_path: dm_path,
                    class_name: class.as_str().to_string(),
                    source,
                };

                let mut q = message_queue.lock().unwrap();
                q.push_back(msg);
            }
            FileEvent::Deleted(path) => {
                let reg = registry.read().unwrap();
                let dm_path = match reg.get_by_file(&path) {
                    Some(entry) => entry.datamodel_path.clone(),
                    None => continue,
                };
                drop(reg);

                log::info!("File deleted on disk: {} -> {}", path.display(), dm_path);

                let msg = OutgoingMessage::FileDeleted {
                    datamodel_path: dm_path,
                    action: "notify".to_string(),
                };

                let mut q = message_queue.lock().unwrap();
                q.push_back(msg);
            }
            FileEvent::Renamed { from, to } => {
                log::debug!("File renamed: {} -> {}", from.display(), to.display());
            }
        }
    }
}

fn show_start_message(bind_address: IpAddr, port: u16, color: ColorChoice) -> io::Result<()> {
    let mut green = ColorSpec::new();
    green.set_fg(Some(Color::Green)).set_bold(true);

    let writer = BufferWriter::stdout(color);
    let mut buffer = writer.buffer();

    let address_string = if bind_address.is_loopback() {
        "localhost".to_owned()
    } else {
        bind_address.to_string()
    };

    writeln!(&mut buffer, "ScriptSync server listening:")?;

    write!(&mut buffer, "  Address: ")?;
    buffer.set_color(&green)?;
    writeln!(&mut buffer, "{}", address_string)?;

    buffer.set_color(&ColorSpec::new())?;
    write!(&mut buffer, "  Port:    ")?;
    buffer.set_color(&green)?;
    writeln!(&mut buffer, "{}", port)?;

    writeln!(&mut buffer)?;

    buffer.set_color(&ColorSpec::new())?;
    writeln!(
        &mut buffer,
        "Waiting for Roblox Studio plugin to connect..."
    )?;

    writer.print(&buffer)?;

    Ok(())
}
