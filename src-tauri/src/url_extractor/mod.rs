pub mod deepseek;
pub mod chatgpt;
pub mod claude;
pub mod grok;
pub mod fallback;

use crate::extractor::learning::LearningModel;
use crate::models::ExtractResult;
use crate::extractor::extract_from_text;

#[derive(Debug, Clone, PartialEq)]
pub enum AiPlatform {
    DeepSeek,
    ChatGPT,
    Claude,
    Grok,
    ZAI,
    Unknown,
}

pub fn detect_platform(url: &str) -> AiPlatform {
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

pub async fn fetch_and_extract(url: &str, model: &LearningModel) -> Result<ExtractResult, String> {
    let platform = detect_platform(url);

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client.get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch URL: {}", e))?;

    let html = response.text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

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
        // Try fallback: extract all code blocks directly via regex
        result = crate::extractor::extract(text_content.trim(), model);
        result.source = platform_name;
    }
    Ok(result)
}
