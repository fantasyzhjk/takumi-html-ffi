mod convert;
mod emoji;
mod metadata;
mod options;
mod parser;
mod resource;
mod style;

use std::{fmt, path::PathBuf};

use takumi::{
    StyleSheetParseError,
    layout::{node::Node, style::StyleSheet},
};

pub use emoji::EmojiType;
pub use options::{FromHtmlOptions, LocalAssetMode};

pub type Result<T> = std::result::Result<T, HtmlError>;

#[derive(Debug, Clone)]
pub struct FromHtmlResult {
    pub node: Node,
    pub stylesheets: Vec<String>,
}

impl FromHtmlResult {
    /// Returns the stylesheet sources extracted from the HTML input.
    pub fn stylesheet_sources(&self) -> &[String] {
        &self.stylesheets
    }

    /// Returns stylesheet sources after prepending extra sources.
    ///
    /// Additional sources are placed before styles extracted from the HTML so the
    /// markup-local `<style>` contents keep later source-order precedence.
    pub fn stylesheet_sources_with<I, S>(&self, additional: I) -> Vec<String>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        merge_stylesheet_sources(
            additional.into_iter().map(Into::into),
            self.stylesheets.iter().cloned(),
        )
    }

    /// Consumes the result and returns the extracted stylesheet sources.
    pub fn into_stylesheet_sources(self) -> Vec<String> {
        self.stylesheets
    }

    /// Consumes the result and returns additional + extracted stylesheet sources.
    pub fn into_stylesheet_sources_with<I, S>(self, additional: I) -> Vec<String>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        merge_stylesheet_sources(additional.into_iter().map(Into::into), self.stylesheets)
    }

    /// Strictly parses the extracted stylesheet sources.
    pub fn try_stylesheet(&self) -> std::result::Result<StyleSheet, StyleSheetParseError> {
        StyleSheet::parse_list(self.stylesheet_sources())
    }

    /// Strictly parses additional + extracted stylesheet sources.
    pub fn try_stylesheet_with<I, S>(
        &self,
        additional: I,
    ) -> std::result::Result<StyleSheet, StyleSheetParseError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        StyleSheet::parse_list(self.stylesheet_sources_with(additional))
    }

    /// Parses the extracted stylesheet sources using Takumi's lossy stylesheet parser.
    pub fn stylesheet(&self) -> StyleSheet {
        StyleSheet::parse_list_loosy(self.stylesheet_sources())
    }

    /// Parses additional + extracted stylesheet sources using Takumi's lossy parser.
    pub fn stylesheet_with<I, S>(&self, additional: I) -> StyleSheet
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        StyleSheet::parse_owned_list_loosy(self.stylesheet_sources_with(additional))
    }

    /// Consumes the result and parses the extracted stylesheet sources lossily.
    pub fn into_stylesheet(self) -> StyleSheet {
        StyleSheet::parse_owned_list_loosy(self.into_stylesheet_sources())
    }

    /// Consumes the result and parses additional + extracted stylesheet sources lossily.
    pub fn into_stylesheet_with<I, S>(self, additional: I) -> StyleSheet
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        StyleSheet::parse_owned_list_loosy(self.into_stylesheet_sources_with(additional))
    }
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
        }
    }
}

impl std::error::Error for HtmlError {}

fn merge_stylesheet_sources<I, J>(additional: I, extracted: J) -> Vec<String>
where
    I: IntoIterator<Item = String>,
    J: IntoIterator<Item = String>,
{
    let mut stylesheets = additional.into_iter().collect::<Vec<_>>();
    stylesheets.extend(extracted);
    stylesheets
}

/// Parse HTML using fragment semantics and convert it into a Takumi node tree.
///
/// This is the lightweight public entry point for the MVP skeleton. Full
/// document-specific behavior can be layered on top later without changing the
/// conversion boundary exposed here.
pub fn from_html(markup: &str) -> Result<FromHtmlResult> {
    from_html_with_options(markup, &FromHtmlOptions::default())
}

/// Parse HTML using fragment semantics with conversion options.
pub fn from_html_with_options(markup: &str, options: &FromHtmlOptions) -> Result<FromHtmlResult> {
    from_fragment_with_options(markup, options)
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

    use takumi::layout::node::Node;
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

        assert_eq!(result.node.to_html(), "<div><span>Nested</span></div>");
    }

    #[test]
    fn converts_images() {
        let result =
            from_html(r#"<img src="https://example.com/example.png" width="64" height="32">"#)
                .unwrap();

        assert_eq!(
            result.node.resource_urls().collect::<Vec<_>>(),
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
    fn extracts_stylesheets() {
        let result = from_html("<style>body { color: red; }</style><div>Ready</div>").unwrap();

        assert_eq!(result.stylesheets, vec!["body { color: red; }".to_string()]);
        assert_eq!(result.node.to_html(), "<div>Ready</div>");
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
            "<div><span>foo</span><span>bar</span></div>"
        );
    }

    #[test]
    fn converts_full_documents() {
        let result = from_document(
			r#"<!doctype html><html><head><meta charset="utf-8"><title>Demo</title><style>body { background-color: red; }</style></head><body><div class="message">Ready</div></body></html>"#,
		)
		.unwrap();

        assert_eq!(
            result.stylesheets,
            vec!["body { background-color: red; }".to_string()]
        );
        assert_eq!(
            result.node.to_html(),
            r#"<html><body><div class="message">Ready</div></body></html>"#
        );
    }

    #[test]
    fn stylesheet_sources_with_preserve_external_then_extracted_order() {
        let result = from_html("<style>.inline { color: blue; }</style><div>Ready</div>").unwrap();

        assert_eq!(
            result.stylesheet_sources_with([".external { color: red; }"]),
            vec![
                ".external { color: red; }".to_string(),
                ".inline { color: blue; }".to_string(),
            ]
        );
    }

    #[test]
    fn try_stylesheet_reports_invalid_css() {
        let result = FromHtmlResult {
            node: Node::container([]),
            stylesheets: vec![".broken { color: ".to_string()],
        };

        assert!(result.try_stylesheet().is_err());
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

        assert_eq!(
            result.stylesheets,
            vec![
                ".external { color: red; }".to_string(),
                ".inline { color: blue; }".to_string(),
            ]
        );
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
        write(temp.path().join("relative.png"), &tiny_png_bytes());

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
        write(&image_path, &tiny_png_bytes());
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

        assert!(result.node.to_html().contains(&format!(r#"src="{}""#, expected)));
    }

    #[test]
    fn rewrites_local_css_urls_using_their_source_directory() {
        let temp = tempdir();
        fs::create_dir_all(temp.path().join("styles")).unwrap();
        write(
            temp.path().join("styles/linked.css"),
            ".panel { background-image: url('../relative.png'); }",
        );
        write(temp.path().join("relative.png"), &tiny_png_bytes());

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
        write(&image_path, &tiny_png_bytes());
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
        write(temp.path().join("relative.png"), &tiny_png_bytes());

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
        write(temp.path().join("relative.png"), &tiny_png_bytes());

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
        write(&image_path, &tiny_png_bytes());
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
            "<html><body><div>Ready</div></body></html>"
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
            "<html><body><div><span>Hello </span><span>world</span></div></body></html>"
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
            result.node.resource_urls().collect::<Vec<_>>(),
            vec!["https://cdn.jsdelivr.net/gh/jdecked/twemoji@17.0.2/assets/svg/1f600.svg"]
        );
        assert!(result.node.to_html().contains("Hello "));
        assert!(result.node.to_html().contains("vertical-align: -0.1em;"));
    }

    #[test]
    fn leaves_emoji_text_unchanged_when_emoji_extraction_is_disabled() {
        let result = from_document("<div>Hello 😀!</div>").unwrap();

        assert!(result.node.resource_urls().collect::<Vec<_>>().is_empty());
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
