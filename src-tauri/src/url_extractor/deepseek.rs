use scraper::{Html, Selector};

pub fn extract_from_html(html: &str) -> String {
    let document = Html::parse_document(html);
    let mut result = String::new();

    // Try markdown-body divs (DeepSeek's main content containers)
    let selectors = [
        "div.markdown-body",
        "div[class*=\"markdown\"]",
        "div.markdown-content",
        "div[class*=\"message\"]",
        "pre code[class*=\"language\"]",
    ];

    for sel_str in &selectors {
        if let Ok(selector) = Selector::parse(sel_str) {
            for element in document.select(&selector) {
                let text = element.text().collect::<Vec<_>>().join("\n");
                if !text.trim().is_empty() {
                    // Check if this is a code block with language class
                    if let Some(class_attr) = element.value().attr("class") {
                        if class_attr.contains("language-") {
                            let lang = class_attr
                                .split_whitespace()
                                .filter(|c| c.starts_with("language-"))
                                .next()
                                .unwrap_or("")
                                .replace("language-", "");
                            result.push_str(&format!("```{}\n", lang));
                        }
                    }
                    result.push_str(text.trim());
                    if sel_str.contains("pre") {
                        result.push_str("\n```\n");
                    } else {
                        result.push('\n');
                    }
                }
            }
        }
    }

    // Also try extracting just pre > code blocks if nothing found
    if result.trim().is_empty() {
        if let Ok(pre_sel) = Selector::parse("pre code") {
            for element in document.select(&pre_sel) {
                let text = element.text().collect::<Vec<_>>().join("\n");
                if !text.trim().is_empty() {
                    if let Some(class_attr) = element.value().attr("class") {
                        if class_attr.contains("language-") {
                            let lang = class_attr
                                .split_whitespace()
                                .filter(|c| c.starts_with("language-"))
                                .next()
                                .unwrap_or("")
                                .replace("language-", "");
                            result.push_str(&format!("```{}\n{}\n```\n\n", lang, text.trim()));
                        } else {
                            result.push_str(&format!("```\n{}\n```\n\n", text.trim()));
                        }
                    } else {
                        result.push_str(&format!("```\n{}\n```\n\n", text.trim()));
                    }
                }
            }
        }
    }

    result
}
