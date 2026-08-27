use std::{fmt, path::{Path, PathBuf}};

use super::emoji::EmojiType;

/// Callback signature for compiling stylesheet source (e.g. SCSS → CSS).
/// Receives the raw source text and an optional base path for resolving imports.
pub type StylesheetCompiler = Box<dyn Fn(&str, Option<&Path>) -> super::Result<String>>;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LocalAssetMode {
    #[default]
    Preserve,
    InlineDataUri,
    AbsolutePath,
}

#[derive(Default)]
pub struct FromHtmlOptions {
    pub base_path: Option<PathBuf>,
    pub load_linked_stylesheets: bool,
    pub local_asset_mode: LocalAssetMode,
    pub normalize_whitespace: bool,
    pub emoji_type: Option<EmojiType>,
    pub stylesheet_compiler: Option<StylesheetCompiler>,
}

impl fmt::Debug for FromHtmlOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FromHtmlOptions")
            .field("base_path", &self.base_path)
            .field("load_linked_stylesheets", &self.load_linked_stylesheets)
            .field("local_asset_mode", &self.local_asset_mode)
            .field("normalize_whitespace", &self.normalize_whitespace)
            .field("emoji_type", &self.emoji_type)
            .field("stylesheet_compiler", &self.stylesheet_compiler.as_ref().map(|_| ".."))
            .finish()
    }
}

impl FromHtmlOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_base_path<P>(mut self, base_path: P) -> Self
    where
        P: Into<PathBuf>,
    {
        self.base_path = Some(base_path.into());
        self
    }

    pub fn load_linked_stylesheets(mut self, enabled: bool) -> Self {
        self.load_linked_stylesheets = enabled;
        self
    }

    pub fn resolve_local_assets(mut self, enabled: bool) -> Self {
        self.local_asset_mode = if enabled {
            LocalAssetMode::InlineDataUri
        } else {
            LocalAssetMode::Preserve
        };
        self
    }

    pub fn local_asset_mode(mut self, mode: LocalAssetMode) -> Self {
        self.local_asset_mode = mode;
        self
    }

    pub fn normalize_whitespace(mut self, enabled: bool) -> Self {
        self.normalize_whitespace = enabled;
        self
    }

    pub fn emoji_type(mut self, emoji_type: EmojiType) -> Self {
        self.emoji_type = Some(emoji_type);
        self
    }

    pub fn stylesheet_compiler(mut self, compiler: StylesheetCompiler) -> Self {
        self.stylesheet_compiler = Some(compiler);
        self
    }
}
