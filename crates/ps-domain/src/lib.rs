//! Core domain models and entities for PacketSmith.
//!
//! This crate contains protocol-neutral resource definitions, stable identifier types,
//! authentication configurations, and variable scopes.

use std::fmt;
use std::str::FromStr;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Stable, globally unique identifier for any PacketSmith resource (requests, collections, environments).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ResourceId(Uuid);

impl ResourceId {
    /// Generates a new random ResourceId.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Creates a ResourceId from an existing UUID.
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Access the underlying UUID.
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for ResourceId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ResourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for ResourceId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s).map(Self)
    }
}

/// The top-level protocol-neutral document representing an executable request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestDocument {
    pub id: ResourceId,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub protocol: ProtocolRequest,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub scripts: RequestScripts,
    #[serde(default)]
    pub settings: RequestSettings,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<ResponseExampleRef>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl RequestDocument {
    /// Creates a new request document with default settings.
    pub fn new(name: impl Into<String>, protocol: ProtocolRequest) -> Self {
        let now = Utc::now();
        Self {
            id: ResourceId::new(),
            name: name.into(),
            description: None,
            tags: Vec::new(),
            protocol,
            auth: AuthConfig::default(),
            scripts: RequestScripts::default(),
            settings: RequestSettings::default(),
            examples: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// Supported protocol request payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProtocolRequest {
    Http(HttpRequestPayload),
    GraphQl(GraphQlRequestPayload),
    Grpc(GrpcRequestPayload),
    WebSocket(WebSocketRequestPayload),
    SocketIo(SocketIoRequestPayload),
    Mqtt(MqttRequestPayload),
    Mcp(McpRequestPayload),
    Ai(AiRequestPayload),
    Soap(SoapRequestPayload),
}

/// Minimal placeholder payload for HTTP requests within the domain crate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpRequestPayload {
    pub method: String,
    pub url: String,
}

/// GraphQL request configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GraphQlRequestPayload {
    pub endpoint: String,
    pub query: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variables: Option<String>,
}

/// gRPC request configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GrpcRequestPayload {
    pub service: String,
    pub method: String,
    pub payload_json: String,
}

/// WebSocket request configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct WebSocketRequestPayload {
    pub url: String,
}

/// Socket.IO request configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SocketIoRequestPayload {
    pub url: String,
    pub event_name: String,
}

/// MQTT request configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MqttRequestPayload {
    pub broker_url: String,
    pub topic: String,
}

/// Model Context Protocol (MCP) request configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct McpRequestPayload {
    pub server_command: String,
    pub tool_name: String,
    pub arguments_json: String,
}

/// AI model prompt/request configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AiRequestPayload {
    pub provider: String,
    pub model: String,
    pub prompt: String,
}

/// SOAP request configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SoapRequestPayload {
    pub endpoint: String,
    pub soap_action: Option<String>,
    pub xml_envelope: String,
}

/// Authentication configuration attached to requests or collections.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthConfig {
    #[default]
    Inherit,
    None,
    Basic {
        username: String,
        password_secret_ref: Option<String>,
    },
    Bearer {
        token_secret_ref: String,
    },
    ApiKey {
        key: String,
        value_secret_ref: String,
        location: ApiKeyLocation,
    },
    OAuth2 {
        flow: String,
        token_secret_ref: Option<String>,
    },
}

/// Location of an API key in the request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiKeyLocation {
    #[default]
    Header,
    Query,
    Cookie,
}

/// User scripts associated with pre-request and post-response phases.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RequestScripts {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_request: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_response: Option<String>,
}

/// Per-request behavior execution settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestSettings {
    pub follow_redirects: bool,
    pub max_redirects: u32,
    pub verify_ssl: bool,
    pub timeout_ms: u64,
}

impl Default for RequestSettings {
    fn default() -> Self {
        Self {
            follow_redirects: true,
            max_redirects: 10,
            verify_ssl: true,
            timeout_ms: 30_000,
        }
    }
}

/// Reference to a saved response example.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseExampleRef {
    pub id: ResourceId,
    pub name: String,
    pub status_code: u16,
}

/// Variable scope classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VariableScope {
    Global,
    Environment,
    Collection,
    Folder,
    Request,
    Ephemeral,
}

/// A variable entry with optional secret masking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariableEntry {
    pub key: String,
    pub value: String,
    pub is_secret: bool,
    pub enabled: bool,
}

impl VariableEntry {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            is_secret: false,
            enabled: true,
        }
    }

    pub fn secret(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            is_secret: true,
            enabled: true,
        }
    }
}

/// Execution configuration for automated collection and folder runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunnerSettings {
    pub iterations: u32,
    pub delay_ms: u64,
    pub stop_on_error: bool,
    pub persist_variables: bool,
}

impl Default for RunnerSettings {
    fn default() -> Self {
        Self {
            iterations: 1,
            delay_ms: 0,
            stop_on_error: false,
            persist_variables: false,
        }
    }
}

/// Collection document grouping related folders, requests, and shared configurations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollectionDocument {
    pub id: ResourceId,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variables: Vec<VariableEntry>,
    #[serde(default)]
    pub scripts: RequestScripts,
    #[serde(default)]
    pub runner_settings: RunnerSettings,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub item_order: Vec<ResourceId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CollectionDocument {
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: ResourceId::new(),
            name: name.into(),
            description: None,
            auth: AuthConfig::default(),
            variables: Vec::new(),
            scripts: RequestScripts::default(),
            runner_settings: RunnerSettings::default(),
            item_order: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// Nested folder document within a collection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FolderDocument {
    pub id: ResourceId,
    pub collection_id: ResourceId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<ResourceId>,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variables: Vec<VariableEntry>,
    #[serde(default)]
    pub scripts: RequestScripts,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub item_order: Vec<ResourceId>,
}

impl FolderDocument {
    pub fn new(collection_id: ResourceId, name: impl Into<String>) -> Self {
        Self {
            id: ResourceId::new(),
            collection_id,
            parent_id: None,
            name: name.into(),
            description: None,
            auth: AuthConfig::default(),
            variables: Vec::new(),
            scripts: RequestScripts::default(),
            item_order: Vec::new(),
        }
    }
}

/// Environment document storing variables and environment-specific credentials.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvironmentDocument {
    pub id: ResourceId,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variables: Vec<VariableEntry>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl EnvironmentDocument {
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: ResourceId::new(),
            name: name.into(),
            description: None,
            variables: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// Full response example snapshot associated with a request document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseExample {
    pub id: ResourceId,
    pub request_id: ResourceId,
    pub name: String,
    pub status_code: u16,
    pub status_text: String,
    pub headers: std::collections::HashMap<String, String>,
    pub body: String,
    pub recorded_at: DateTime<Utc>,
}

impl ResponseExample {
    pub fn new(
        request_id: ResourceId,
        name: impl Into<String>,
        status_code: u16,
        status_text: impl Into<String>,
        headers: std::collections::HashMap<String, String>,
        body: impl Into<String>,
    ) -> Self {
        Self {
            id: ResourceId::new(),
            request_id,
            name: name.into(),
            status_code,
            status_text: status_text.into(),
            headers,
            body: body.into(),
            recorded_at: Utc::now(),
        }
    }

    pub fn to_ref(&self) -> ResponseExampleRef {
        ResponseExampleRef {
            id: self.id,
            name: self.name.clone(),
            status_code: self.status_code,
        }
    }
}

/// Metadata recorded when a resource is soft-deleted to `.packetsmith/trash/`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrashMetadata {
    pub resource_id: ResourceId,
    pub original_path: String,
    pub resource_name: String,
    pub resource_type: String,
    pub deleted_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_id_roundtrip() {
        let id = ResourceId::new();
        let s = id.to_string();
        let parsed: ResourceId = s.parse().expect("Failed to parse ResourceId");
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_request_document_serialization() {
        let doc = RequestDocument::new(
            "Get Users",
            ProtocolRequest::Http(HttpRequestPayload {
                method: "GET".to_string(),
                url: "https://api.example.com/users".to_string(),
            }),
        );
        let serialized = serde_json::to_string_pretty(&doc).expect("Serialization failed");
        let deserialized: RequestDocument =
            serde_json::from_str(&serialized).expect("Deserialization failed");
        assert_eq!(doc.id, deserialized.id);
        assert_eq!(doc.name, deserialized.name);
    }

    #[test]
    fn test_collection_and_environment_models() {
        let mut col = CollectionDocument::new("Store API");
        col.variables.push(VariableEntry::new("base_url", "https://api.store.com"));
        let env = EnvironmentDocument::new("Staging");
        assert_eq!(col.name, "Store API");
        assert_eq!(env.name, "Staging");
    }
}
