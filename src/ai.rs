use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiRequest {
    pub provider: String,
    pub model: String,
    pub system: String,
    pub user: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiResponse {
    pub content: String,
}

#[derive(Debug, Clone)]
pub enum AiError {
    MissingApiKey(&'static str),
    Http(String),
    InvalidResponse(String),
}

impl std::fmt::Display for AiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AiError::MissingApiKey(name) => write!(f, "Missing API key: {}", name),
            AiError::Http(msg) => write!(f, "HTTP error: {}", msg),
            AiError::InvalidResponse(msg) => write!(f, "Invalid response: {}", msg),
        }
    }
}

impl std::error::Error for AiError {}

pub async fn send_request(req: AiRequest) -> Result<AiResponse, AiError> {
    match req.provider.as_str() {
        "openai" => openai_request(req).await,
        "anthropic" => anthropic_request(req).await,
        "ollama" => ollama_request(req).await,
        _ => Err(AiError::InvalidResponse("Unknown provider".to_string())),
    }
}

async fn openai_request(req: AiRequest) -> Result<AiResponse, AiError> {
    let key = std::env::var("OPENAI_API_KEY").map_err(|_| AiError::MissingApiKey("OPENAI_API_KEY"))?;
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "model": req.model,
        "messages": [
            {"role": "system", "content": req.system},
            {"role": "user", "content": req.user},
        ],
        "temperature": 0.2
    });
    let res = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(key)
        .json(&body)
        .send()
        .await
        .map_err(|e| AiError::Http(e.to_string()))?;
    let status = res.status();
    let json: serde_json::Value = res.json().await.map_err(|e| AiError::Http(e.to_string()))?;
    if !status.is_success() {
        return Err(AiError::Http(json.to_string()));
    }
    let content = json
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| AiError::InvalidResponse("Missing content".to_string()))?
        .to_string();
    Ok(AiResponse { content })
}

async fn anthropic_request(req: AiRequest) -> Result<AiResponse, AiError> {
    let key = std::env::var("ANTHROPIC_API_KEY").map_err(|_| AiError::MissingApiKey("ANTHROPIC_API_KEY"))?;
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "model": req.model,
        "max_tokens": 512,
        "system": req.system,
        "messages": [
            {"role": "user", "content": req.user}
        ]
    });
    let res = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await
        .map_err(|e| AiError::Http(e.to_string()))?;
    let status = res.status();
    let json: serde_json::Value = res.json().await.map_err(|e| AiError::Http(e.to_string()))?;
    if !status.is_success() {
        return Err(AiError::Http(json.to_string()));
    }
    let content = json
        .get("content")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("text"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| AiError::InvalidResponse("Missing content".to_string()))?
        .to_string();
    Ok(AiResponse { content })
}

async fn ollama_request(req: AiRequest) -> Result<AiResponse, AiError> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "model": req.model,
        "prompt": format!("{}\n\n{}", req.system, req.user),
        "stream": false
    });
    let res = client
        .post("http://localhost:11434/api/generate")
        .json(&body)
        .send()
        .await
        .map_err(|e| AiError::Http(e.to_string()))?;
    let status = res.status();
    let json: serde_json::Value = res.json().await.map_err(|e| AiError::Http(e.to_string()))?;
    if !status.is_success() {
        return Err(AiError::Http(json.to_string()));
    }
    let content = json
        .get("response")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AiError::InvalidResponse("Missing response".to_string()))?
        .to_string();
    Ok(AiResponse { content })
}
