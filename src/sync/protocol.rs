use serde::{Deserialize, Serialize};

/// Protocol version for WebSocket communication.
pub const PROTOCOL_VERSION: u32 = 1;

/// Messages FROM the plugin to the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum IncomingMessage {
    #[serde(rename = "initial_sync")]
    InitialSync { scripts: Vec<StudioScriptInfo> },

    #[serde(rename = "source_changed")]
    SourceChanged {
        datamodel_path: String,
        class_name: String,
        source: String,
        checksum: String,
    },

    #[serde(rename = "script_created")]
    ScriptCreated {
        datamodel_path: String,
        class_name: String,
        source: String,
    },

    #[serde(rename = "script_deleted")]
    ScriptDeleted { datamodel_path: String },

    #[serde(rename = "script_moved")]
    ScriptMoved {
        old_datamodel_path: String,
        new_datamodel_path: String,
    },
}

/// Messages FROM the server to the plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OutgoingMessage {
    #[serde(rename = "update_source")]
    UpdateSource {
        datamodel_path: String,
        source: String,
        checksum: String,
    },

    #[serde(rename = "create_script")]
    CreateScript {
        datamodel_path: String,
        class_name: String,
        source: String,
    },

    #[serde(rename = "file_deleted")]
    FileDeleted {
        datamodel_path: String,
        action: String,
    },

    #[serde(rename = "sync_confirmed")]
    SyncConfirmed { count: usize },

    #[serde(rename = "server_info")]
    ServerInfo {
        name: String,
        protocol_version: u32,
        server_version: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudioScriptInfo {
    pub datamodel_path: String,
    pub class_name: String,
    pub source: String,
    #[serde(default)]
    pub checksum: String,
}
