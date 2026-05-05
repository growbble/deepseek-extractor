use scraper::{Html, Selector};

pub fn extract_from_html(html: &str) -> String {
    let document = Html::parse_document(html);
    let mut result = String::new();

    // Universal fallback: try multiple strategies in order

    // Strategy 1: Get all text content from common content containers
    let content_selectors = [
        "article",
        "main",
        "div.content",
        "div.post-content",
        "div.entry-content",
        "div[class*=\"content\"]",
        "div[class*=\"message\"]",
        "div[class*=\"response\"]",
        "div[class*=\"conversation\"]",
        "div.prose",
        "div.markdown-body",
        "div.markdown",
        "body",
    ];

    for sel_str in &content_selectors {
        if let Ok(selector) = Selector::parse(sel_str) {
            let mut found_text = false;
            for element in document.select(&selector) {
                let text = element.text().collect::<Vec<_>>().join("\n");
                if !text.trim().is_empty() {
                    result.push_str(text.trim());
                    result.push('\n');
                    found_text = true;
                }
            }
            if found_text && !result.trim().is_empty() {
                break;
            }
        }
    }

    // Strategy 2: Find all pre > code blocks (most reliable for code extraction)
    let mut code_blocks = String::new();
    if let Ok(code_sel) = Selector::parse("pre code") {
        for element in document.select(&code_sel) {
            let text = element.text().collect::<Vec<_>>().join("\n");
            if !text.trim().is_empty() {
                let lang = element.value()
                    .attr("class")
                    .map(|c| {
                        c.split_whitespace()
                            .filter(|s| s.starts_with("language-"))
                            .next()
                            .unwrap_or("")
                            .replace("language-", "")
                    })
                    .unwrap_or_default();

                if !lang.is_empty() {
                    code_blocks.push_str(&format!("```{}\n{}\n```\n\n", lang, text.trim()));
                } else {
                    code_blocks.push_str(&format!("```\n{}\n```\n\n", text.trim()));
                }
            }
        }
    }

    // Combine: if we have code blocks, use them. Otherwise use full text.
    if !code_blocks.trim().is_empty() {
        result = code_blocks;
    }

    result
}
