pub mod api;
pub mod handler;

use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use hyper::{
    server::Server,
    service::{make_service_fn, service_fn},
    Body, Request,
};
use tokio::runtime::Runtime;

use crate::echo_guard::EchoGuard;
use crate::manifest::Manifest;
use crate::script_registry::ScriptRegistry;

use api::{new_message_queue, MessageQueue};

pub struct LiveServer {
    manifest: Arc<RwLock<Manifest>>,
    registry: Arc<RwLock<ScriptRegistry>>,
    project_root: Arc<PathBuf>,
    message_queue: MessageQueue,
    echo_guard: Arc<Mutex<EchoGuard>>,
}

impl LiveServer {
    pub fn new(
        manifest: Arc<RwLock<Manifest>>,
        registry: Arc<RwLock<ScriptRegistry>>,
        project_root: Arc<PathBuf>,
    ) -> Self {
        LiveServer {
            manifest,
            registry,
            project_root,
            message_queue: new_message_queue(),
            echo_guard: Arc::new(Mutex::new(EchoGuard::new())),
        }
    }

    /// Returns a clone of the message queue so callers (e.g. the file watcher)
    /// can push outgoing messages for the plugin to pick up via polling.
    pub fn message_queue(&self) -> MessageQueue {
        Arc::clone(&self.message_queue)
    }

    /// Returns a clone of the echo guard so the file watcher can check it.
    pub fn echo_guard(&self) -> Arc<Mutex<EchoGuard>> {
        Arc::clone(&self.echo_guard)
    }

    pub fn start(self, address: SocketAddr) {
        let manifest = self.manifest;
        let registry = self.registry;
        let project_root = self.project_root;
        let message_queue = self.message_queue;
        let echo_guard = self.echo_guard;

        let make_service = make_service_fn(move |_conn| {
            let manifest = Arc::clone(&manifest);
            let registry = Arc::clone(&registry);
            let project_root = Arc::clone(&project_root);
            let message_queue = Arc::clone(&message_queue);
            let echo_guard = Arc::clone(&echo_guard);

            async {
                let service = move |req: Request<Body>| {
                    let manifest = Arc::clone(&manifest);
                    let registry = Arc::clone(&registry);
                    let project_root = Arc::clone(&project_root);
                    let message_queue = Arc::clone(&message_queue);
                    let echo_guard = Arc::clone(&echo_guard);

                    async move {
                        Ok::<_, Infallible>(
                            api::call(
                                req,
                                manifest,
                                registry,
                                project_root,
                                message_queue,
                                echo_guard,
                            )
                            .await,
                        )
                    }
                };

                Ok::<_, Infallible>(service_fn(service))
            }
        });

        let rt = Runtime::new().unwrap();
        let _guard = rt.enter();
        let server = Server::bind(&address).serve(make_service);
        rt.block_on(server).unwrap();
    }
}
