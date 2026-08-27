mod convert;
mod emoji;
mod metadata;
mod options;
mod parser;
mod resource;

use std::{fmt, path::PathBuf};

use takumi::prelude::{ImageSourceInput, Node, NodeKind, StyleSheet};

pub use emoji::EmojiType;
pub use options::{FromHtmlOptions, LocalAssetMode};

pub type Result<T> = std::result::Result<T, HtmlError>;

#[derive(Debug, Clone)]
pub struct FromHtmlResult {
    pub node: Node,
    pub stylesheets: Vec<String>,
}

impl FromHtmlResult {
    /// Parses the extracted stylesheet sources using Takumi's lossy stylesheet parser.
    pub fn stylesheet(&self) -> StyleSheet {
        StyleSheet::parse_list_loosy(&self.stylesheets)
    }

    /// Returns all resource URLs referenced by this result, combining node-level
    /// resources (image `src`, inline style `url(...)`) with stylesheet-level
    /// resources (`url(...)` in extracted `<style>` and linked CSS).
    pub fn resource_urls(&self) -> impl Iterator<Item = &str> {
        let mut image_urls = Vec::new();
        collect_image_src_urls(&self.node, &mut image_urls);

        image_urls
            .into_iter()
            .chain(self.stylesheets.iter().flat_map(|s| css_resource_urls(s)))
    }
}

fn collect_image_src_urls<'a>(node: &'a Node, urls: &mut Vec<&'a str>) {
    if let NodeKind::Image(image) = &node.kind
        && let ImageSourceInput::Url(src) = &image.src
    {
        urls.push(src.as_ref());
    }
    if let NodeKind::Container { children } = &node.kind {
        for child in children {
            collect_image_src_urls(child, urls);
        }
    }
}

fn css_resource_urls(stylesheet: &str) -> impl Iterator<Item = &str> {
    let mut cursor = 0;
    std::iter::from_fn(move || {
        while cursor < stylesheet.len() {
            if let Some((token, end_index)) = resource::parse_css_url(stylesheet, cursor) {
                cursor = end_index;
                return Some(token);
            }
            let ch = stylesheet[cursor..]
                .chars()
                .next()
                .expect("cursor always points to a valid character boundary");
            cursor += ch.len_utf8();
        }
        None
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HtmlError {
    MissingImageSource {
        tag_name: String,
    },
    MissingLinkedStylesheetHref {
        tag_name: String,
    },
    MissingBasePathForRelativeReference {
        reference: String,
        kind: String,
    },
    AssetReadFailed {
        path: PathBuf,
        reason: String,
    },
    InlineStyleParse {
        tag_name: String,
        value: String,
        reason: String,
    },
    StylesheetCompilationFailed {
        reason: String,
    },
}

impl fmt::Display for HtmlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingImageSource { tag_name } => {
                write!(f, "<{tag_name}> elements require a src attribute")
            }
            Self::MissingLinkedStylesheetHref { tag_name } => {
                write!(f, "<{tag_name}> stylesheet links require an href attribute")
            }
            Self::MissingBasePathForRelativeReference { reference, kind } => {
                write!(
                    f,
                    "cannot resolve relative {kind} reference `{reference}` without a base_path"
                )
            }
            Self::AssetReadFailed { path, reason } => {
                write!(f, "failed to read asset `{}` ({reason})", path.display())
            }
            Self::InlineStyleParse {
                tag_name,
                value,
                reason,
            } => {
                write!(
                    f,
                    "failed to parse inline style on <{tag_name}>: `{value}` ({reason})"
                )
            }
            Self::StylesheetCompilationFailed { reason } => {
                write!(f, "failed to compile stylesheet ({reason})")
            }
        }
    }
}

impl std::error::Error for HtmlError {}

/// Parse HTML using document semantics for `<html>` roots and fragment semantics
/// for all other markup.
pub fn from_html(markup: &str) -> Result<FromHtmlResult> {
    from_html_with_options(markup, &FromHtmlOptions::default())
}

/// Parse HTML with automatic document/fragment root detection.
pub fn from_html_with_options(markup: &str, options: &FromHtmlOptions) -> Result<FromHtmlResult> {
    if starts_with_html_element(markup) {
        from_document_with_options(markup, options)
    } else {
        from_fragment_with_options(markup, options)
    }
}

fn starts_with_html_element(source: &str) -> bool {
    let mut rest = source.trim_start();

    loop {
        let lowered = rest.get(..9).map(str::to_ascii_lowercase);
        if lowered
            .as_deref()
            .is_some_and(|prefix| prefix.starts_with("<!doctype"))
        {
            let Some((_, after)) = rest.split_once('>') else {
                return false;
            };
            rest = after.trim_start();
            continue;
        }

        if rest.starts_with("<!--") {
            let Some((_, after)) = rest.split_once("-->") else {
                return false;
            };
            rest = after.trim_start();
            continue;
        }

        let Some((prefix, after)) = rest.split_at_checked("<html".len()) else {
            return false;
        };
        return prefix.eq_ignore_ascii_case("<html")
            && (after.starts_with(['>', '/']) || after.starts_with(char::is_whitespace));
    }
}

/// Parse a complete HTML document and convert it into a Takumi node tree.
pub fn from_document(markup: &str) -> Result<FromHtmlResult> {
    from_document_with_options(markup, &FromHtmlOptions::default())
}

/// Parse a complete HTML document and convert it with custom options.
pub fn from_document_with_options(
    markup: &str,
    options: &FromHtmlOptions,
) -> Result<FromHtmlResult> {
    let parsed = parser::parse_document(markup);
    convert::convert(parsed, options)
}

/// Parse an HTML fragment and convert it into a Takumi node tree.
pub fn from_fragment(markup: &str) -> Result<FromHtmlResult> {
    from_fragment_with_options(markup, &FromHtmlOptions::default())
}

/// Parse an HTML fragment and convert it with custom options.
pub fn from_fragment_with_options(
    markup: &str,
    options: &FromHtmlOptions,
) -> Result<FromHtmlResult> {
    let parsed = parser::parse_fragment(markup);
    convert::convert(parsed, options)
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use takumi::prelude::Node;
    use tempfile::TempDir;

    use super::{
        EmojiType, FromHtmlOptions, FromHtmlResult, HtmlError, LocalAssetMode, from_document,
        from_document_with_options, from_fragment, from_html,
    };

    #[test]
    fn converts_plain_text() {
        let result = from_html("Hello, world!").unwrap();

        assert!(result.stylesheets.is_empty());
        assert_eq!(result.node.to_html(), "<span>Hello, world!</span>");
    }

    #[test]
    fn converts_nested_elements() {
        let result = from_html("<div><span>Nested</span></div>").unwrap();

        assert_eq!(
            result.node.to_html(),
            "<div style=\"display: block;\"><span>Nested</span></div>"
        );
    }

    #[test]
    fn applies_default_element_styles() {
        let result = from_html("<h1>Title</h1>").unwrap();
        let rendered = result.node.to_html();

        assert!(rendered.contains("font-size: 2em;"));
        assert!(rendered.contains("margin-top: 0.67em;"));
        assert!(rendered.contains("font-weight: 700;"));
        assert!(rendered.contains("display: block;"));
    }

    #[test]
    fn parses_document_root_from_general_entry_point() {
        let result =
            from_html("<!doctype html><!-- comment --><html><body><div>Ready</div></body></html>")
                .unwrap();

        assert!(result.node.to_html().starts_with("<html"));
        assert!(result.node.to_html().contains("<body"));
    }

    #[test]
    fn parses_lang_attribute() {
        let result = from_html(r#"<p lang="zh-Hant">你好</p>"#).unwrap();
        let rendered = result.node.to_html();

        assert_eq!(rendered.matches("lang=").count(), 1);
        assert!(rendered.contains(r#"lang="zh-Hant""#));
    }

    #[test]
    fn ignores_unreadable_inline_declarations_but_keeps_valid_ones() {
        let result =
            from_html(r#"<div style="font-size:64px;width:fit-content;color:red">Ready</div>"#)
                .unwrap();
        let rendered = result.node.to_html();

        assert!(rendered.contains("font-size: 64px;"));
        assert!(rendered.contains("color: rgb(255, 0, 0);"));
        assert!(!rendered.contains("fit-content"));
    }

    #[test]
    fn converts_images() {
        let result =
            from_html(r#"<img src="https://example.com/example.png" width="64" height="32">"#)
                .unwrap();

        assert_eq!(
            result.resource_urls().collect::<Vec<_>>(),
            vec!["https://example.com/example.png"]
        );
        assert!(
            result
                .node
                .to_html()
                .contains(r#"src="https://example.com/example.png""#)
        );
    }

    #[test]
    fn finds_css_resource_urls() {
        let result = FromHtmlResult {
            node: Node::container([]),
            stylesheets: vec![
                r#".card { background: URL( "./图片.png" ); mask: url(icon.svg) }"#.to_string(),
            ],
        };

        assert_eq!(
            result.resource_urls().collect::<Vec<_>>(),
            vec!["./图片.png", "icon.svg"]
        );
    }

    #[test]
    fn extracts_stylesheets() {
        let result = from_html("<style>body { color: red; }</style><div>Ready</div>").unwrap();

        assert!(result.stylesheets[0].contains("body"));
        assert!(result.stylesheets[0].contains("color: red"));
        assert_eq!(
            result.node.to_html(),
            "<div style=\"display: block;\">Ready</div>"
        );
    }

    #[test]
    fn converts_line_breaks() {
        let result = from_html("<div>foo<br>bar</div>").unwrap();
        let rendered = result.node.to_html();

        assert!(rendered.contains("foo"));
        assert!(rendered.contains("bar"));
        assert!(rendered.contains('\n'));
    }

    #[test]
    fn wraps_multiple_root_nodes() {
        let result = from_fragment("<span>foo</span><span>bar</span>").unwrap();

        assert_eq!(
            result.node.to_html(),
            "<div style=\"display: block; width: 100%; height: 100%;\"><span>foo</span><span>bar</span></div>"
        );
    }

    #[test]
    fn converts_full_documents() {
        let result = from_document(
			r#"<!doctype html><html><head><meta charset="utf-8"><title>Demo</title><style>body { background-color: red; }</style></head><body><div class="message">Ready</div></body></html>"#,
		)
		.unwrap();

        assert!(result.stylesheets[0].contains("body"));
        assert!(result.stylesheets[0].contains("background-color: red"));
        assert_eq!(
            result.node.to_html(),
            r#"<html style="display: block;"><body style="margin-top: 8px; margin-right: 8px; margin-bottom: 8px; margin-left: 8px; display: block;"><div class="message" style="display: block;">Ready</div></body></html>"#
        );
    }

    #[test]
    fn loads_linked_stylesheets_before_inline_styles() {
        let temp = tempdir();
        write(temp.path().join("linked.css"), ".external { color: red; }");

        let options = FromHtmlOptions::new()
            .with_base_path(temp.path())
            .load_linked_stylesheets(true);
        let result = from_document_with_options(
            r#"<!doctype html><html><head><link rel="stylesheet" href="linked.css"><style>.inline { color: blue; }</style></head><body><div>Ready</div></body></html>"#,
            &options,
        )
        .unwrap();

        assert_eq!(result.stylesheets.len(), 2);
        assert!(result.stylesheets[0].contains(".external"));
        assert!(result.stylesheets[1].contains(".inline"));
    }

    #[test]
    fn ignores_non_stylesheet_links() {
        let temp = tempdir();
        write(temp.path().join("linked.css"), ".external { color: red; }");

        let options = FromHtmlOptions::new()
            .with_base_path(temp.path())
            .load_linked_stylesheets(true);
        let result = from_document_with_options(
			r#"<!doctype html><html><head><link rel="preload" href="linked.css"></head><body><div>Ready</div></body></html>"#,
			&options,
		)
		.unwrap();

        assert!(result.stylesheets.is_empty());
    }

    #[test]
    fn stylesheet_links_require_href() {
        let options = FromHtmlOptions::new().load_linked_stylesheets(true);
        let error = from_document_with_options(
			r#"<!doctype html><html><head><link rel="stylesheet"></head><body><div>Ready</div></body></html>"#,
			&options,
		)
		.unwrap_err();

        assert_eq!(
            error,
            HtmlError::MissingLinkedStylesheetHref {
                tag_name: "link".to_string(),
            }
        );
    }

    #[test]
    fn resolves_local_image_sources_to_data_uris() {
        let temp = tempdir();
        write(temp.path().join("relative.png"), tiny_png_bytes());

        let options = FromHtmlOptions::new()
            .with_base_path(temp.path())
            .resolve_local_assets(true);
        let result = from_document_with_options(
			r#"<!doctype html><html><body><img src="relative.png" width="16" height="16"></body></html>"#,
			&options,
		)
		.unwrap();

        assert!(
            result
                .node
                .to_html()
                .contains(r#"src="data:image/png;base64,"#)
        );
    }

    #[test]
    fn resolves_local_image_sources_to_absolute_paths() {
        let temp = tempdir();
        let image_path = temp.path().join("relative.png");
        write(&image_path, tiny_png_bytes());
        let expected = image_path
            .canonicalize()
            .expect("canonicalize image")
            .to_string_lossy()
            .replace('\\', "/");

        let options = FromHtmlOptions::new()
            .with_base_path(temp.path())
            .local_asset_mode(LocalAssetMode::AbsolutePath);
        let result = from_document_with_options(
            r#"<!doctype html><html><body><img src="relative.png" width="16" height="16"></body></html>"#,
            &options,
        )
        .unwrap();

        assert!(
            result
                .node
                .to_html()
                .contains(&format!(r#"src="{}""#, expected))
        );
    }

    #[test]
    fn rewrites_local_css_urls_using_their_source_directory() {
        let temp = tempdir();
        fs::create_dir_all(temp.path().join("styles")).unwrap();
        write(
            temp.path().join("styles/linked.css"),
            ".panel { background-image: url('../relative.png'); }",
        );
        write(temp.path().join("relative.png"), tiny_png_bytes());

        let options = FromHtmlOptions::new()
            .with_base_path(temp.path())
            .load_linked_stylesheets(true)
            .resolve_local_assets(true);
        let result = from_document_with_options(
			r#"<!doctype html><html><head><link rel="stylesheet" href="styles/linked.css"></head><body><div class="panel"></div></body></html>"#,
			&options,
		)
		.unwrap();

        assert!(result.stylesheets[0].contains("url(\"data:image/png;base64,"));
    }

    #[test]
    fn rewrites_local_css_urls_to_absolute_paths_using_their_source_directory() {
        let temp = tempdir();
        let styles_dir = temp.path().join("styles");
        let image_path = temp.path().join("relative.png");
        fs::create_dir_all(&styles_dir).unwrap();
        write(
            styles_dir.join("linked.css"),
            ".panel { background-image: url('../relative.png'); }",
        );
        write(&image_path, tiny_png_bytes());
        let expected = image_path
            .canonicalize()
            .expect("canonicalize image")
            .to_string_lossy()
            .replace('\\', "/");

        let options = FromHtmlOptions::new()
            .with_base_path(temp.path())
            .load_linked_stylesheets(true)
            .local_asset_mode(LocalAssetMode::AbsolutePath);
        let result = from_document_with_options(
            r#"<!doctype html><html><head><link rel="stylesheet" href="styles/linked.css"></head><body><div class="panel"></div></body></html>"#,
            &options,
        )
        .unwrap();

        assert!(result.stylesheets[0].contains(&expected));
    }

    #[test]
    fn rewrites_local_css_urls_inside_inline_style_blocks() {
        let temp = tempdir();
        write(temp.path().join("relative.png"), tiny_png_bytes());

        let result = from_document_with_options(
			r#"<!doctype html><html><head><style>.panel { background-image: url('./relative.png'); }</style></head><body><div class="panel"></div></body></html>"#,
			&FromHtmlOptions::new()
				.with_base_path(temp.path())
				.resolve_local_assets(true),
		)
		.unwrap();

        assert!(result.stylesheets[0].contains("url(\"data:image/png;base64,"));
    }

    #[test]
    fn rewrites_local_css_urls_inside_inline_style_attributes() {
        let temp = tempdir();
        write(temp.path().join("relative.png"), tiny_png_bytes());

        let result = from_document_with_options(
			r#"<!doctype html><html><body><div style="background-image: url('./relative.png')"></div></body></html>"#,
			&FromHtmlOptions::new()
				.with_base_path(temp.path())
				.resolve_local_assets(true),
		)
		.unwrap();

        let rendered = result.node.to_html();
        assert!(rendered.contains("background-image"));
        assert!(rendered.contains("data:image/png;base64,"));
    }

    #[test]
    fn rewrites_local_css_urls_inside_inline_style_attributes_to_absolute_paths() {
        let temp = tempdir();
        let image_path = temp.path().join("relative.png");
        write(&image_path, tiny_png_bytes());
        let expected = image_path
            .canonicalize()
            .expect("canonicalize image")
            .to_string_lossy()
            .replace('\\', "/");

        let result = from_document_with_options(
            r#"<!doctype html><html><body><div style="background-image: url('./relative.png')"></div></body></html>"#,
            &FromHtmlOptions::new()
                .with_base_path(temp.path())
                .local_asset_mode(LocalAssetMode::AbsolutePath),
        )
        .unwrap();

        let rendered = result.node.to_html();
        assert!(rendered.contains("background-image"));
        assert!(rendered.contains(&expected));
    }

    #[test]
    fn compiles_inline_scss() {
        let result =
            from_html(r#"<style lang="scss">$color: red; .foo { color: $color; }</style>"#)
                .unwrap();
        assert!(result.stylesheets[0].contains("color: red"));
    }

    #[test]
    fn stylesheet_compiler_is_invoked_for_inline_style() {
        let options = FromHtmlOptions::new().stylesheet_compiler(Box::new(|source, _base| {
            Ok(source.replace("$color", "blue"))
        }));
        let result =
            from_document_with_options(r#"<style>.foo { color: $color; }</style>"#, &options)
                .unwrap();

        assert!(result.stylesheets[0].contains("color: blue"));
    }

    #[test]
    fn stylesheet_compiler_is_invoked_for_linked_stylesheet() {
        let temp = tempdir();
        write(temp.path().join("linked.css"), ".foo { color: $color; }");

        let options = FromHtmlOptions::new()
            .with_base_path(temp.path())
            .load_linked_stylesheets(true)
            .stylesheet_compiler(Box::new(|source, _base| {
                Ok(source.replace("$color", "green"))
            }));
        let result = from_document_with_options(
            r#"<html><head><link rel="stylesheet" href="linked.css"></head></html>"#,
            &options,
        )
        .unwrap();

        assert!(result.stylesheets[0].contains("color: green"));
    }

    #[test]
    fn stylesheet_compiler_error_is_propagated() {
        let options = FromHtmlOptions::new().stylesheet_compiler(Box::new(|_source, _base| {
            Err(HtmlError::StylesheetCompilationFailed {
                reason: "bad scss".to_string(),
            })
        }));
        let result = from_document_with_options(r#"<style>.foo { color: red; }</style>"#, &options);
        assert!(matches!(
            result,
            Err(HtmlError::StylesheetCompilationFailed { .. })
        ));
    }

    #[test]
    fn inline_style_relative_asset_resolution_requires_base_path() {
        let error = from_document_with_options(
			r#"<!doctype html><html><body><div style="background-image: url('./relative.png')"></div></body></html>"#,
			&FromHtmlOptions::new().resolve_local_assets(true),
		)
		.unwrap_err();

        assert_eq!(
            error,
            HtmlError::MissingBasePathForRelativeReference {
                reference: "./relative.png".to_string(),
                kind: "css url".to_string(),
            }
        );
    }

    #[test]
    fn reports_missing_linked_stylesheet_files() {
        let temp = tempdir();
        let error = from_document_with_options(
			r#"<!doctype html><html><head><link rel="stylesheet" href="missing.css"></head><body><div>Ready</div></body></html>"#,
			&FromHtmlOptions::new()
				.with_base_path(temp.path())
				.load_linked_stylesheets(true),
		)
		.unwrap_err();

        assert!(matches!(
            error,
            HtmlError::AssetReadFailed { ref path, .. } if path.ends_with("missing.css")
        ));
    }

    #[test]
    fn whitespace_normalization_is_opt_in() {
        let markup = "<div>\n  Ready\n</div>";

        let default_result = from_document(markup).unwrap();
        assert!(default_result.node.to_html().contains("\n  Ready\n"));

        let normalized =
            from_document_with_options(markup, &FromHtmlOptions::new().normalize_whitespace(true))
                .unwrap();

        assert_eq!(
            normalized.node.to_html(),
            "<html style=\"display: block;\"><body style=\"margin-top: 8px; margin-right: 8px; margin-bottom: 8px; margin-left: 8px; display: block;\"><div style=\"display: block;\">Ready</div></body></html>"
        );
    }

    #[test]
    fn whitespace_normalization_preserves_inline_spacing_around_elements() {
        let result = from_document_with_options(
            "<div>Hello <span>world</span>\n</div>",
            &FromHtmlOptions::new().normalize_whitespace(true),
        )
        .unwrap();

        assert_eq!(
            result.node.to_html(),
            "<html style=\"display: block;\"><body style=\"margin-top: 8px; margin-right: 8px; margin-bottom: 8px; margin-left: 8px; display: block;\"><div style=\"display: block;\"><span>Hello </span><span>world</span></div></body></html>"
        );
    }

    #[test]
    fn inline_svg_markup_adds_missing_xmlns() {
        let result = from_html(
            r##"<svg viewBox="0 0 10 10"><rect width="10" height="10" fill="#fff"></rect></svg>"##,
        )
        .unwrap();

        assert!(
            result
                .node
                .to_html()
                .contains("xmlns=&quot;http://www.w3.org/2000/svg&quot;")
        );
    }

    #[test]
    fn extracts_emojis_into_inline_images_when_enabled() {
        let result = from_document_with_options(
            "<div>Hello 😀!</div>",
            &FromHtmlOptions::new().emoji_type(EmojiType::Twemoji),
        )
        .unwrap();

        assert_eq!(
            result.resource_urls().collect::<Vec<_>>(),
            vec!["https://cdn.jsdelivr.net/gh/jdecked/twemoji@17.0.2/assets/svg/1f600.svg"]
        );
        assert!(result.node.to_html().contains("Hello "));
        assert!(result.node.to_html().contains("vertical-align: -0.1em;"));
    }

    #[test]
    fn leaves_emoji_text_unchanged_when_emoji_extraction_is_disabled() {
        let result = from_document("<div>Hello 😀!</div>").unwrap();

        assert!(result.resource_urls().collect::<Vec<_>>().is_empty());
        assert!(result.node.to_html().contains("😀"));
    }

    #[test]
    fn relative_asset_resolution_requires_base_path() {
        let error = from_document_with_options(
            r#"<!doctype html><html><body><img src="relative.png"></body></html>"#,
            &FromHtmlOptions::new().resolve_local_assets(true),
        )
        .unwrap_err();

        assert_eq!(
            error,
            HtmlError::MissingBasePathForRelativeReference {
                reference: "relative.png".to_string(),
                kind: "image".to_string(),
            }
        );
    }

    fn tempdir() -> TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    fn write(path: impl AsRef<Path>, content: impl AsRef<[u8]>) {
        fs::write(path, content).expect("write fixture")
    }

    fn tiny_png_bytes() -> Vec<u8> {
        vec![
            137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1,
            8, 6, 0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 248, 255,
            255, 63, 0, 5, 254, 2, 254, 167, 53, 129, 132, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96,
            130,
        ]
    }
}
