use syntect::highlighting::ThemeSet;
use syntect::html::styled_line_to_highlighted_html;
use syntect::parsing::SyntaxSet;
use syntect::easy::HighlightLines;

pub fn highlight_code(code: &str, language: &str) -> Result<String, String> {
    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    let theme = ts.themes.get("base16-ocean.dark")
        .or_else(|| ts.themes.get("InspiredGitHub"))
        .ok_or_else(|| "No theme found".to_string())?;

    let syntax = if language.is_empty() || language == "text" || language == "plain" {
        ss.find_syntax_plain_text()
    } else {
        ss.find_syntax_by_token(language)
            .or_else(|| ss.find_syntax_by_extension(language))
            .unwrap_or_else(|| ss.find_syntax_plain_text())
    };

    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut html_output = String::new();

    html_output.push_str(
        "<pre style=\"background:#0a0e1a; color:#d4d4d4; padding:16px; \
         border-radius:8px; overflow-x:auto; font-family:'JetBrains Mono','Fira Code',monospace; \
         font-size:13px; line-height:1.5;\"><code>"
    );

    for line in code.lines() {
        match highlighter.highlight_line(line, &ss) {
            Ok(regions) => {
                let html = styled_line_to_highlighted_html(&regions, syntect::html::IncludeBackground::No);
                html_output.push_str(&html);
            }
            Err(_) => {
                // Fallback: escape the line as plain HTML
                let escaped = line
                    .replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;")
                    .replace('"', "&quot;");
                html_output.push_str(&escaped);
                html_output.push('\n');
            }
        }
    }

    html_output.push_str("</code></pre>");

    Ok(html_output)
}
