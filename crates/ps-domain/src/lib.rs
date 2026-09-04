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
}
