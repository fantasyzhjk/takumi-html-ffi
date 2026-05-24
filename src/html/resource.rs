use std::{
    fs,
    path::{Path, PathBuf},
};

use base64::{Engine as _, prelude::BASE64_STANDARD};

use super::{FromHtmlOptions, HtmlError, Result, options::LocalAssetMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LocalReferenceKind {
    Stylesheet,
    Image,
    CssUrl,
}

impl LocalReferenceKind {
    fn label(self) -> &'static str {
        match self {
            Self::Stylesheet => "stylesheet",
            Self::Image => "image",
            Self::CssUrl => "css url",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct AssetResolver<'a> {
    options: &'a FromHtmlOptions,
}

impl<'a> AssetResolver<'a> {
    pub(crate) fn new(options: &'a FromHtmlOptions) -> Self {
        Self { options }
    }

    pub(crate) fn resolve_image_source(&self, src: &str) -> Result<String> {
        match self.options.effective_local_asset_mode() {
            LocalAssetMode::Preserve => Ok(src.to_owned()),
            LocalAssetMode::InlineDataUri | LocalAssetMode::AbsolutePath => self.resolve_local_reference(
            src,
            self.options.base_path.as_deref(),
            LocalReferenceKind::Image,
        ),
        }
    }

    pub(crate) fn resolve_inline_stylesheet(&self, stylesheet: &str) -> Result<String> {
        if matches!(self.options.effective_local_asset_mode(), LocalAssetMode::Preserve) {
            return Ok(stylesheet.to_owned());
        }

        self.rewrite_css_urls(stylesheet, self.options.base_path.as_deref())
    }

    pub(crate) fn resolve_inline_style_attribute(&self, style_text: &str) -> Result<String> {
        if matches!(self.options.effective_local_asset_mode(), LocalAssetMode::Preserve) {
            return Ok(style_text.to_owned());
        }

        self.rewrite_css_urls(style_text, self.options.base_path.as_deref())
    }

    pub(crate) fn load_linked_stylesheet(&self, href: &str) -> Result<Option<String>> {
        if !self.options.load_linked_stylesheets {
            return Ok(None);
        }

        let Some(path) = self.resolve_local_path(
            href,
            self.options.base_path.as_deref(),
            LocalReferenceKind::Stylesheet,
        )?
        else {
            return Ok(None);
        };

        let stylesheet = fs::read_to_string(&path).map_err(|error| HtmlError::AssetReadFailed {
            path: path.clone(),
            reason: error.to_string(),
        })?;

        if matches!(self.options.effective_local_asset_mode(), LocalAssetMode::Preserve) {
            return Ok(Some(stylesheet));
        }

        Ok(Some(self.rewrite_css_urls(&stylesheet, path.parent())?))
    }

    pub(crate) fn rewrite_css_urls(
        &self,
        stylesheet: &str,
        base_dir: Option<&Path>,
    ) -> Result<String> {
        let mut rewritten = String::with_capacity(stylesheet.len());
        let mut cursor = 0;

        while cursor < stylesheet.len() {
            if let Some((token, end_index)) = parse_url_token(stylesheet, cursor) {
                let resolved = self.resolve_local_reference(&token, base_dir, LocalReferenceKind::CssUrl)?;
                rewritten.push_str("url(\"");
                rewritten.push_str(&resolved);
                rewritten.push_str("\")");
                cursor = end_index;
                continue;
            }

            let ch = stylesheet[cursor..]
                .chars()
                .next()
                .expect("cursor always points to a valid character boundary");
            rewritten.push(ch);
            cursor += ch.len_utf8();
        }

        Ok(rewritten)
    }

    fn resolve_local_reference(
        &self,
        reference: &str,
        base_dir: Option<&Path>,
        kind: LocalReferenceKind,
    ) -> Result<String> {
        let Some(path) = self.resolve_local_path(reference, base_dir, kind)? else {
            return Ok(reference.to_owned());
        };

        match self.options.effective_local_asset_mode() {
            LocalAssetMode::Preserve => Ok(reference.to_owned()),
            LocalAssetMode::InlineDataUri => self.inline_path_as_data_uri(reference, &path),
            LocalAssetMode::AbsolutePath => self.absolute_path_reference(reference, &path),
        }
    }

    fn inline_path_as_data_uri(&self, reference: &str, path: &Path) -> Result<String> {
        let bytes = fs::read(path).map_err(|error| HtmlError::AssetReadFailed {
            path: path.to_path_buf(),
            reason: error.to_string(),
        })?;
        let mime = mime_guess::from_path(path)
            .first_or_octet_stream()
            .essence_str()
            .to_owned();
        let encoded = BASE64_STANDARD.encode(bytes);
        let (_, suffix) = split_reference_suffix(reference.trim());
        let mut data_uri = format!("data:{mime};base64,{encoded}");
        if !suffix.is_empty() {
            data_uri.push_str(suffix);
        }
        Ok(data_uri)
    }

    fn absolute_path_reference(&self, reference: &str, path: &Path) -> Result<String> {
        let normalized = path.canonicalize().map_err(|error| HtmlError::AssetReadFailed {
            path: path.to_path_buf(),
            reason: error.to_string(),
        })?;
        let (_, suffix) = split_reference_suffix(reference.trim());
        let mut data_uri = normalized.to_string_lossy().replace('\\', "/");
        if !suffix.is_empty() {
            data_uri.push_str(suffix);
        }
        Ok(data_uri)
    }

    fn resolve_local_path(
        &self,
        reference: &str,
        base_dir: Option<&Path>,
        kind: LocalReferenceKind,
    ) -> Result<Option<PathBuf>> {
        let reference = reference.trim();
        if reference.is_empty() || !is_local_relative_reference(reference) {
            return Ok(None);
        }

        let Some(base_dir) = base_dir else {
            return Err(HtmlError::MissingBasePathForRelativeReference {
                reference: reference.to_owned(),
                kind: kind.label().to_owned(),
            });
        };

        let (path_part, _) = split_reference_suffix(reference);
        Ok(Some(base_dir.join(path_part)))
    }
}

fn parse_url_token(source: &str, start: usize) -> Option<(String, usize)> {
    let bytes = source.as_bytes();
    if bytes.get(start..start + 3)?.eq_ignore_ascii_case(b"url") {
        let mut cursor = start + 3;
        while matches!(bytes.get(cursor), Some(b' ' | b'\n' | b'\r' | b'\t' | 0x0C)) {
            cursor += 1;
        }

        if !matches!(bytes.get(cursor), Some(b'(')) {
            return None;
        }
        cursor += 1;

        while matches!(bytes.get(cursor), Some(b' ' | b'\n' | b'\r' | b'\t' | 0x0C)) {
            cursor += 1;
        }

        let quote = match bytes.get(cursor) {
            Some(b'\'') => Some('\''),
            Some(b'"') => Some('"'),
            _ => None,
        };

        if quote.is_some() {
            cursor += 1;
        }

        let value_start = cursor;
        while let Some(ch) = source[cursor..].chars().next() {
            if let Some(quote) = quote {
                if ch == quote {
                    let value = source[value_start..cursor].to_owned();
                    cursor += ch.len_utf8();
                    while matches!(bytes.get(cursor), Some(b' ' | b'\n' | b'\r' | b'\t' | 0x0C)) {
                        cursor += 1;
                    }
                    if matches!(bytes.get(cursor), Some(b')')) {
                        return Some((value, cursor + 1));
                    }
                    return None;
                }
            } else if ch == ')' {
                let value = source[value_start..cursor].trim().to_owned();
                return Some((value, cursor + 1));
            }
            cursor += ch.len_utf8();
        }
    }

    None
}

fn split_reference_suffix(reference: &str) -> (&str, &str) {
    let split_at = reference.find(['?', '#']).unwrap_or(reference.len());
    reference.split_at(split_at)
}

fn is_local_relative_reference(reference: &str) -> bool {
    let reference = reference.trim();
    if reference.is_empty()
        || reference.starts_with('/')
        || reference.starts_with('#')
        || reference.starts_with("//")
        || reference.trim_start().starts_with("<svg")
    {
        return false;
    }

    let lower = reference.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("data:") {
        return false;
    }

    !has_scheme(reference)
}

fn has_scheme(reference: &str) -> bool {
    for ch in reference.chars() {
        match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '+' | '-' | '.' => continue,
            ':' => return true,
            '/' | '?' | '#' => return false,
            _ => return false,
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::{is_local_relative_reference, parse_url_token, split_reference_suffix};

    #[test]
    fn parses_quoted_css_url_tokens() {
        assert_eq!(
            parse_url_token(r#"background:url("./image.png")"#, 11),
            Some(("./image.png".to_string(), 29))
        );
    }

    #[test]
    fn parses_unquoted_css_url_tokens() {
        assert_eq!(
            parse_url_token("background:url(./image.png)", 11),
            Some(("./image.png".to_string(), 27))
        );
    }

    #[test]
    fn splits_reference_suffixes() {
        assert_eq!(
            split_reference_suffix("./font.woff2?#iefix"),
            ("./font.woff2", "?#iefix")
        );
    }

    #[test]
    fn detects_local_relative_references() {
        assert!(is_local_relative_reference("./image.png"));
        assert!(is_local_relative_reference("../image.png"));
        assert!(!is_local_relative_reference(
            "https://example.com/image.png"
        ));
        assert!(!is_local_relative_reference("data:image/png;base64,abc"));
        assert!(!is_local_relative_reference("/image.png"));
    }
}
