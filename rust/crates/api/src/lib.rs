use async_trait::async_trait;
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use reqwest::Client;
use std::pin::Pin;
use futures_core::Stream;
use futures_util::StreamExt;

// --- runtime 크레이트의 타입을 재수출하여 타입 호환성 확보 ---
// Cargo.toml에 runtime = { path = "../runtime" }이 선언되어 있어야 합니다.
pub use runtime::config::OAuthConfig;
pub use runtime::oauth::OAuthTokenExchangeRequest;

// CLI가 기대하는 필드명과 타입(u64)을 정확히 일치시킨 구조체
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSet {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<u64>,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum AuthSource {
    None,
    ApiKey(String),
}

// CLI의 초기화 클로저와 타입을 맞추기 위한 함수
pub fn resolve_startup_auth_source<F>(_f: F) -> Result<AuthSource> 
where 
    F: FnOnce() -> Result<Option<OAuthConfig>> 
{
    Ok(AuthSource::None)
}

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Auth error: {0}")]
    Auth(String),
    #[error("IO error: {0}")]
    Io(String),
}

impl From<std::io::Error> for ApiError {
    fn from(err: std::io::Error) -> Self {
        ApiError::Io(err.to_string())
    }
}

// --- Anthropic API 데이터 구조체 정의 ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRequest {
    pub model: String,
    pub messages: Vec<InputMessage>,
    pub max_tokens: u32,
    pub system: Option<String>,
    pub tools: Option<Vec<ToolDefinition>>,
    pub tool_choice: Option<ToolChoice>,
    pub stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputMessage {
    pub role: String,
    pub content: Vec<InputContentBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputContentBlock {
    Text { text: String },
    ToolUse { id: String, name: String, input: serde_json::Value },
    ToolResult { 
        tool_use_id: String, 
        content: Vec<ToolResultContentBlock>,
        #[serde(default)]
        is_error: bool 
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolResultContentBlock {
    Text { text: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolChoice {
    Auto,
    Any,
    Tool { name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageResponse {
    pub content: Vec<OutputContentBlock>,
    pub usage: Usage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputContentBlock {
    Text { text: String },
    ToolUse { id: String, name: String, input: serde_json::Value },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_creation_input_tokens: u32,
    pub cache_read_input_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    MessageStart(MessageStartEvent),
    ContentBlockStart(ContentBlockStartEvent),
    ContentBlockDelta(ContentBlockDeltaEvent),
    ContentBlockStop(ContentBlockStopEvent),
    MessageDelta(MessageDeltaEvent),
    MessageStop(MessageStopEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageStartEvent { pub message: MessageResponse }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentBlockStartEvent { pub index: usize, pub content_block: OutputContentBlock }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentBlockDeltaEvent { pub index: usize, pub delta: ContentBlockDelta }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentBlockStopEvent { pub index: usize }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDeltaEvent { pub usage: Usage }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageStopEvent {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlockDelta {
    TextDelta { text: String },
    InputJsonDelta { partial_json: String },
}

// --- 스트림 처리를 위한 확장 트레이트 ---

#[async_trait]
pub trait StreamExtPad {
    async fn next_event(&mut self) -> Result<Option<StreamEvent>>;
}

#[async_trait]
impl<S> StreamExtPad for Pin<Box<S>> 
where 
    S: Stream<Item = Result<StreamEvent>> + Send + ?Sized 
{
    async fn next_event(&mut self) -> Result<Option<StreamEvent>> {
        Ok(self.next().await.transpose()?)
    }
}

// --- AnthropicClient 구현 (Ollama 백엔드 연결) ---

pub fn read_base_url() -> String {
    std::env::var("ANTHROPIC_BASE_URL").unwrap_or_else(|_| "http://localhost:11434/v1".to_string())
}

pub struct AnthropicClient {
    pub client: Client,
    pub base_url: String,
}

impl AnthropicClient {
    pub fn new(_api_key: &str, base_url: String) -> Self {
        Self { client: Client::new(), base_url }
    }

    pub fn from_auth(_auth: AuthSource) -> Self {
        Self::new("ollama", read_base_url())
    }

    pub fn from_env() -> Result<Self> {
        Ok(Self::new("ollama", read_base_url()))
    }

    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }

    // CLI의 기대에 맞춰 인자를 참조형(&)으로 정의
    pub async fn exchange_oauth_code(
        &self, 
        _oauth: &OAuthConfig, 
        _req: &OAuthTokenExchangeRequest
    ) -> Result<TokenSet> {
        bail!("OAuth not supported for local Ollama server");
    }

    pub async fn stream_message(&self, req: &MessageRequest)
        -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>>>
    {
        let endpoint = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));

        let mut prompt_messages = Vec::new();
        if let Some(system) = &req.system {
            prompt_messages.push(serde_json::json!({ "role": "system", "content": system }));
        }
        for m in &req.messages {
            let mut text_content = String::new();
            for c in &m.content {
                if let InputContentBlock::Text { text } = c {
                    text_content.push_str(text);
                }
            }
            prompt_messages.push(serde_json::json!({ "role": m.role, "content": text_content }));
        }

        // Ollama 에 non-streaming 으로 요청 (응답 전체를 받은 뒤 가짜 스트림으로 변환)
        let response = self.client.post(&endpoint)
            .json(&serde_json::json!({
                "model": req.model,
                "messages": prompt_messages,
                "stream": false
            }))
            .send()
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        let json: serde_json::Value = response.json().await.map_err(|e| anyhow::anyhow!(e))?;

        let message = &json["choices"][0]["message"];
        let raw_content = message["content"].as_str().unwrap_or("").to_string();
        let reasoning = message["reasoning"].as_str()
            .or_else(|| message["reasoning_content"].as_str())
            .unwrap_or("")
            .to_string();

        // thinking 모델에서 content 가 비어있으면 reasoning 을 답변으로 사용
        let content = if raw_content.trim().is_empty() && !reasoning.trim().is_empty() {
            reasoning
        } else {
            raw_content
        };

        let events: Vec<Result<StreamEvent>> = vec![
            Ok(StreamEvent::MessageStart(MessageStartEvent {
                message: MessageResponse { content: vec![], usage: Usage::default() },
            })),
            Ok(StreamEvent::ContentBlockStart(ContentBlockStartEvent {
                index: 0,
                content_block: OutputContentBlock::Text { text: String::new() },
            })),
            Ok(StreamEvent::ContentBlockDelta(ContentBlockDeltaEvent {
                index: 0,
                delta: ContentBlockDelta::TextDelta { text: content },
            })),
            Ok(StreamEvent::ContentBlockStop(ContentBlockStopEvent { index: 0 })),
            Ok(StreamEvent::MessageDelta(MessageDeltaEvent { usage: Usage::default() })),
            Ok(StreamEvent::MessageStop(MessageStopEvent {})),
        ];

        Ok(Box::pin(futures_util::stream::iter(events)))
    }

    pub async fn send_message(&self, req: &MessageRequest) -> Result<MessageResponse> {
        let endpoint = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        
        // 간단한 텍스트 변환 (Ollama/OpenAI 포맷 대응)
        let mut prompt_messages = Vec::new();
        for m in &req.messages {
            let mut text_content = String::new();
            for c in &m.content {
                if let InputContentBlock::Text { text } = c {
                    text_content.push_str(text);
                }
            }
            prompt_messages.push(serde_json::json!({
                "role": m.role,
                "content": text_content
            }));
        }

        let response = self.client.post(&endpoint)
            .json(&serde_json::json!({
                "model": req.model,
                "messages": prompt_messages,
                "stream": false
            }))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        // 응답 파싱 및 MessageResponse 구조체로 변환
        let content_text = response["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(MessageResponse { 
            content: vec![OutputContentBlock::Text { text: content_text }], 
            usage: Usage::default() 
        })
    }

    pub async fn create_message(&self, req: &MessageRequest) -> Result<MessageResponse> {
        self.send_message(req).await
    }
}