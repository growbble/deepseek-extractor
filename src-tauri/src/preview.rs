use syntect::highlighting::ThemeSet;
use syntect::html::styled_line_to_highlighted_html;
use syntect::parsing::SyntaxSet;
use syntect::easy::HighlightLines;

/// Maximum code size to syntax-highlight (1 MB — beyond this, return plain text).
const MAX_HIGHLIGHT_SIZE: usize = 1_048_576;

/// Highlight code using syntect, returning safe HTML.
/// The output is wrapped in <pre><code> with inline styles.
/// Syntect only emits styled spans and we escape all fallback text,
/// so the output is XSS-safe.
pub(crate) fn highlight_code(code: &str, language: &str) -> Result<String, String> {
    // If code is too large, return plain text (no highlighting)
    if code.len() > MAX_HIGHLIGHT_SIZE {
        return Ok(format!(
            "<pre style=\"background:#0a0e1a; color:#d4d4d4; padding:16px; \
             border-radius:8px; overflow-x:auto; font-family:monospace; font-size:13px; \
             line-height:1.5;\"><code>{}<br><em style=\"color:#888;\">... \
             (truncated at 1 MB, syntax highlighting disabled for performance)</em></code></pre>",
            html_escape(&code[..code.len().min(500)])
        ));
    }

    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    // Try dark theme, fallback to InspiredGitHub, then any available theme
    let theme = ts.themes.get("base16-ocean.dark")
        .or_else(|| ts.themes.get("InspiredGitHub"))
        .or_else(|| ts.themes.iter().next().map(|(_, t)| t))
        .ok_or_else(|| "No syntax theme available".to_string())?;

    let syntax = if language.is_empty() || language == "text" || language == "plain" {
        ss.find_syntax_plain_text()
    } else {
        ss.find_syntax_by_token(language)
            .or_else(|| ss.find_syntax_by_extension(language))
            .unwrap_or_else(|| ss.find_syntax_plain_text())
    };

    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut html_output = String::new();
    let mut line_number: u32 = 1;

    html_output.push_str(
        "<pre style=\"background:#0a0e1a; color:#d4d4d4; padding:16px; \
         border-radius:8px; overflow-x:auto; font-family:'JetBrains Mono','Fira Code',monospace; \
         font-size:13px; line-height:1.5;\"><code>"
    );

    for line in code.lines() {
        match highlighter.highlight_line(line, &ss) {
            Ok(regions) => {
                let html = styled_line_to_highlighted_html(&regions, syntect::html::IncludeBackground::No);
                match html {
                    Ok(html_str) => {
                        html_output.push_str(&format!(
                            "<span style=\"color:#4a5568; user-select:none; margin-right:1em;\">{:>4}</span>{}",
                            line_number, html_str
                        ));
                        html_output.push('\n');
                    }
                    Err(_) => {
                        // Fallback: escape as plain text
                        html_output.push_str(&format!(
                            "<span style=\"color:#4a5568; user-select:none; margin-right:1em;\">{:>4}</span>{}",
                            line_number, html_escape(line)
                        ));
                        html_output.push('\n');
                    }
                }
            }
            Err(_) => {
                html_output.push_str(&format!(
                    "<span style=\"color:#4a5568; user-select:none; margin-right:1em;\">{:>4}</span>{}",
                    line_number, html_escape(line)
                ));
                html_output.push('\n');
            }
        }
        line_number = line_number.saturating_add(1);
    }

    html_output.push_str("</code></pre>");

    Ok(html_output)
}

/// Simple HTML entity escaping to prevent XSS via code content
fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}
