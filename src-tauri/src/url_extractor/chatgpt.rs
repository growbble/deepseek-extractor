use scraper::{Html, Selector};

pub(crate) fn extract_from_html(html: &str) -> String {
    let document = Html::parse_document(html);
    let mut result = String::new();

    // Try ChatGPT share page selectors
    let selectors = [
        "article",
        "div[data-message-content]",
        "div[class*=\"markdown\"]",
        "div[class*=\"message-content\"]",
        "div.prose",
        "pre code",
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

    // Extract code blocks with language info
    if let Ok(code_sel) = Selector::parse("pre code[class*=\"language\"]") {
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
                    result.push_str(&format!("\n```{}\n{}\n```\n", lang, text.trim()));
                }
            }
        }
    }

    // Extract any pre > code blocks
    if let Ok(code_sel) = Selector::parse("pre code") {
        for element in document.select(&code_sel) {
            let text = element.text().collect::<Vec<_>>().join("\n");
            if !text.trim().is_empty() {
                result.push_str(&format!("\n```\n{}\n```\n", text.trim()));
            }
        }
    }

    result
}
