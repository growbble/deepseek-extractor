use scraper::{Html, Selector};

pub fn extract_from_html(html: &str) -> String {
    let document = Html::parse_document(html);
    let mut result = String::new();

    // Claude share page selectors
    let selectors = [
        "div.font-claude-message",
        "div[class*=\"message\"]",
        "div[class*=\"conversation\"]",
        "div.prose",
        "div.markdown",
    ];

    for sel_str in &selectors {
        if let Ok(selector) = Selector::parse(sel_str) {
            for element in document.select(&selector) {
                // Extract text content
                let text = element.text().collect::<Vec<_>>().join(" ");
                if !text.trim().is_empty() {
                    result.push_str(text.trim());

                    // Extract code blocks within this message
                    if let Ok(code_sel) = Selector::parse("pre code[class*=\"language\"]") {
                        for code_elem in element.select(&code_sel) {
                            let code_text = code_elem.text().collect::<Vec<_>>().join("\n");
                            if !code_text.trim().is_empty() {
                                if let Some(class_attr) = code_elem.value().attr("class") {
                                    let lang = class_attr
                                        .split_whitespace()
                                        .filter(|c| c.starts_with("language-"))
                                        .next()
                                        .unwrap_or("")
                                        .replace("language-", "");
                                    result.push_str(&format!("\n```{}\n{}\n```\n", lang, code_text.trim()));
                                }
                            }
                        }
                    }

                    // Also get generic pre > code
                    if let Ok(code_sel) = Selector::parse("pre code") {
                        for code_elem in element.select(&code_sel) {
                            let code_text = code_elem.text().collect::<Vec<_>>().join("\n");
                            if !code_text.trim().is_empty() {
                                result.push_str(&format!("\n```\n{}\n```\n", code_text.trim()));
                            }
                        }
                    }

                    result.push('\n');
                }
            }
        }
    }

    // Fallback: try to get any code blocks from the whole page
    if result.trim().is_empty() {
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
                    result.push_str(&format!("```{}\n{}\n```\n\n", lang, text.trim()));
                }
            }
        }
    }

    result
}
