use std::sync::LazyLock;

use comrak::{
    markdown_to_html_with_plugins,
    options::{Options, Plugins},
    plugins::syntect::SyntectAdapterBuilder,
};
use syntect::{
    easy::HighlightLines,
    highlighting::ThemeSet,
    html::{IncludeBackground, styled_line_to_highlighted_html},
    parsing::SyntaxSet,
    util::LinesWithEndings,
};

use crate::error::{RendererError, Result};

pub(crate) const DEFAULT_SYNTAX_THEME: &str = "base16-ocean.dark";

static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static THEME_SET: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FormattingConfig {
    syntax_theme: String,
}

impl FormattingConfig {
    pub(crate) fn new(theme: Option<&str>) -> Result<Self> {
        let syntax_theme = match theme {
            Some(theme) => {
                let trimmed = theme.trim();
                if trimmed.is_empty() {
                    return Err(RendererError::invalid_request(
                        "request.syntax_theme cannot be empty",
                    ));
                }
                trimmed.to_string()
            }
            None => DEFAULT_SYNTAX_THEME.to_string(),
        };

        if !THEME_SET.themes.contains_key(&syntax_theme) {
            return Err(RendererError::invalid_request(format!(
                "request.syntax_theme `{syntax_theme}` is not a supported built-in syntect theme (supported: {})",
                supported_theme_names().join(", "),
            )));
        }

        Ok(Self { syntax_theme })
    }

    pub(crate) fn syntax_theme(&self) -> &str {
        &self.syntax_theme
    }
}

pub(crate) fn render_markdown_to_html(
    markdown: &str,
    formatting: &FormattingConfig,
) -> Result<String> {
    let mut options = Options::default();
    options.extension.table = true;
    options.extension.strikethrough = true;
    options.extension.tasklist = true;
    options.extension.footnotes = true;
    options.render.r#unsafe = true;

    let adapter = SyntectAdapterBuilder::new()
        .theme(formatting.syntax_theme())
        .build();
    let mut plugins = Plugins::default();
    plugins.render.codefence_syntax_highlighter = Some(&adapter);

    Ok(markdown_to_html_with_plugins(markdown, &options, &plugins))
}

pub(crate) fn highlight_code(
    code: &str,
    lang: &str,
    formatting: &FormattingConfig,
) -> Result<String> {
    let syntax = SYNTAX_SET
        .find_syntax_by_token(lang)
        .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());
    let theme = &THEME_SET.themes[formatting.syntax_theme()];
    let mut highlighter = HighlightLines::new(syntax, theme);

    let mut html = String::from("<pre><code>");
    for line in LinesWithEndings::from(code) {
        let ranges = highlighter
            .highlight_line(line, &SYNTAX_SET)
            .map_err(|error| RendererError::Template(error.to_string()))?;
        let highlighted = styled_line_to_highlighted_html(&ranges, IncludeBackground::No)
            .map_err(|error| RendererError::Template(error.to_string()))?;
        html.push_str(&highlighted);
    }
    html.push_str("</code></pre>");

    Ok(html)
}

fn supported_theme_names() -> Vec<String> {
    let mut names = THEME_SET.themes.keys().cloned().collect::<Vec<_>>();
    names.sort();
    names
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_SYNTAX_THEME, FormattingConfig, highlight_code, render_markdown_to_html};

    #[test]
    fn rejects_unknown_syntax_theme() {
        let error = FormattingConfig::new(Some("missing-theme")).expect_err("reject theme");
        assert!(
            error
                .to_string()
                .contains("supported built-in syntect theme")
        );
    }

    #[test]
    fn renders_markdown_with_syntect_code_blocks() {
        let html = render_markdown_to_html(
            "```rust\nlet x = 1;\n```",
            &FormattingConfig::new(None).expect("default formatting"),
        )
        .expect("render markdown");

        assert!(html.contains("<pre"));
        assert!(html.contains("let"));
        assert!(html.contains("x"));
    }

    #[test]
    fn highlights_code_with_default_theme() {
        let html = highlight_code(
            "let x = 1;",
            "rust",
            &FormattingConfig::new(Some(DEFAULT_SYNTAX_THEME)).expect("theme"),
        )
        .expect("highlight code");

        assert!(html.starts_with("<pre><code>"));
        assert!(html.ends_with("</code></pre>"));
        assert!(html.contains("let"));
    }
}
