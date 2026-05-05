use scraper::{Html, Selector};

pub fn extract_from_html(html: &str) -> String {
    let document = Html::parse_document(html);
    let mut result = String::new();

    // Grok / x.ai selectors
    let selectors = [
        "div.message-content",
        "div[class*=\"grok-message\"]",
        "div[class*=\"message\"]",
        "div.prose",
        "div.markdown-body",
    ];

    for sel_str in &selectors {
        if let Ok(selector) = Selector::parse(sel_str) {
            for element in document.select(&selector) {
                let text = element.text().collect::<Vec<_>>().join("\n");
                if !text.trim().is_empty() {
                    result.push_str(text.trim());
                    result.push('\n');
                }
            }
        }
    }

    // Extract pre.CodeBlock specifically (Grok uses this class)
    if let Ok(pre_sel) = Selector::parse("pre.CodeBlock, pre.code-block") {
        for element in document.select(&pre_sel) {
            let text = element.text().collect::<Vec<_>>().join("\n");
            if !text.trim().is_empty() {
                result.push_str(&format!("```\n{}\n```\n\n", text.trim()));
            }
        }
    }

    // Extract code blocks with language
    if let Ok(code_sel) = Selector::parse("code[class*=\"language\"]") {
        for element in document.select(&code_sel) {
            let text = element.text().collect::<Vec<_>>().join("\n");
            if !text.trim().is_empty() {
                if let Some(class_attr) = element.value().attr("class") {
                    let lang = class_attr
                        .split_whitespace()
                        .filter(|c| c.starts_with("language-"))
                        .next()
                        .unwrap_or("")
                        .replace("language-", "");
                    result.push_str(&format!("```{}\n{}\n```\n\n", lang, text.trim()));
                }
            }
        }
    }

    // Generic pre > code fallback
    if let Ok(code_sel) = Selector::parse("pre code") {
        for element in document.select(&code_sel) {
            let text = element.text().collect::<Vec<_>>().join("\n");
            if !text.trim().is_empty() {
                result.push_str(&format!("```\n{}\n```\n\n", text.trim()));
            }
        }
    }

    result
}
