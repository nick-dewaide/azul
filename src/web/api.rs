use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use hyper::body::Bytes;
use hyper::{Body, Method, Request, Response, StatusCode};

use crate::echo_guard::EchoGuard;
use crate::manifest::Manifest;
use crate::script_registry::ScriptRegistry;
use crate::sync::protocol::{IncomingMessage, OutgoingMessage, PROTOCOL_VERSION};

use super::handler;

/// Thread-safe queue of messages waiting for the plugin to pick up via polling.
pub type MessageQueue = Arc<Mutex<VecDeque<OutgoingMessage>>>;

pub fn new_message_queue() -> MessageQueue {
    Arc::new(Mutex::new(VecDeque::new()))
}

pub async fn call(
    req: Request<Body>,
    manifest: Arc<RwLock<Manifest>>,
    registry: Arc<RwLock<ScriptRegistry>>,
    project_root: Arc<PathBuf>,
    message_queue: MessageQueue,
    echo_guard: Arc<Mutex<EchoGuard>>,
) -> Response<Body> {
    let path = req.uri().path().to_string();
    let method = req.method().clone();

    if path != "/api/poll" {
        log::info!("{} {}", method, path);
    }

    match (method, path.as_str()) {
        (Method::GET, "/api/scriptsync") => server_info(),
        (Method::POST, "/api/sync") => {
            handle_sync_post(req, manifest, registry, project_root, message_queue, echo_guard)
                .await
        }
        (Method::GET, "/api/poll") => handle_poll(message_queue),
        (Method::OPTIONS, _) => cors_preflight(),
        _ => not_found(),
    }
}

fn server_info() -> Response<Body> {
    let info = OutgoingMessage::ServerInfo {
        name: "ScriptSync".to_string(),
        protocol_version: PROTOCOL_VERSION,
        server_version: env!("CARGO_PKG_VERSION").to_string(),
    };

    let body = serde_json::to_string(&info).unwrap_or_default();

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(Body::from(body))
        .unwrap()
}

async fn handle_sync_post(
    req: Request<Body>,
    manifest: Arc<RwLock<Manifest>>,
    registry: Arc<RwLock<ScriptRegistry>>,
    project_root: Arc<PathBuf>,
    message_queue: MessageQueue,
    echo_guard: Arc<Mutex<EchoGuard>>,
) -> Response<Body> {
    let body_bytes: Bytes = match hyper::body::to_bytes(req.into_body()).await {
        Ok(b) => b,
        Err(e) => {
            log::error!("Failed to read request body: {}", e);
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header("Access-Control-Allow-Origin", "*")
                .body(Body::from("Failed to read body"))
                .unwrap();
        }
    };

    let incoming: IncomingMessage = match serde_json::from_slice(&body_bytes) {
        Ok(m) => m,
        Err(e) => {
            log::warn!("Invalid message from plugin: {}", e);
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header("Access-Control-Allow-Origin", "*")
                .body(Body::from(format!("Invalid JSON: {}", e)))
                .unwrap();
        }
    };

    log::debug!("Received from plugin: {:?}", incoming);

    let responses = {
        let mut guard = echo_guard.lock().unwrap();
        handler::handle_message(incoming, &manifest, &registry, &project_root, &mut guard)
    };

    // Any immediate responses (like SyncConfirmed) go back in the HTTP response.
    // Any async responses (like UpdateSource) get queued for polling.
    let (immediate, queued): (Vec<_>, Vec<_>) = responses.into_iter().partition(|msg| {
        matches!(
            msg,
            OutgoingMessage::SyncConfirmed { .. } | OutgoingMessage::ServerInfo { .. }
        )
    });

    if !queued.is_empty() {
        let mut q = message_queue.lock().unwrap();
        for msg in queued {
            q.push_back(msg);
        }
    }

    let body = serde_json::to_string(&immediate).unwrap_or_else(|_| "[]".to_string());

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(Body::from(body))
        .unwrap()
}

/// Plugin polls this endpoint to get pending server→plugin messages.
fn handle_poll(message_queue: MessageQueue) -> Response<Body> {
    let messages: Vec<OutgoingMessage> = {
        let mut q = message_queue.lock().unwrap();
        q.drain(..).collect()
    };

    if !messages.is_empty() {
        log::info!("Poll: sending {} messages to plugin", messages.len());
    }

    let body = serde_json::to_string(&messages).unwrap_or_else(|_| "[]".to_string());

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(Body::from(body))
        .unwrap()
}

fn cors_preflight() -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        .header("Access-Control-Allow-Headers", "Content-Type")
        .body(Body::empty())
        .unwrap()
}

fn not_found() -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from("Not Found"))
        .unwrap()
}
