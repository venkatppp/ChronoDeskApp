//! LLM Models and Data Structures

use serde::{Deserialize, Serialize};

/// LLM provider type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LLMProviderType {
    OpenAI,
    Ollama,
    Custom,
}

impl std::fmt::Display for LLMProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LLMProviderType::OpenAI => write!(f, "openai"),
            LLMProviderType::Ollama => write!(f, "ollama"),
            LLMProviderType::Custom => write!(f, "custom"),
        }
    }
}

/// LLM request message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMMessage {
    pub role: String,
    pub content: String,
    /// Provider-native tool calls attached to an assistant message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<LLMToolCall>>,
    /// For `role = "tool"` messages, links the result to a tool call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl LLMMessage {
    /// Creates a plain (no tool metadata) message.
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Creates a `tool` role message carrying a tool result.
    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: "tool".to_string(),
            content: content.into(),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
        }
    }

    /// Creates an `assistant` message that advertises provider tool calls.
    pub fn assistant_tool_calls(content: impl Into<String>, tool_calls: Vec<LLMToolCall>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
            tool_calls: Some(tool_calls),
            tool_call_id: None,
        }
    }
}

/// A provider-native tool/function call attached to an assistant message.
///
/// Serializes to the OpenAI wire shape (`{id, type: "function", function:
/// {name, arguments}}`) so tool-call rounds can be sent back to the
/// provider verbatim, and deserializes from that same shape. `arguments`
/// is kept as parsed JSON for direct use by the tool executor.
#[derive(Debug, Clone, PartialEq)]
pub struct LLMToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

impl Serialize for LLMToolCall {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("LLMToolCall", 3)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("type", "function")?;
        state.serialize_field(
            "function",
            &serde_json::json!({
                "name": self.name,
                "arguments": wire_arguments(&self.arguments),
            }),
        )?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for LLMToolCall {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        #[derive(Deserialize)]
        struct Wire {
            id: Option<String>,
            name: Option<String>,
            arguments: Option<serde_json::Value>,
            function: Option<WireFunction>,
        }
        #[derive(Deserialize)]
        struct WireFunction {
            name: Option<String>,
            arguments: Option<String>,
        }
        let wire = Wire::deserialize(deserializer)?;
        let (name, arguments) = match wire.function {
            Some(function) => (
                function.name,
                function
                    .arguments
                    .and_then(|args| serde_json::from_str(&args).ok()),
            ),
            None => (wire.name, wire.arguments),
        };
        let id = wire.id.unwrap_or_default();
        if id.is_empty() {
            return Err(D::Error::custom("missing tool call id"));
        }
        Ok(Self {
            id,
            name: name.unwrap_or_default(),
            arguments: arguments.unwrap_or(serde_json::Value::Null),
        })
    }
}

/// A tool/function schema advertised to the LLM so it can request calls.
#[derive(Debug, Clone, PartialEq)]
pub struct LLMTool {
    pub name: String,
    pub description: String,
    pub parameters: LLMToolParameters,
}

impl LLMTool {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: LLMToolParameters,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters,
        }
    }
}

impl Serialize for LLMTool {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("LLMTool", 2)?;
        state.serialize_field("type", "function")?;
        state.serialize_field(
            "function",
            &serde_json::json!({
                "name": self.name,
                "description": self.description,
                "parameters": self.parameters,
            }),
        )?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for LLMTool {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Wire {
            name: Option<String>,
            description: Option<String>,
            parameters: Option<LLMToolParameters>,
            function: Option<WireFunction>,
        }
        #[derive(Deserialize)]
        struct WireFunction {
            name: Option<String>,
            description: Option<String>,
            parameters: Option<LLMToolParameters>,
        }
        let wire = Wire::deserialize(deserializer)?;
        let function = wire.function.unwrap_or(WireFunction {
            name: None,
            description: None,
            parameters: None,
        });
        Ok(Self {
            name: wire.name.or(function.name).unwrap_or_default(),
            description: wire
                .description
                .or(function.description)
                .unwrap_or_default(),
            parameters: wire.parameters.or(function.parameters).unwrap_or_default(),
        })
    }
}

/// The OpenAI-style JSON schema for a tool's parameters.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct LLMToolParameters {
    pub properties: Vec<LLMToolParameter>,
}

impl LLMToolParameters {
    pub fn add(&mut self, property: LLMToolParameter) {
        self.properties.push(property);
    }
}

impl Serialize for LLMToolParameters {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("type", "object")?;
        let mut required = Vec::new();
        let mut properties = serde_json::Map::new();
        for property in &self.properties {
            properties.insert(
                property.name.clone(),
                serde_json::json!({
                    "type": property.param_type.as_str(),
                    "description": property.description,
                }),
            );
            if property.required {
                required.push(property.name.clone());
            }
        }
        map.serialize_entry("properties", &serde_json::Value::Object(properties))?;
        if !required.is_empty() {
            map.serialize_entry("required", &required)?;
        }
        map.end()
    }
}

/// A JSON-level parameter of an LLM tool.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct LLMToolParameter {
    pub name: String,
    pub description: String,
    pub param_type: LLMToolParameterType,
    pub required: bool,
}

impl LLMToolParameter {
    pub fn required(
        name: impl Into<String>,
        param_type: LLMToolParameterType,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            param_type,
            required: true,
        }
    }

    pub fn optional(
        name: impl Into<String>,
        param_type: LLMToolParameterType,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            param_type,
            required: false,
        }
    }
}

/// JSON-level parameter types supported by LLM tool schemas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum LLMToolParameterType {
    String,
    Number,
    Boolean,
    Object,
    Array,
}

impl LLMToolParameterType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Number => "number",
            Self::Boolean => "boolean",
            Self::Object => "object",
            Self::Array => "array",
        }
    }
}

/// LLM completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRequest {
    pub messages: Vec<LLMMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// Provider-native tool schemas available to the model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<LLMTool>>,
}

impl Default for LLMRequest {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            temperature: Some(0.7),
            max_tokens: Some(2000),
            top_p: Some(1.0),
            stream: Some(false),
            tools: None,
        }
    }
}

/// LLM completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    pub content: String,
    pub usage: TokenUsage,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    /// Provider-native tool calls requested by the model (when any).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<LLMToolCall>>,
}

/// Token usage information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

impl TokenUsage {
    pub fn new(prompt_tokens: usize, completion_tokens: usize) -> Self {
        Self {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
        }
    }

    pub fn add(&mut self, other: &TokenUsage) {
        self.prompt_tokens += other.prompt_tokens;
        self.completion_tokens += other.completion_tokens;
        self.total_tokens += other.total_tokens;
    }
}

/// Stream chunk from LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub content: String,
    pub finish_reason: Option<String>,
}

/// LLM error types
#[derive(Debug, thiserror::Error)]
pub enum LLMError {
    #[error("Provider not configured")]
    NotConfigured,

    #[error("Invalid API key")]
    InvalidApiKey,

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Context length exceeded")]
    ContextLengthExceeded,

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Timeout")]
    Timeout,

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

impl From<reqwest::Error> for LLMError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            LLMError::Timeout
        } else {
            LLMError::NetworkError(err.to_string())
        }
    }
}

impl From<serde_json::Error> for LLMError {
    fn from(err: serde_json::Error) -> Self {
        LLMError::SerializationError(err.to_string())
    }
}

/// Renders a tool-call's arguments as the JSON string expected on the
/// OpenAI wire (`function.arguments`).
fn wire_arguments(arguments: &serde_json::Value) -> String {
    serde_json::to_string(arguments).unwrap_or_else(|_| "null".to_string())
}
