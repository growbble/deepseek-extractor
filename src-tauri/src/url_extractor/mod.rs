pub(crate) mod deepseek;
pub(crate) mod chatgpt;
pub(crate) mod claude;
pub(crate) mod grok;
pub(crate) mod fallback;

use crate::models::{ExtractResult, LearningModel};
use crate::extractor::extract_from_text;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AiPlatform {
    DeepSeek,
    ChatGPT,
    Claude,
    Grok,
    ZAI,
    Unknown,
}

pub(crate) fn detect_platform(url: &str) -> AiPlatform {
    let url_lower = url.to_lowercase();
    if url_lower.contains("chat.deepseek.com") {
        AiPlatform::DeepSeek
    } else if url_lower.contains("chatgpt.com") || url_lower.contains("chat.openai.com") {
        AiPlatform::ChatGPT
    } else if url_lower.contains("claude.ai") {
        AiPlatform::Claude
    } else if url_lower.contains("x.ai") || url_lower.contains("grok.com") {
        AiPlatform::Grok
    } else if url_lower.contains("z.ai") {
        AiPlatform::ZAI
    } else {
        AiPlatform::Unknown
    }
}

/// Maximum response body size (10 MB)
const MAX_BODY_SIZE: u64 = 10 * 1024 * 1024;

async fn fetch_body(client: &reqwest::Client, url: &str) -> Result<String, String> {
    let response = client.get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch URL: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server returned HTTP {}", response.status()));
    }

    // Stream body chunk-by-chunk with size limit to prevent memory exhaustion
    let mut body = Vec::new();
    let mut stream = response.bytes_stream();
    use futures_util::StreamExt;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Read error: {}", e))?;
        if body.len().saturating_add(chunk.len()) as u64 > MAX_BODY_SIZE {
            return Err(format!("Response too large (>{} MB)", MAX_BODY_SIZE / 1024 / 1024));
        }
        body.extend_from_slice(&chunk);
    }

    String::from_utf8(body).map_err(|_| "Response is not valid UTF-8".to_string())
}

pub(crate) async fn fetch_and_extract(url: &str, model: &LearningModel) -> Result<ExtractResult, String> {
    let platform = detect_platform(url);

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
        .timeout(std::time::Duration::from_secs(25))
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let html = fetch_body(&client, url).await?;

    let text_content = match platform {
        AiPlatform::DeepSeek => deepseek::extract_from_html(&html),
        AiPlatform::ChatGPT => chatgpt::extract_from_html(&html),
        AiPlatform::Claude => claude::extract_from_html(&html),
        AiPlatform::Grok => grok::extract_from_html(&html),
        AiPlatform::ZAI | AiPlatform::Unknown => fallback::extract_from_html(&html),
    };

    let platform_name = format!("url:{:?}", platform).to_lowercase();
    let mut result = extract_from_text(&text_content, &platform_name, model);

    if result.files.is_empty() && !text_content.trim().is_empty() {
        result = crate::extractor::extract(text_content.trim(), model);
        result.source = platform_name;
    }

    Ok(result)
}
